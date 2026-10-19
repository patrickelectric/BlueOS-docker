//! Job semantics: a retry returns the same Job, a reused id is rejected, only cancellable Jobs cancel, and the ack
//! carries the status after the step.
#![allow(clippy::unwrap_used, reason = "test helpers")]

use blueos_idl::blueos_msgs::{CommandAck, JobStatus};
use blueos_service::{JobId, Jobs, Submitted, jobs::HISTORY_LENGTH, testing::assert_rejection_leaves_state};

const FIRST: &str = "0b5e8f5c-6f0a-4c4e-9a52-2f1e7d3c9b10";
const SECOND: &str = "1c6f9a6d-7a1b-4d5f-8b63-3a2f8e4dac21";

fn id(text: &str) -> JobId {
    JobId::parse(text).unwrap()
}

#[test]
fn only_a_uuid_is_a_job_id() {
    assert!(JobId::parse("not a uuid").is_none());
    assert_eq!(id(FIRST).as_str(), FIRST);
}

#[test]
fn a_retry_returns_the_job_and_a_reused_id_is_rejected() {
    let mut jobs = Jobs::default();
    assert_eq!(jobs.submit(&id(FIRST), "SetLevel", b"goal", false), Ok(Submitted::New));
    jobs.start(&id(FIRST));
    let Ok(Submitted::Retry(ack)) = jobs.submit(&id(FIRST), "SetLevel", b"goal", false) else {
        panic!("a retry was not recognised");
    };
    assert!(ack.accepted);
    assert_eq!(ack.status, CommandAck::STATUS_EXECUTING);
    assert_rejection_leaves_state(&mut jobs, |jobs| {
        jobs.submit(&id(FIRST), "SetLevel", b"other goal", false)
    });
    let Err(rejection) = jobs.submit(&id(FIRST), "SetLevel", b"other goal", false) else {
        panic!("a reused id was accepted");
    };
    assert_eq!(rejection.reason, "id reused");
}

#[test]
fn a_job_the_service_refuses_leaves_no_trace() {
    let mut jobs = Jobs::default();
    let before = jobs.clone();
    jobs.submit(&id(FIRST), "SetLevel", b"goal", false).unwrap();
    let ack = jobs.reject(&id(FIRST), "the level is above the maximum");
    assert_eq!(
        (ack.accepted, ack.reason.as_str()),
        (false, "the level is above the maximum")
    );
    assert_eq!(jobs, before);
}

#[test]
fn only_an_active_cancellable_job_cancels() {
    let mut jobs = Jobs::default();
    jobs.submit(&id(FIRST), "SetLevel", b"", false).unwrap();
    jobs.submit(&id(SECOND), "Drain", b"", true).unwrap();
    assert!(!jobs.cancel(&id(FIRST)).accepted);
    let unknown = jobs.cancel(&id("2d7a0b7e-8b2c-4e60-9c74-4b3f9f5ebd32"));
    assert_eq!((unknown.accepted, unknown.reason.as_str()), (false, "unknown job"));
    let ack = jobs.cancel(&id(SECOND));
    assert_eq!((ack.accepted, ack.status), (true, CommandAck::STATUS_CANCELING));
    assert!(jobs.is_canceling(&id(SECOND)));
    jobs.finish(&id(SECOND), JobStatus::STATUS_CANCELED, "cancelled", &())
        .unwrap();
    assert_eq!(jobs.cancel(&id(SECOND)).reason, "the job already ended");
}

#[test]
fn a_finished_job_leaves_the_list_and_publishes_its_result() {
    let mut jobs = Jobs::default();
    jobs.submit(&id(FIRST), "SetLevel", b"", false).unwrap();
    jobs.feedback(&id(FIRST), &3u8).unwrap();
    assert_eq!(jobs.feedback_list("SetLevel").jobs.len(), 1);
    jobs.finish(&id(FIRST), JobStatus::STATUS_SUCCEEDED, "", &5u8).unwrap();
    assert!(jobs.list().jobs.is_empty());
    assert!(jobs.feedback_list("SetLevel").jobs.is_empty());
    assert_eq!(jobs.ack(&id(FIRST)).status, CommandAck::STATUS_SUCCEEDED);
    let results = jobs.take_results();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].job.status, JobStatus::STATUS_SUCCEEDED);
    assert!(jobs.take_results().is_empty());
    assert_eq!(jobs.history("SetLevel").jobs.len(), 1);
}

#[test]
fn the_history_keeps_the_last_jobs_only() {
    let mut jobs = Jobs::default();
    for index in 0..HISTORY_LENGTH + 3 {
        let id = id(&format!("00000000-0000-4000-8000-{index:012}"));
        jobs.submit(&id, "SetLevel", b"", false).unwrap();
        jobs.finish(&id, JobStatus::STATUS_SUCCEEDED, "", &()).unwrap();
    }
    assert_eq!(jobs.history("SetLevel").jobs.len(), HISTORY_LENGTH);
}
