//! Logs reach the `log` key: records from before the session, panics, and the last records before shutdown. Zenoh's
//! own records never do, or publishing them would loop. One test, since logging is global to the process.

use std::time::Duration;

use blueos_idl::{cdr, foxglove_msgs::Log};
use blueos_service::{
    logging,
    testing::{sessions, start, wait_for_subscriber},
};

#[tokio::test(flavor = "multi_thread")]
async fn records_panics_and_the_last_lines_reach_the_log_key() {
    logging::init(0);
    tracing::info!("before the session");
    let (service, client) = sessions(17521).await.unwrap();
    let records = client.declare_subscriber("blueos/v1/logger/log").await.unwrap();
    wait_for_subscriber(&service, "blueos/v1/logger/log").await.unwrap();
    let settings = tempfile::tempdir().unwrap();

    start("logger", settings.path().to_owned(), (), service, |_service| async {
        tracing::info!(target: "zenoh::transport", "inside zenoh");
        let _ = std::thread::spawn(|| panic!("a panicking thread")).join();
        tracing::warn!(attempt = 2, "just before shutdown");
        Ok(())
    })
    .await
    .unwrap();

    let mut messages = Vec::new();
    while let Ok(Ok(sample)) = tokio::time::timeout(Duration::from_secs(2), records.recv_async()).await {
        let record: Log = cdr::decode(&sample.payload().to_bytes()).unwrap();
        assert_eq!(record.name, "logger");
        messages.push((record.level, record.message));
    }
    let has = |level: u8, text: &str| {
        messages
            .iter()
            .any(|(found, message)| *found == level && message.contains(text))
    };
    assert!(has(Log::INFO, "before the session"), "{messages:?}");
    assert!(has(Log::ERROR, "a panicking thread"), "{messages:?}");
    assert!(has(Log::WARNING, "just before shutdown attempt=2"), "{messages:?}");
    assert!(!has(Log::INFO, "inside zenoh"), "{messages:?}");
}
