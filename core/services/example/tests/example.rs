//! The example service as a client sees it: a SetLevel Job reports feedback and ends with its result, a level
//! above the maximum is refused, a cancel stops the pump, and a startup failure is printed with a failure exit.
#![allow(clippy::unwrap_used, reason = "test helpers")]

use std::{path::Path, time::Duration};

use blueos_example::{Arguments, NAME, routes::ROUTES, serve};
use blueos_idl::{
    blueos_example_msgs::{LevelResponse, PumpState, SetLevelGoal, SetLevelResult},
    blueos_msgs::{CommandAck, JobResult, JobStatus},
    cdr,
};
use blueos_service::testing::{assert_routes_match_api_lock, sessions, start, wait_for_subscriber};

const JOB: &str = "0b5e8f5c-6f0a-4c4e-9a52-2f1e7d3c9b10";
const OTHER_JOB: &str = "1c6f9a6d-7a1b-4d5f-8b63-3a2f8e4dac21";
const RESULTS: &str = "blueos/v1/example/jobs/SetLevel/result";

type Results = zenoh::pubsub::Subscriber<zenoh::handlers::FifoChannelHandler<zenoh::sample::Sample>>;

/// Starts the example and returns a client with a subscriber on the SetLevel results the service already knows.
async fn started(port: u16, settings: &Path) -> (zenoh::Session, Results) {
    let (service, client) = sessions(port).await.unwrap();
    let results = client.declare_subscriber(RESULTS).await.unwrap();
    wait_for_subscriber(&service, RESULTS).await.unwrap();
    tokio::spawn(start(NAME, settings.to_owned(), Arguments {}, service, serve));
    for _ in 0..100 {
        let alive = client.liveliness().get("blueos/v1/services/example").await.unwrap();
        if alive.recv_async().await.is_ok() {
            return (client, results);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the example never became alive");
}

async fn command(client: &zenoh::Session, name: &str, job_id: &str, goal: Option<u8>) -> CommandAck {
    let mut get = client
        .get(format!("blueos/v1/example/command/{name}"))
        .attachment(job_id);
    if let Some(level) = goal {
        get = get.payload(cdr::encode(&SetLevelGoal { level }).unwrap());
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

#[test]
fn the_route_table_matches_the_api_lock() {
    assert_routes_match_api_lock(NAME, ROUTES, &Path::new(env!("CARGO_MANIFEST_DIR")).join("api.lock"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_set_level_job_moves_the_pump_and_ends_with_its_result() {
    let settings = tempfile::tempdir().unwrap();
    let (client, results) = started(17601, settings.path()).await;

    let refused = command(&client, "SetLevel", OTHER_JOB, Some(200)).await;
    assert_eq!(
        (refused.accepted, refused.reason.as_str()),
        (false, "the level is above the maximum")
    );
    let ack = command(&client, "SetLevel", JOB, Some(2)).await;
    assert_eq!((ack.accepted, ack.status), (true, CommandAck::STATUS_EXECUTING));

    let sample = tokio::time::timeout(Duration::from_secs(5), results.recv_async())
        .await
        .unwrap()
        .unwrap();
    let result: JobResult = cdr::decode(&sample.payload().to_bytes()).unwrap();
    assert_eq!(result.job.status, JobStatus::STATUS_SUCCEEDED);
    assert_eq!(cdr::decode::<SetLevelResult>(&result.result).unwrap().level, 2);
    let pump: PumpState = query(&client, "blueos/v1/example/state/pump").await;
    assert_eq!(pump.level, 2);
    let level: LevelResponse = query(&client, "blueos/v1/example/query/Level").await;
    assert_eq!((level.level, level.max_level), (2, 100));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancel_stops_the_pump() {
    let settings = tempfile::tempdir().unwrap();
    let (client, results) = started(17602, settings.path()).await;
    assert!(command(&client, "SetLevel", JOB, Some(50)).await.accepted);
    let cancelled = command(&client, "CancelJob", JOB, None).await;
    assert_eq!(
        (cancelled.accepted, cancelled.status),
        (true, CommandAck::STATUS_CANCELED)
    );
    let sample = tokio::time::timeout(Duration::from_secs(5), results.recv_async())
        .await
        .unwrap()
        .unwrap();
    let result: JobResult = cdr::decode(&sample.payload().to_bytes()).unwrap();
    assert_eq!(result.job.status, JobStatus::STATUS_CANCELED);
}

#[test]
fn a_startup_failure_is_printed_and_exits_with_a_failure() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_example"))
        .args(["--zenoh-config", "/nonexistent/zenoh.json5"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Service stopped"), "{stderr}");
}
