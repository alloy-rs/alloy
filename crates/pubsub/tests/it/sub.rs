use alloy_primitives::B256;
use alloy_pubsub::{RawSubscription, SubscriptionItem};
use futures::StreamExt;
use serde_json::value::RawValue;
use tokio::sync::broadcast::{self, error::RecvError};

fn subscription(capacity: usize, items: &[&str]) -> RawSubscription {
    let (tx, rx) = broadcast::channel(capacity);
    for item in items {
        tx.send(RawValue::from_string(item.to_string()).unwrap()).unwrap();
    }
    RawSubscription { rx, local_id: B256::ZERO }
}

#[tokio::test]
async fn typed_recv_skips_other_types() {
    let mut sub = subscription(8, &[r#""a""#, "{}", "7", "null", "8"]).into_typed::<u64>();

    assert_eq!(sub.recv().await.unwrap(), 7);
    assert_eq!(sub.try_recv().unwrap(), 8);
    assert!(sub.try_recv().is_err());
}

#[tokio::test]
async fn streams_skip_lag() {
    let lagged = || subscription(2, &["0", r#""x""#, "2"]).into_typed::<u64>();

    assert!(matches!(lagged().recv().await, Err(RecvError::Lagged(1))));

    let items: Vec<_> = lagged().into_stream().collect().await;
    assert_eq!(items, [2]);

    let results: Vec<_> = lagged().into_result_stream().map(Result::ok).collect().await;
    assert_eq!(results, [None, Some(2)]);

    let any: Vec<_> = lagged()
        .into_any_stream()
        .map(|item| match item {
            SubscriptionItem::Item(n) => Ok(n),
            SubscriptionItem::Other(raw) => Err(raw.get().to_owned()),
        })
        .collect()
        .await;
    assert_eq!(any, [Err(r#""x""#.to_owned()), Ok(2)]);
}
