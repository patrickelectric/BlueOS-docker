//! The route table: one list per service of its public endpoints. It declares the Command queryables, fills
//! `ServiceInfo`, and is compared with the service's `api.lock`, so the three always agree.

use std::{future::Future, time::Duration};

use blueos_idl::{
    Message,
    blueos_msgs::{CommandAck, EndpointInfo},
    cdr::{self, CdrError},
};
use tokio::sync::{mpsc, oneshot};
use zenoh::query::Query;

use crate::{
    Error, Service,
    jobs::{self, JobId},
    keys, lock,
};

/// The Command endpoint that cancels a Job, served when a Job type of the table is cancellable.
pub const CANCEL_JOB: &str = "CancelJob";
const COMMAND_QUEUE: usize = 64;
const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

/// Turns the CDR Goal of a Job type into the service's own Request.
pub type Decode<Request> = fn(&[u8]) -> Result<Request, CdrError>;

pub struct Route<Request: 'static> {
    pub name: &'static str,
    /// The interface type, for example `blueos_example_msgs/action/SetLevel`.
    pub interface: &'static str,
    pub kind: RouteKind<Request>,
    schema: fn() -> String,
}

pub enum RouteKind<Request: 'static> {
    Job { decode: Decode<Request>, cancellable: bool },
    Query,
    State,
}

/// A Request for the service loop. The loop answers it through `reply`.
pub struct Command<Request> {
    pub job_id: JobId,
    pub job_type: &'static str,
    /// The Goal as it arrived, so the same id with another Goal is rejected.
    pub goal: Vec<u8>,
    pub cancellable: bool,
    pub action: Action<Request>,
    pub reply: Reply,
}

pub enum Action<Request> {
    Submit(Request),
    Cancel,
}

pub struct Reply(oneshot::Sender<CommandAck>);

impl<Request> Route<Request> {
    pub const fn job<Goal: Message, Output: Message, Feedback: Message>(
        name: &'static str,
        interface: &'static str,
        decode: Decode<Request>,
        cancellable: bool,
    ) -> Self {
        Self {
            name,
            interface,
            kind: RouteKind::Job { decode, cancellable },
            schema: action_schema::<Goal, Output, Feedback>,
        }
    }

    pub const fn query<QueryRequest: Message, QueryResponse: Message>(
        name: &'static str,
        interface: &'static str,
    ) -> Self {
        Self {
            name,
            interface,
            kind: RouteKind::Query,
            schema: service_schema::<QueryRequest, QueryResponse>,
        }
    }

    pub const fn state<M: Message>(name: &'static str) -> Self {
        Self {
            name,
            interface: M::SCHEMA_NAME,
            kind: RouteKind::State,
            schema: message_schema::<M>,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            RouteKind::Job { .. } => "job",
            RouteKind::Query => "query",
            RouteKind::State => "state",
        }
    }

    pub fn key(&self, service: &str) -> String {
        match self.kind {
            RouteKind::Job { .. } => keys::command(service, self.name),
            RouteKind::Query => keys::query(service, self.name),
            RouteKind::State => keys::state(service, self.name),
        }
    }
}

impl Reply {
    pub fn send(self, ack: CommandAck) {
        // The client may have gone away, the ack is then for nobody.
        let _ = self.0.send(ack);
    }
}

