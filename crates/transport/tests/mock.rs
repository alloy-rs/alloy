//! Request recording of the [`MockTransport`].
#![allow(missing_docs)]

use alloy_json_rpc::{Id, Request, RequestPacket, RpcSend, SerializedRequest};
use alloy_transport::mock::{Asserter, MockTransport};
use serde_json::value::RawValue;
use tower::ServiceExt;

fn request(id: u64, method: &'static str, params: impl RpcSend) -> SerializedRequest {
    Request::new(method, Id::Number(id), params).serialize().unwrap()
}

fn summary(requests: &[SerializedRequest]) -> Vec<(Id, &str, Option<&str>)> {
    requests
        .iter()
        .map(|req| (req.id().clone(), req.method(), req.params().map(RawValue::get)))
        .collect()
}

#[tokio::test]
async fn records_single_and_batch_requests_in_order() {
    let asserter = Asserter::new();
    let transport = MockTransport::new(asserter.clone());
    asserter.push_success(&"0x1");
    asserter.push_success(&"0x2");
    asserter.push_success(&"0x3");

    transport.clone().oneshot(request(0, "eth_blockNumber", ()).into()).await.unwrap();
    let batch: RequestPacket = [
        request(1, "eth_getBalance", ("0x0000000000000000000000000000000000000001", "latest")),
        request(2, "eth_getCode", ["0x0000000000000000000000000000000000000002"]),
    ]
    .into_iter()
    .collect();
    transport.oneshot(batch).await.unwrap();

    let expected = [
        (Id::Number(0), "eth_blockNumber", None),
        (
            Id::Number(1),
            "eth_getBalance",
            Some(r#"["0x0000000000000000000000000000000000000001","latest"]"#),
        ),
        (Id::Number(2), "eth_getCode", Some(r#"["0x0000000000000000000000000000000000000002"]"#)),
    ];
    assert_eq!(summary(&asserter.requests()), expected);
    assert_eq!(summary(&asserter.requests()), expected);

    let first = asserter.pop_request().unwrap();
    assert_eq!(summary(&[first]), expected[..1]);
    assert_eq!(summary(&asserter.take_requests()), expected[1..]);
    assert!(asserter.requests().is_empty());
    assert!(asserter.pop_request().is_none());
}

#[tokio::test]
async fn records_requests_without_queued_response() {
    let asserter = Asserter::new();
    asserter.push_success(&"0x1");

    let batch: RequestPacket = (0..3).map(|id| request(id, "eth_chainId", ())).collect();
    let err = MockTransport::new(asserter.clone()).oneshot(batch).await.unwrap_err();
    assert!(err.to_string().contains("empty asserter response queue"), "{err}");

    let ids: Vec<_> = asserter.take_requests().iter().map(|req| req.id().clone()).collect();
    assert_eq!(ids, [Id::Number(0), Id::Number(1), Id::Number(2)]);
    assert!(asserter.read_q().is_empty());
}
