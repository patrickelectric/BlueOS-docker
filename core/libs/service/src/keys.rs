//! Keys of the BlueOS API, `blueos/v1/<service>/...`.

pub const PREFIX: &str = "blueos/v1";

pub fn liveliness(service: &str) -> String {
    format!("{PREFIX}/services/{service}")
}

pub fn command(service: &str, name: &str) -> String {
    format!("{PREFIX}/{service}/command/{name}")
}

pub fn query(service: &str, name: &str) -> String {
    format!("{PREFIX}/{service}/query/{name}")
}

pub fn state(service: &str, name: &str) -> String {
    format!("{PREFIX}/{service}/state/{name}")
}

pub fn event(service: &str, name: &str) -> String {
    format!("{PREFIX}/{service}/event/{name}")
}

pub fn settings(service: &str) -> String {
    format!("{PREFIX}/{service}/settings")
}

pub fn log(service: &str) -> String {
    format!("{PREFIX}/{service}/log")
}

pub fn jobs(service: &str) -> String {
    format!("{PREFIX}/{service}/jobs")
}

pub fn job_feedback(service: &str, job_type: &str) -> String {
    format!("{PREFIX}/{service}/jobs/{job_type}/feedback")
}

pub fn job_result(service: &str, job_type: &str) -> String {
    format!("{PREFIX}/{service}/jobs/{job_type}/result")
}

pub fn job_history(service: &str, job_type: &str) -> String {
    format!("{PREFIX}/{service}/jobs/{job_type}/history")
}
