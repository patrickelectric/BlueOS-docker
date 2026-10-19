//! The entry point of a service: CLI, logging, the Zenoh session, shutdown on SIGTERM, and the exit code.

use std::{
    future::Future,
    path::PathBuf,
    process::ExitCode,
    sync::{Arc, Mutex},
    time::Duration,
};

use blueos_idl::blueos_msgs::EndpointInfo;
use clap::Parser;
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{Error, logging};

/// How long Tasks get to stop once the service loop returned.
pub const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, clap::Args)]
pub struct CommonArguments {
    /// More logs: -v for debug, -vv for trace. RUST_LOG overrides it.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Folder holding the settings folder of the service, $XDG_CONFIG_HOME or ~/.config by default.
    #[arg(long)]
    pub settings_path: Option<PathBuf>,
    /// The Zenoh router to connect to.
    #[arg(long, default_value = "tcp/127.0.0.1:7447")]
    pub zenoh_endpoint: String,
    /// A Zenoh configuration file, used instead of --zenoh-endpoint.
    #[arg(long, env = "ZENOH_CONFIG")]
    pub zenoh_config: Option<PathBuf>,
}

#[derive(clap::Parser)]
struct Cli<Arguments: clap::Args> {
    #[command(flatten)]
    common: CommonArguments,
    #[command(flatten)]
    service: Arguments,
}

/// A running service: its arguments, its Zenoh session, and the shutdown its loop and Tasks follow.
pub struct Service<Arguments> {
    pub arguments: Arguments,
    pub session: zenoh::Session,
    pub shutdown: CancellationToken,
    pub tasks: TaskTracker,
    pub(crate) name: &'static str,
    pub(crate) version: &'static str,
    pub(crate) settings_folder: PathBuf,
    pub(crate) endpoints: Arc<Mutex<Vec<EndpointInfo>>>,
}

impl CommonArguments {
    pub fn zenoh_config(&self) -> Result<zenoh::Config, Error> {
        if let Some(path) = &self.zenoh_config {
            return Ok(zenoh::Config::from_file(path)?);
        }
        let mut config = zenoh::Config::default();
        config.insert_json5("mode", "\"client\"")?;
        config.insert_json5("connect/endpoints", &serde_json::to_string(&[&self.zenoh_endpoint])?)?;
        Ok(config)
    }

    /// The folder of the settings files, where the Python settings manager puts them too.
    pub fn settings_folder(&self, service: &str) -> PathBuf {
        let base = self
            .settings_path
            .clone()
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
            .or_else(|| std::env::home_dir().map(|home| home.join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));
        base.join(service.to_lowercase())
    }
}

impl<Arguments> Service<Arguments> {
    pub fn name(&self) -> &'static str {
        self.name
    }
}

/// Parses the command line, starts logging and the runtime, opens the session and runs `main` until it returns.
pub fn run<Arguments, Main, Output>(name: &'static str, version: &'static str, main: Main) -> ExitCode
where
    Arguments: clap::Args,
    Main: FnOnce(Service<Arguments>) -> Output,
    Output: Future<Output = Result<(), Error>>,
{
    let cli = Cli::<Arguments>::parse();
    logging::init(cli.common.verbose);
    let result = tokio::runtime::Runtime::new().map_err(Error::from).and_then(|runtime| {
        runtime.block_on(async move {
            let session = zenoh::open(cli.common.zenoh_config()?).await?;
            start(name, version, &cli.common, cli.service, session, main).await
        })
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "Service stopped");
            ExitCode::FAILURE
        }
    }
}

/// `run` on an open session, without the command line and the runtime.
pub(crate) async fn start<Arguments, Main, Output>(
    name: &'static str,
    version: &'static str,
    common: &CommonArguments,
    arguments: Arguments,
    session: zenoh::Session,
    main: Main,
) -> Result<(), Error>
where
    Main: FnOnce(Service<Arguments>) -> Output,
    Output: Future<Output = Result<(), Error>>,
{
    let shutdown = CancellationToken::new();
    let tasks = TaskTracker::new();
    logging::publish(&session, name, &tasks, shutdown.clone());
    tasks.spawn(cancel_on_signal(shutdown.clone()));
    let service = Service {
        arguments,
        session,
        shutdown: shutdown.clone(),
        tasks: tasks.clone(),
        name,
        version,
        settings_folder: common.settings_folder(name),
        endpoints: Arc::default(),
    };
    let result = main(service).await;
    stop(&tasks, &shutdown, SHUTDOWN_BUDGET).await;
    result
}

async fn cancel_on_signal(shutdown: CancellationToken) {
    let Ok(mut terminate) = signal(SignalKind::terminate()) else {
        return;
    };
    tokio::select! {
        _ = terminate.recv() => shutdown.cancel(),
        _ = tokio::signal::ctrl_c() => shutdown.cancel(),
        () = shutdown.cancelled() => {}
    }
}

/// Stops the Tasks within `budget`, so a stuck one never keeps the process from exiting. False when one was left.
pub(crate) async fn stop(tasks: &TaskTracker, shutdown: &CancellationToken, budget: Duration) -> bool {
    shutdown.cancel();
    tasks.close();
    let stopped = tokio::time::timeout(budget, tasks.wait()).await.is_ok();
    if !stopped {
        tracing::warn!(remaining = tasks.len(), "Tasks still running after the shutdown budget");
    }
    stopped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn stop_waits_for_tasks_that_follow_the_shutdown() {
        let (tasks, shutdown) = (TaskTracker::new(), CancellationToken::new());
        let token = shutdown.clone();
        tasks.spawn(async move { token.cancelled().await });
        assert!(stop(&tasks, &shutdown, SHUTDOWN_BUDGET).await);
    }

    #[tokio::test(start_paused = true)]
    async fn stop_gives_up_on_a_stuck_task_after_the_budget() {
        let (tasks, shutdown) = (TaskTracker::new(), CancellationToken::new());
        tasks.spawn(std::future::pending::<()>());
        let started = tokio::time::Instant::now();
        assert!(!stop(&tasks, &shutdown, SHUTDOWN_BUDGET).await);
        assert_eq!(started.elapsed(), SHUTDOWN_BUDGET);
    }
}
