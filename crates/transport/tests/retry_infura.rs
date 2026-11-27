//! Regression tests for Infura's transient block-fetch errors.
#![allow(missing_docs)]

use alloy_json_rpc::{ErrorPayload, Id, Request, RequestPacket};
use alloy_transport::{
    layers::RetryBackoffLayer,
    mock::{Asserter, MockTransport},
    TransportError,
};
use tower::{Layer, Service};

fn block_request() -> RequestPacket {
    RequestPacket::Single(
        Request::new("eth_getBlockByNumber", Id::Number(1), ("0x123", false)).serialize().unwrap(),
    )
}

fn precondition_failure() -> ErrorPayload {
    serde_json::from_str(r#"{"code":-32603,"message":"precondition failure"}"#).unwrap()
}

#[tokio::test]
async fn transient_precondition_failure_recovers() {
    let asserter = Asserter::new();
    asserter.push_failure(precondition_failure());
    let block = serde_json::json!({"number": "0x123"});
    asserter.push_success(&block);
    let mut service =
        RetryBackoffLayer::new(1, 0, 10_000).layer(MockTransport::new(asserter.clone()));

    let response = service.call(block_request()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&response.responses()[0]).unwrap(),
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": block}),
    );
    assert_eq!(asserter.requests().len(), 2);
}

#[tokio::test]
async fn persistent_precondition_failure_exhausts_retry_budget() {
    for retries in [0, 1, 3] {
        let asserter = Asserter::new();
        for _ in 0..=retries {
            asserter.push_failure(precondition_failure());
        }
        let mut service =
            RetryBackoffLayer::new(retries, 0, 10_000).layer(MockTransport::new(asserter.clone()));

        let err = service.call(block_request()).await.unwrap_err();
        assert_eq!(asserter.requests().len(), retries as usize + 1);
        assert!(err.to_string().starts_with("Max retries exceeded"));
        let TransportError::Transport(kind) = err else { panic!("expected transport error") };
        let source = kind.as_custom().unwrap().source().unwrap();
        let source = source.downcast_ref::<TransportError>().unwrap();
        let payload = source.as_error_resp().unwrap();
        assert_eq!(payload.code, -32603);
        assert_eq!(payload.message, "precondition failure");
    }
}
