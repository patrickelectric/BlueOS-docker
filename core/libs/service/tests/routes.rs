//! A service built on the route table, driven by a client on a real Zenoh session: discovery, Job semantics on
//! the wire, the Job keys, Queries, States and settings updates.
#![allow(clippy::unwrap_used, reason = "test helpers")]

use std::{path::Path, time::Duration};

use blueos_idl::{
    blueos_example_msgs::{LevelRequest, LevelResponse, PumpState, SetLevelFeedback, SetLevelGoal, SetLevelResult},
    blueos_msgs::{CommandAck, JobList, JobResult, JobStatus, ServiceInfo, SettingsEnvelope, UpdateSettingsGoal},
    cdr,
};
use blueos_service::{
    Action, Error, Jobs, Route, Service, Submitted,
    settings::Settings,
    testing::{assert_routes_match_api_lock, sessions, start},
};
use serde::{Deserialize, Serialize};

const JOB: &str = "0b5e8f5c-6f0a-4c4e-9a52-2f1e7d3c9b10";

enum Request {
    SetLevel(u8),
}

static ROUTES: &[Route<Request>] = &[
    Route::job::<SetLevelGoal, SetLevelResult, SetLevelFeedback>(
        "SetLevel",
        "blueos_example_msgs/action/SetLevel",
        |bytes| Ok(Request::SetLevel(cdr::decode::<SetLevelGoal>(bytes)?.level)),
        true,
    ),
    Route::query::<LevelRequest, LevelResponse>("Level", "blueos_example_msgs/srv/Level"),
    Route::state::<PumpState>("pump"),
];

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TestSettings {
    max_level: u8,
}

impl Settings for TestSettings {
    const VERSION: u64 = 1;
}

/// Accepts SetLevel as a lasting Job: level 0 ends at once, any other level runs until it is cancelled.
async fn test_service(service: Service<()>) -> Result<(), Error> {
    let mut commands = service.routes(ROUTES).await?;
    let job_keys = service.job_keys(ROUTES).await?;
    let pump = service.state::<PumpState>("pump").await?;
    service
        .io_query("Level", |_: LevelRequest| async {
            Ok(LevelResponse {
                level: 1,
                max_level: 10,
            })
        })
        .await?;
    let _settings = service.settings::<TestSettings>().await?;
    assert!(matches!(
        service.state::<PumpState>("other").await,
        Err(Error::UnknownRoute(_))
    ));
    pump.put_if_changed(&PumpState {
        level: 1,
        max_level: 10,
    })
    .await?;
    let _ready = service.ready().await?;

    let mut jobs = Jobs::default();
    loop {
        let command = tokio::select! {
            command = commands.recv() => command,
            () = service.shutdown.cancelled() => return Ok(()),
        };
        let Some(command) = command else {
            return Ok(());
        };
        let id = command.job_id.clone();
        let ack = match command.action {
            Action::Submit(Request::SetLevel(level)) => {
                match jobs.submit(&id, command.job_type, &command.goal, command.cancellable) {
                    Ok(Submitted::New) => {
                        jobs.start(&id);
                        if level == 0 {
                            jobs.finish(&id, JobStatus::STATUS_SUCCEEDED, "", &SetLevelResult { level })?;
                        }
                        jobs.ack(&id)
                    }
                    Ok(Submitted::Retry(ack)) | Err(ack) => ack,
                }
            }
            Action::Cancel => {
                let ack = jobs.cancel(&id);
                if ack.accepted {
                    jobs.finish(
                        &id,
                        JobStatus::STATUS_CANCELED,
                        "cancelled",
                        &SetLevelResult { level: 1 },
                    )?;
                }
                ack
            }
        };
        command.reply.send(ack);
        job_keys.sync(&mut jobs).await?;
    }
}

