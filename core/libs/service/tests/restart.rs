//! A supervised Task that fails or panics is started again with a growing backoff, until it returns `Ok`.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use blueos_service::{
    Error,
    testing::{sessions, start},
};

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_and_panicking_task_restarts_until_it_succeeds() {
    let (service, _client) = sessions(17531).await.unwrap();
    let settings = tempfile::tempdir().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let started = Instant::now();

    start("restart", settings.path().to_owned(), (), service, |service| {
        let attempts = Arc::clone(&attempts);
        async move {
            let (done, finished) = tokio::sync::oneshot::channel::<()>();
            let mut done = Some(done);
            service.supervise("flaky", move || {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                let done = if attempt == 2 { done.take() } else { None };
                async move {
                    match attempt {
                        0 => Err(Error::UnknownRoute("a failure".into())),
                        1 => panic!("a panic"),
                        _ => {
                            if let Some(done) = done {
                                let _ = done.send(());
                            }
                            Ok(())
                        }
                    }
                }
            });
            let _ = tokio::time::timeout(Duration::from_secs(5), finished).await;
            Ok(())
        }
    })
    .await
    .unwrap();

    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    // 100 ms after the failure, then 200 ms after the panic.
    assert!(started.elapsed() >= Duration::from_millis(300));
}
