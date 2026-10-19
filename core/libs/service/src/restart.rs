//! Restarts a Task that fails or panics, with an exponential backoff, until the service shuts down.

use std::{future::Future, time::Duration};

use crate::{Error, Service};

pub const INITIAL_BACKOFF: Duration = Duration::from_millis(100);
pub const MAX_BACKOFF: Duration = Duration::from_secs(30);

impl<Arguments> Service<Arguments> {
    /// Runs `task` until it returns `Ok`, restarting it after an error or a panic. A panic is logged by the
    /// panic hook, so it reaches the `log` key.
    pub fn supervise<Task, Output>(&self, name: &'static str, mut task: Task)
    where
        Task: FnMut() -> Output + Send + 'static,
        Output: Future<Output = Result<(), Error>> + Send + 'static,
    {
        let shutdown = self.shutdown.clone();
        self.tasks.spawn(async move {
            let mut backoff = INITIAL_BACKOFF;
            loop {
                let mut running = tokio::spawn(task());
                let outcome = tokio::select! {
                    outcome = &mut running => outcome,
                    () = shutdown.cancelled() => {
                        running.abort();
                        return;
                    }
                };
                match outcome {
                    Ok(Ok(())) => return,
                    Ok(Err(error)) => tracing::warn!(task = name, %error, "Task failed, restarting"),
                    Err(error) => tracing::warn!(task = name, %error, "Task panicked, restarting"),
                }
                tokio::select! {
                    () = tokio::time::sleep(backoff) => {}
                    () = shutdown.cancelled() => return,
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        });
    }
}
