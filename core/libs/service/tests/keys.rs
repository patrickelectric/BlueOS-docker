//! The keys match the shared vectors, which the frontend and the Python services check too.

use blueos_service::keys;
use serde_json::Value;

type ServiceKey = fn(&str) -> String;
type NamedKey = fn(&str, &str) -> String;

#[test]
fn every_key_matches_its_vector() {
    let vectors: Value = serde_json::from_str(include_str!("vectors/keys.json")).unwrap();
    assert_eq!(vectors["constants"]["key_prefix"], keys::PREFIX);
    let by_service: [(&str, ServiceKey); 5] = [
        ("service_liveliness_key", keys::liveliness),
        ("jobs_key", keys::jobs),
        ("settings_key", keys::settings),
        ("log_key", keys::log),
        ("info_query_key", |service| keys::query(service, "info")),
    ];
    let by_name: [(&str, NamedKey); 7] = [
        ("command_key", keys::command),
        ("state_key", keys::state),
        ("event_key", keys::event),
        ("query_key", keys::query),
        ("job_feedback_key", keys::job_feedback),
        ("job_result_key", keys::job_result),
        ("job_history_key", keys::job_history),
    ];
    for (section, key) in by_service {
        for vector in vectors[section].as_array().unwrap() {
            assert_eq!(
                key(vector["service"].as_str().unwrap()),
                vector["expected"],
                "{section}"
            );
        }
    }
    for (section, key) in by_name {
        for vector in vectors[section].as_array().unwrap() {
            let service = vector["service"].as_str().unwrap();
            assert_eq!(
                key(service, vector["name"].as_str().unwrap()),
                vector["expected"],
                "{section}"
            );
        }
    }
}
