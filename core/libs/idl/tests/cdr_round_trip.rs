//! Any value of a message encodes and decodes back to itself.

use blueos_idl::{
    blueos_msgs::{CommandAck, JobList, JobStatus},
    builtin_interfaces::Time,
    cdr,
    foxglove_msgs::Log,
};
use proptest::prelude::*;

fn job_status() -> impl Strategy<Value = JobStatus> {
    (".*", ".*", any::<u8>(), ".*").prop_map(|(job_id, job_type, status, reason)| JobStatus {
        job_id,
        job_type,
        status,
        reason,
    })
}

proptest! {
    #[test]
    fn command_ack_round_trips(accepted in any::<bool>(), job_id in ".*", status in any::<u8>(), reason in ".*") {
        let ack = CommandAck { accepted, job_id, status, reason };
        prop_assert_eq!(cdr::decode::<CommandAck>(&cdr::encode(&ack).unwrap()).unwrap(), ack);
    }

    #[test]
    fn job_list_round_trips(jobs in prop::collection::vec(job_status(), 0..8)) {
        let list = JobList { jobs };
        prop_assert_eq!(cdr::decode::<JobList>(&cdr::encode(&list).unwrap()).unwrap(), list);
    }

    #[test]
    fn log_round_trips(sec in any::<i32>(), nanosec in any::<u32>(), level in any::<u8>(), message in ".*", line in any::<u32>()) {
        let log = Log { timestamp: Time { sec, nanosec }, level, message, name: "test".into(), file: "test.rs".into(), line };
        prop_assert_eq!(cdr::decode::<Log>(&cdr::encode(&log).unwrap()).unwrap(), log);
    }
}