async fn started(port: u16, settings: &Path) -> (zenoh::Session, tokio::task::JoinHandle<Result<(), Error>>) {
    let (service, client) = sessions(port).await.unwrap();
    let handle = tokio::spawn(start("test", settings.to_owned(), (), service, test_service));
    for _ in 0..100 {
        let alive = client.liveliness().get("blueos/v1/services/test").await.unwrap();
        if alive.recv_async().await.is_ok() {
            return (client, handle);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the service never became alive");
}

async fn command(client: &zenoh::Session, name: &str, job_id: Option<&str>, goal: Option<Vec<u8>>) -> CommandAck {
    let mut get = client.get(format!("blueos/v1/test/command/{name}"));
    if let Some(job_id) = job_id {
        get = get.attachment(job_id);
    }
    if let Some(goal) = goal {
        get = get.payload(goal);
    }
    let reply = get.await.unwrap().recv_async().await.unwrap().into_result().unwrap();
    cdr::decode(&reply.payload().to_bytes()).unwrap()
}

async fn query<M: blueos_idl::Message>(client: &zenoh::Session, key: &str) -> M {
    let reply = client
        .get(key)
        .await
        .unwrap()
        .recv_async()
        .await
        .unwrap()
        .into_result()
        .unwrap();
    cdr::decode(&reply.payload().to_bytes()).unwrap()
}

fn set_level(level: u8) -> Option<Vec<u8>> {
    Some(cdr::encode(&SetLevelGoal { level }).unwrap())
}

#[test]
fn the_route_table_matches_the_api_lock() {
    let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/api.lock");
    assert_routes_match_api_lock("test", ROUTES, &lock);
}

#[tokio::test(flavor = "multi_thread")]
async fn once_alive_every_endpoint_answers_and_info_lists_them() {
    let settings = tempfile::tempdir().unwrap();
    let (client, handle) = started(17511, settings.path()).await;
    let info: ServiceInfo = query(&client, "blueos/v1/test/query/info").await;
    let names: Vec<_> = info.endpoints.iter().map(|endpoint| endpoint.name.as_str()).collect();
    assert_eq!(names, ["SetLevel", "Level", "pump", "CancelJob", "UpdateSettings"]);
    assert!(info.endpoints[0].schema.contains("uint8 level\n---\n"));
    let level: LevelResponse = query(&client, "blueos/v1/test/query/Level").await;
    assert_eq!(level.max_level, 10);
    let pump: PumpState = query(&client, "blueos/v1/test/state/pump").await;
    assert_eq!(pump.level, 1);
    handle.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn job_semantics_hold_on_the_wire() {
    let settings = tempfile::tempdir().unwrap();
    let (client, handle) = started(17512, settings.path()).await;

    let missing_id = command(&client, "SetLevel", None, set_level(3)).await;
    assert!(!missing_id.accepted);
    let ack = command(&client, "SetLevel", Some(JOB), set_level(3)).await;
    assert_eq!((ack.accepted, ack.status), (true, CommandAck::STATUS_EXECUTING));
    let retry = command(&client, "SetLevel", Some(JOB), set_level(3)).await;
    assert_eq!(retry, ack);
    let reused = command(&client, "SetLevel", Some(JOB), set_level(4)).await;
    assert_eq!((reused.accepted, reused.reason.as_str()), (false, "id reused"));
    let jobs: JobList = query(&client, "blueos/v1/test/jobs").await;
    assert_eq!(jobs.jobs.len(), 1);

    let results = client
        .declare_subscriber("blueos/v1/test/jobs/SetLevel/result")
        .await
        .unwrap();
    let cancelled = command(&client, "CancelJob", Some(JOB), None).await;
    assert!(cancelled.accepted);
    let sample = tokio::time::timeout(Duration::from_secs(5), results.recv_async())
        .await
        .unwrap()
        .unwrap();
    let result: JobResult = cdr::decode(&sample.payload().to_bytes()).unwrap();
    assert_eq!(result.job.status, JobStatus::STATUS_CANCELED);
    let history: JobList = query(&client, "blueos/v1/test/jobs/SetLevel/history").await;
    assert_eq!(history.jobs.len(), 1);

    let instant = command(
        &client,
        "SetLevel",
        Some("1c6f9a6d-7a1b-4d5f-8b63-3a2f8e4dac21"),
        set_level(0),
    )
    .await;
    assert_eq!(
        instant.status,
        CommandAck::STATUS_SUCCEEDED,
        "a Job that ends in its first step acks its end"
    );
    handle.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_settings_update_is_saved_and_published_and_a_wrong_version_is_rejected() {
    let settings = tempfile::tempdir().unwrap();
    let (client, handle) = started(17513, settings.path()).await;
    let update = |document: &str| {
        Some(
            cdr::encode(&UpdateSettingsGoal {
                envelope: SettingsEnvelope {
                    document_json: document.into(),
                    fields: Vec::new(),
                },
            })
            .unwrap(),
        )
    };

    let wrong = command(
        &client,
        "UpdateSettings",
        Some(JOB),
        update(r#"{"VERSION": 2, "max_level": 5}"#),
    )
    .await;
    assert!(!wrong.accepted);
    let ack = command(
        &client,
        "UpdateSettings",
        Some(JOB),
        update(r#"{"VERSION": 1, "max_level": 5}"#),
    )
    .await;
    assert_eq!((ack.accepted, ack.status), (true, CommandAck::STATUS_SUCCEEDED));
    let published: SettingsEnvelope = query(&client, "blueos/v1/test/settings").await;
    assert_eq!(published.document_json, r#"{"VERSION":1,"max_level":5}"#);
    let file = std::fs::read_to_string(settings.path().join("test/settings-1.json")).unwrap();
    assert!(file.contains("\"max_level\": 5"));
    handle.abort();
}
