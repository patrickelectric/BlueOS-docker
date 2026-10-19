//! Keyed timers: a re-armed key fires once at its new time, a cancelled key never fires.

use std::time::Duration;

use blueos_service::Timers;
use tokio::time::Instant;

#[tokio::test(start_paused = true)]
async fn a_rearmed_key_fires_once_at_its_new_time() {
    let mut timers = Timers::default();
    let started = Instant::now();
    timers.schedule("status", Duration::from_secs(1));
    timers.schedule("status", Duration::from_secs(3));
    timers.schedule("rescan", Duration::from_secs(2));
    assert_eq!(timers.next().await, "rescan");
    assert_eq!(timers.next().await, "status");
    assert_eq!(started.elapsed(), Duration::from_secs(3));
    assert!(!timers.is_armed(&"status"));
}

#[tokio::test(start_paused = true)]
async fn a_cancelled_key_never_fires() {
    let mut timers = Timers::default();
    timers.schedule("status", Duration::from_secs(1));
    timers.cancel(&"status");
    timers.cancel(&"never armed");
    let fired = tokio::time::timeout(Duration::from_secs(10), timers.next()).await;
    assert!(fired.is_err());
}
