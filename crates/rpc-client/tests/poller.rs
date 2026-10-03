//! Poller termination, pausing and channel behaviour.
#![allow(missing_docs)]

use alloy_json_rpc::ErrorPayload;
use alloy_primitives::U64;
use alloy_rpc_client::{PollerBuilder, RpcClient};
use alloy_transport::mock::Asserter;
use futures::{stream::FusedStream, FutureExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

const INTERVAL: Duration = Duration::from_secs(1);

fn mocked(numbers: impl IntoIterator<Item = u64>) -> (RpcClient, Asserter) {
    let asserter = Asserter::new();
    for n in numbers {
        asserter.push_success(&U64::from(n));
    }
    (RpcClient::mocked(asserter.clone()), asserter)
}

fn poller(client: &RpcClient) -> PollerBuilder<(), U64> {
    client.prepare_static_poller("eth_blockNumber", ()).with_poll_interval(INTERVAL)
}

fn numbers(numbers: &[u64]) -> Vec<U64> {
    numbers.iter().copied().map(U64::from).collect()
}

#[tokio::test(start_paused = true)]
async fn poller_stops_at_limit() {
    let (client, asserter) = mocked([1, 2, 3]);

    let items: Vec<U64> = poller(&client).with_limit(Some(2)).into_stream().collect().await;
    assert_eq!(items, numbers(&[1, 2]));
    assert_eq!(asserter.read_q().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn poller_terminal_errors() {
    let error =
        |code, message: &'static str| ErrorPayload { code, message: message.into(), data: None };
    let cases = [
        (vec![], error(-32000, "filter not found"), true),
        (vec![], error(-32000, "header not found"), false),
        (vec![-32000], error(-32000, "boom"), true),
        (vec![-32001], error(-32000, "filter not found"), false),
    ];

    for (codes, error, stops) in cases {
        let (client, asserter) = mocked([]);
        asserter.push_failure(error.clone());
        asserter.push_success(&U64::from(7));

        let items: Vec<U64> = poller(&client)
            .with_limit(Some(1))
            .with_terminal_error_codes(codes.clone())
            .into_stream()
            .collect()
            .await;
        let case = format!("codes {codes:?}, {error}");
        assert_eq!(items, if stops { vec![] } else { numbers(&[7]) }, "{case}");
        assert_eq!(asserter.read_q().len(), usize::from(stops), "{case}");
    }
}

#[tokio::test(start_paused = true)]
async fn poller_stops_when_client_is_dropped() {
    let (client, asserter) = mocked([1, 2]);

    let mut stream = poller(&client).into_stream();
    assert_eq!(stream.next().await, Some(U64::from(1)));
    drop(client);

    assert_eq!(stream.next().await, None);
    assert!(stream.is_terminated());
    assert_eq!(asserter.read_q().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn paused_poller_sends_no_requests() {
    let (client, asserter) = mocked([1, 2]);
    let mut stream = poller(&client).into_stream();

    stream.pause();
    tokio::time::advance(INTERVAL * 10).await;
    assert!(stream.next().now_or_never().is_none());
    assert_eq!(asserter.read_q().len(), 2);

    stream.unpause();
    assert_eq!(stream.next().await, Some(U64::from(1)));

    stream.pause();
    tokio::time::advance(INTERVAL * 10).await;
    assert!(stream.next().now_or_never().is_none());
    assert_eq!(asserter.read_q().len(), 1);

    stream.unpause();
    assert_eq!(stream.next().await, Some(U64::from(2)));
}

#[tokio::test(start_paused = true)]
async fn spawned_poller_exits_when_channel_is_dropped() {
    let (client, asserter) = mocked([1, 2, 3, 4]);

    let mut rx = poller(&client).spawn();
    assert_eq!(rx.recv().await.unwrap(), U64::from(1));
    drop(rx);

    tokio::time::sleep(INTERVAL * 10).await;
    assert_eq!(Arc::weak_count(client.inner()), 0);
    assert_eq!(asserter.read_q().len(), 2);
}

#[tokio::test(start_paused = true)]
async fn poll_channel_stream_skips_lagged_responses() {
    let (client, _asserter) = mocked([1, 2, 3]);

    let rx = poller(&client).with_channel_size(1).with_limit(Some(3)).spawn();
    let raw = rx.resubscribe();
    tokio::time::sleep(INTERVAL * 10).await;

    assert_eq!(rx.into_stream().collect::<Vec<_>>().await, numbers(&[3]));
    assert_eq!(
        raw.into_stream_raw().collect::<Vec<_>>().await,
        [Err(BroadcastStreamRecvError::Lagged(2)), Ok(U64::from(3))]
    );
}
