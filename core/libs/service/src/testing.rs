//! Helpers for the tests of a service. Behind the `testing` feature, which only dev-dependencies enable.

use std::{
    fmt::Debug,
    future::Future,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{CommonArguments, Error, Service, routes::Route, run};

/// A service session listening on localhost and a client session connected to it, with scouting off.
pub async fn sessions(port: u16) -> Result<(zenoh::Session, zenoh::Session), Error> {
    let endpoint = format!("[\"tcp/127.0.0.1:{port}\"]");
    let mut listening = zenoh::Config::default();
    listening.insert_json5("scouting/multicast/enabled", "false")?;
    listening.insert_json5("listen/endpoints", &endpoint)?;
    let service = zenoh::open(listening).await?;
    let mut connecting = zenoh::Config::default();
    connecting.insert_json5("scouting/multicast/enabled", "false")?;
    connecting.insert_json5("connect/endpoints", &endpoint)?;
    let client = zenoh::open(connecting).await?;
    Ok((service, client))
}

/// Waits until `session` knows a subscriber on `key`, so what it publishes next reaches it.
pub async fn wait_for_subscriber(session: &zenoh::Session, key: &str) -> Result<(), Error> {
    let publisher = session.declare_publisher(key.to_owned()).await?;
    for _ in 0..100 {
        if publisher.matching_status().await?.matching() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(Error::Zenoh(format!("no subscriber on {key}").into()))
}

/// Runs `main` as `run` does, on `session`, with its settings in `settings_path`.
pub async fn start<Arguments, Main, Output>(
    name: &'static str,
    settings_path: PathBuf,
    arguments: Arguments,
    session: zenoh::Session,
    main: Main,
) -> Result<(), Error>
where
    Main: FnOnce(Service<Arguments>) -> Output,
    Output: Future<Output = Result<(), Error>>,
{
    let common = CommonArguments {
        verbose: 0,
        settings_path: Some(settings_path),
        zenoh_endpoint: String::new(),
        zenoh_config: None,
    };
    run::start(name, "test", &common, arguments, session, main).await
}

/// The route table is the API that `lock` records. Run the test with `BLUEOS_UPDATE_LOCK=1` to write it after a
/// reviewed change; CI fails when a line of master is gone from it.
pub fn assert_routes_match_api_lock<Request>(service: &str, routes: &[Route<Request>], lock: &Path) {
    let current: String = routes
        .iter()
        .map(|route| format!("{}: {} {}\n", route.key(service), route.kind_name(), route.interface))
        .collect();
    if std::env::var_os("BLUEOS_UPDATE_LOCK").is_some()
        && let Err(error) = std::fs::write(lock, &current)
    {
        panic!("failed to write {}: {error}", lock.display());
    }
    let committed = std::fs::read_to_string(lock).unwrap_or_default();
    assert_eq!(
        committed, current,
        "the route table changed, review it and run the test again with BLUEOS_UPDATE_LOCK=1"
    );
}

/// Applies a request that must be rejected and checks the state did not change.
pub fn assert_rejection_leaves_state<State, Outcome, Rejection>(
    state: &mut State,
    apply: impl FnOnce(&mut State) -> Result<Outcome, Rejection>,
) where
    State: Clone + PartialEq + Debug,
    Outcome: Debug,
{
    let before = state.clone();
    let outcome = apply(state);
    assert!(outcome.is_err(), "the request was not rejected: {:?}", outcome.ok());
    assert_eq!(*state, before, "a rejected request changed the state");
}
