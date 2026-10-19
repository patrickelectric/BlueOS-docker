//! What every BlueOS Rust service repeats, and nothing more: the command line, logging, Python-compatible
//! settings, liveliness, `info`, States, the route table with its Commands and Queries, Jobs and keyed timers. A
//! service keeps its own state in one `tokio::select!` loop and uses `zenoh::Session` directly.

use std::sync::{Mutex, MutexGuard, PoisonError};

pub mod jobs;
pub mod keys;
pub mod logging;
pub mod routes;
pub mod settings;
pub mod state;
pub mod timers;

mod info;
mod job_keys;
mod restart;
mod run;

#[cfg(feature = "testing")]
pub mod testing;

pub use info::Ready;
pub use job_keys::JobKeys;
pub use jobs::{JobId, Jobs, Submitted};
pub use routes::{Action, Command, Reply, Route, RouteKind};
pub use run::{CommonArguments, SHUTDOWN_BUDGET, Service, run};
pub use state::State;
pub use timers::Timers;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Zenoh(#[from] zenoh::Error),
    #[error(transparent)]
    Cdr(#[from] blueos_idl::cdr::CdrError),
    #[error(transparent)]
    Settings(#[from] settings::SettingsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0} is not in the route table")]
    UnknownRoute(String),
}

/// A poisoned lock still holds valid data here: one panic must not take every Task that shares the lock with it.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
