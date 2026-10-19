//! The Job keys every service with Jobs has, declared once from its route table: the `jobs` State, and per Job
//! type its `feedback` State, its `result` Event and its `history` Query.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use blueos_idl::{
    blueos_msgs::{JobFeedbackList, JobList, JobResult},
    cdr,
};
use zenoh::{Wait, query::Queryable};

use crate::{
    Error, Service, State,
    jobs::Jobs,
    keys, lock,
    routes::{Route, RouteKind},
};

pub struct JobKeys {
    service: &'static str,
    session: zenoh::Session,
    jobs: State<JobList>,
    feedback: Vec<(&'static str, State<JobFeedbackList>)>,
    history: Arc<Mutex<HashMap<&'static str, Vec<u8>>>>,
    _history_queryables: Vec<Queryable<()>>,
}

impl<Arguments> Service<Arguments> {
    pub async fn job_keys<Request>(&self, routes: &'static [Route<Request>]) -> Result<JobKeys, Error> {
        let history = Arc::new(Mutex::new(HashMap::new()));
        let mut feedback = Vec::new();
        let mut history_queryables = Vec::new();
        for route in routes
            .iter()
            .filter(|route| matches!(route.kind, RouteKind::Job { .. }))
        {
            let job_type = route.name;
            feedback.push((
                job_type,
                State::declare(&self.session, keys::job_feedback(self.name, job_type)).await?,
            ));
            let key = keys::job_history(self.name, job_type);
            let queryable = self
                .session
                .declare_queryable(&key)
                .callback({
                    let (history, key) = (Arc::clone(&history), key.clone());
                    move |query| {
                        let bytes = lock(&history).get(job_type).cloned();
                        let bytes = bytes.map_or_else(|| cdr::encode(&JobList::default()), Ok);
                        let result = bytes.map_err(|error| error.to_string()).and_then(|bytes| {
                            query
                                .reply(&key, bytes)
                                .encoding(cdr::encoding::<JobList>())
                                .wait()
                                .map_err(|error| error.to_string())
                        });
                        if let Err(error) = result {
                            tracing::warn!(%error, key, "Failed to reply");
                        }
                    }
                })
                .await?;
            history_queryables.push(queryable);
        }
        Ok(JobKeys {
            service: self.name,
            session: self.session.clone(),
            jobs: State::declare(&self.session, keys::jobs(self.name)).await?,
            feedback,
            history,
            _history_queryables: history_queryables,
        })
    }
}

impl JobKeys {
    /// Publishes what changed in the table and the results of the Jobs that ended. Call it after each loop step.
    pub async fn sync(&self, jobs: &mut Jobs) -> Result<(), Error> {
        self.jobs.put_if_changed(&jobs.list()).await?;
        for (job_type, state) in &self.feedback {
            state.put_if_changed(&jobs.feedback_list(job_type)).await?;
            let history = cdr::encode(&jobs.history(job_type))?;
            lock(&self.history).insert(job_type, history);
        }
        for result in jobs.take_results() {
            let key = keys::job_result(self.service, &result.job.job_type);
            self.session
                .put(key, cdr::encode(&result)?)
                .encoding(cdr::encoding::<JobResult>())
                .await?;
        }
        Ok(())
    }
}
