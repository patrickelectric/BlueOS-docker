//! A State is published only when it changes, and a client that joins later gets the last value by querying.

use std::time::Duration;

use blueos_idl::{blueos_example_msgs::PumpState, cdr};
use blueos_service::{State, testing::sessions};

const KEY: &str = "blueos/v1/test/state/pump";

#[tokio::test(flavor = "multi_thread")]
async fn a_late_joiner_gets_the_last_value_and_unchanged_values_are_not_sent() {
    let (service, client) = sessions(17501).await.unwrap();
    let state = State::<PumpState>::declare(&service, KEY.into()).await.unwrap();
    state
        .put_if_changed(&PumpState {
            level: 3,
            max_level: 10,
        })
        .await
        .unwrap();

    // Subscribe first, then query: an update between the two is not lost.
    let subscriber = client.declare_subscriber(KEY).await.unwrap();
    let mut reply = None;
    // The queryable was declared after the client connected, so the first queries may not know it yet.
    for _ in 0..100 {
        if let Ok(answer) = client.get(KEY).await.unwrap().recv_async().await {
            reply = Some(answer.into_result().unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let reply = reply.expect("the State never answered");
    assert_eq!(
        reply.encoding().to_string(),
        "application/cdr;blueos_example_msgs/msg/PumpState"
    );
    let joined: PumpState = cdr::decode(&reply.payload().to_bytes()).unwrap();
    assert_eq!(joined.level, 3);

    state
        .put_if_changed(&PumpState {
            level: 3,
            max_level: 10,
        })
        .await
        .unwrap();
    state
        .put_if_changed(&PumpState {
            level: 4,
            max_level: 10,
        })
        .await
        .unwrap();
    let sample = tokio::time::timeout(Duration::from_secs(5), subscriber.recv_async())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cdr::decode::<PumpState>(&sample.payload().to_bytes()).unwrap().level, 4);
    let nothing_else = tokio::time::timeout(Duration::from_millis(300), subscriber.recv_async()).await;
    assert!(nothing_else.is_err(), "an unchanged value was published again");
}