impl<Arguments> Service<Arguments> {
    /// Lists the table in `ServiceInfo` and serves its Job types. Their Commands arrive on the receiver.
    pub async fn routes<Request: Send + 'static>(
        &self,
        routes: &'static [Route<Request>],
    ) -> Result<mpsc::Receiver<Command<Request>>, Error> {
        let (sender, receiver) = mpsc::channel(COMMAND_QUEUE);
        for route in routes {
            self.add_endpoint(route.kind_name(), route.name, route.interface, (route.schema)());
            if let RouteKind::Job { decode, cancellable } = &route.kind {
                self.serve_command(route.name, Some(*decode), *cancellable, sender.clone())
                    .await?;
            }
        }
        if routes
            .iter()
            .any(|route| matches!(route.kind, RouteKind::Job { cancellable: true, .. }))
        {
            self.add_endpoint("job", CANCEL_JOB, "", String::new());
            self.serve_command(CANCEL_JOB, None, true, sender).await?;
        }
        Ok(receiver)
    }

    /// Serves a Query of the table outside the service loop, one request at a time and with a timeout. A request
    /// without a payload is the default one.
    pub async fn io_query<Request, Response, Handler, Output>(&self, name: &str, handler: Handler) -> Result<(), Error>
    where
        Request: Message,
        Response: Message,
        Handler: Fn(Request) -> Output + Send + 'static,
        Output: Future<Output = Result<Response, String>> + Send,
    {
        self.check_route("query", name)?;
        let key = keys::query(self.name, name);
        let queryable = self.session.declare_queryable(&key).await?;
        let shutdown = self.shutdown.clone();
        self.tasks.spawn(async move {
            loop {
                let query = tokio::select! {
                    query = queryable.recv_async() => match query {
                        Ok(query) => query,
                        Err(_) => break,
                    },
                    () = shutdown.cancelled() => break,
                };
                let request = match query.payload() {
                    None => Ok(Request::default()),
                    Some(payload) => cdr::decode::<Request>(&payload.to_bytes()),
                };
                let answer = match request {
                    Err(error) => Err(error.to_string()),
                    Ok(request) => tokio::time::timeout(QUERY_TIMEOUT, handler(request))
                        .await
                        .unwrap_or_else(|_| Err("the query timed out".to_owned())),
                };
                match answer {
                    Ok(response) => reply(&query, &key, &response).await,
                    Err(reason) => {
                        if let Err(error) = query.reply_err(reason).await {
                            tracing::warn!(%error, key, "Failed to reply");
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub(crate) fn add_endpoint(&self, kind: &str, name: &str, interface: &str, schema: String) {
        let key = match kind {
            "job" => keys::command(self.name, name),
            "query" => keys::query(self.name, name),
            _ => keys::state(self.name, name),
        };
        lock(&self.endpoints).push(EndpointInfo {
            kind: kind.to_owned(),
            name: name.to_owned(),
            key,
            interface_type: interface.to_owned(),
            schema,
        });
    }

    pub(crate) fn check_route(&self, kind: &str, name: &str) -> Result<(), Error> {
        let known = lock(&self.endpoints)
            .iter()
            .any(|endpoint| endpoint.kind == kind && endpoint.name == name);
        known
            .then_some(())
            .ok_or_else(|| Error::UnknownRoute(format!("{kind} {name}")))
    }

    async fn serve_command<Request: Send + 'static>(
        &self,
        job_type: &'static str,
        decode: Option<Decode<Request>>,
        cancellable: bool,
        sender: mpsc::Sender<Command<Request>>,
    ) -> Result<(), Error> {
        let key = keys::command(self.name, job_type);
        let queryable = self.session.declare_queryable(&key).await?;
        let shutdown = self.shutdown.clone();
        self.tasks.spawn(async move {
            loop {
                let query = tokio::select! {
                    query = queryable.recv_async() => match query {
                        Ok(query) => query,
                        Err(_) => break,
                    },
                    () = shutdown.cancelled() => break,
                };
                let ack = command_ack(&query, job_type, decode, cancellable, &sender).await;
                reply(&query, &key, &ack).await;
            }
        });
        Ok(())
    }
}

/// The Job id a client sends as the attachment of a Command, as text.
pub(crate) fn job_id(query: &Query) -> Option<JobId> {
    let attachment = query.attachment()?.to_bytes();
    JobId::parse(std::str::from_utf8(&attachment).ok()?)
}

pub(crate) async fn reply<M: Message>(query: &Query, key: &str, message: &M) {
    let result = match cdr::encode(message) {
        Ok(bytes) => query
            .reply(key, bytes)
            .encoding(cdr::encoding::<M>())
            .await
            .map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };
    if let Err(error) = result {
        tracing::warn!(%error, key, "Failed to reply");
    }
}

async fn command_ack<Request>(
    query: &Query,
    job_type: &'static str,
    decode: Option<Decode<Request>>,
    cancellable: bool,
    sender: &mpsc::Sender<Command<Request>>,
) -> CommandAck {
    let Some(job_id) = job_id(query) else {
        return jobs::rejected(None, "the attachment must be the Job id, a UUID");
    };
    let goal = query
        .payload()
        .map(|payload| payload.to_bytes().into_owned())
        .unwrap_or_default();
    let action = match decode {
        None => Action::Cancel,
        Some(decode) => match decode(&goal) {
            Ok(request) => Action::Submit(request),
            Err(error) => return jobs::rejected(Some(&job_id), &error.to_string()),
        },
    };
    let (reply, answer) = oneshot::channel();
    let command = Command {
        job_id: job_id.clone(),
        job_type,
        goal,
        cancellable,
        action,
        reply: Reply(reply),
    };
    if sender.send(command).await.is_err() {
        return jobs::rejected(Some(&job_id), "the service is stopping");
    }
    answer
        .await
        .unwrap_or_else(|_| jobs::rejected(Some(&job_id), "the service dropped the Command"))
}

pub(crate) fn action_schema<Goal: Message, Output: Message, Feedback: Message>() -> String {
    format!(
        "{}\n---\n{}\n---\n{}",
        Goal::schema(),
        Output::schema(),
        Feedback::schema()
    )
}

fn service_schema<Request: Message, Response: Message>() -> String {
    format!("{}\n---\n{}", Request::schema(), Response::schema())
}

fn message_schema<M: Message>() -> String {
    M::schema().to_owned()
}
