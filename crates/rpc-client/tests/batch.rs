//! Batch request resolution against out-of-order and malformed replies.
#![allow(missing_docs)]

use alloy_json_rpc::{
    ErrorPayload, Id, RequestPacket, Response, ResponsePacket, ResponsePayload, SerializedRequest,
};
use alloy_primitives::U64;
use alloy_rpc_client::{RpcClient, Waiter};
use alloy_transport::{
    mock::Asserter, RpcError, TransportError, TransportErrorKind, TransportFut, TransportResult,
};
use serde_json::value::RawValue;

fn client_with<F>(reply: F) -> RpcClient
where
    F: Fn(RequestPacket) -> TransportResult<ResponsePacket> + Clone + Send + Sync + 'static,
{
    let service = tower::service_fn(move |req: RequestPacket| {
        let res = reply(req);
        Box::pin(async move { res }) as TransportFut<'static>
    });
    RpcClient::new(service, true)
}

fn echo(req: &SerializedRequest) -> Response {
    let result = serde_json::to_string(req.method()).unwrap();
    Response {
        id: req.id().clone(),
        payload: ResponsePayload::Success(RawValue::from_string(result).unwrap()),
    }
}

fn failure(req: &SerializedRequest, code: i64) -> Response {
    Response {
        id: req.id().clone(),
        payload: ResponsePayload::Failure(ErrorPayload {
            code,
            message: "boom".into(),
            data: None,
        }),
    }
}

fn batch_of(req: RequestPacket) -> Vec<SerializedRequest> {
    match req {
        RequestPacket::Batch(reqs) => reqs,
        RequestPacket::Single(_) => panic!("expected a batch request"),
    }
}

#[derive(Debug)]
enum Expect {
    Ok(&'static str),
    Missing(u64),
    ErrorResp(i64),
}

#[track_caller]
fn check(res: TransportResult<String>, expected: &Expect, case: &str) {
    match (res, expected) {
        (Ok(value), Expect::Ok(expected)) => assert_eq!(value, *expected, "{case}"),
        (
            Err(RpcError::Transport(TransportErrorKind::MissingBatchResponse(id))),
            Expect::Missing(expected),
        ) => {
            assert_eq!(id, Id::Number(*expected), "{case}")
        }
        (Err(RpcError::ErrorResp(err)), Expect::ErrorResp(code)) => {
            assert_eq!(err.code, *code, "{case}")
        }
        (res, expected) => panic!("{case}: expected {expected:?}, got {res:?}"),
    }
}

#[tokio::test]
async fn batch_resolves_waiters_by_id() {
    type Reply = fn(Vec<SerializedRequest>) -> ResponsePacket;
    let cases: [(&str, Reply, [Expect; 3]); 5] = [
        (
            "reversed",
            |reqs| ResponsePacket::Batch(reqs.iter().rev().map(echo).collect()),
            [Expect::Ok("a"), Expect::Ok("b"), Expect::Ok("c")],
        ),
        (
            "unknown id",
            |reqs| {
                let mut unknown = echo(&reqs[0]);
                unknown.id = Id::Number(99);
                let mut responses: Vec<_> = reqs.iter().map(echo).collect();
                responses.insert(1, unknown);
                ResponsePacket::Batch(responses)
            },
            [Expect::Ok("a"), Expect::Ok("b"), Expect::Ok("c")],
        ),
        (
            "missing id",
            |reqs| ResponsePacket::Batch(vec![echo(&reqs[2]), echo(&reqs[0])]),
            [Expect::Ok("a"), Expect::Missing(1), Expect::Ok("c")],
        ),
        (
            "single reply",
            |reqs| ResponsePacket::Single(echo(&reqs[1])),
            [Expect::Missing(0), Expect::Ok("b"), Expect::Missing(2)],
        ),
        (
            "error payload",
            |reqs| {
                ResponsePacket::Batch(vec![
                    echo(&reqs[0]),
                    failure(&reqs[1], -32000),
                    echo(&reqs[2]),
                ])
            },
            [Expect::Ok("a"), Expect::ErrorResp(-32000), Expect::Ok("c")],
        ),
    ];

    for (case, reply, expected) in cases {
        let client = client_with(move |req| Ok(reply(batch_of(req))));
        let mut batch = client.new_batch();
        let waiters: Vec<Waiter<String>> = ["a", "b", "c"]
            .into_iter()
            .map(|method| batch.add_call(method, &()).unwrap())
            .collect();
        batch.await.unwrap();

        for (waiter, expected) in waiters.into_iter().zip(&expected) {
            check(waiter.await, expected, case);
        }
    }
}

#[tokio::test]
async fn batch_transport_error_fails_batch_and_waiters() {
    let client = client_with(|_| Err(TransportErrorKind::custom_str("connection reset")));
    let mut batch = client.new_batch();
    let a: Waiter<String> = batch.add_call("a", &()).unwrap();
    let b: Waiter<String> = batch.add_call("b", &()).unwrap();

    let err = batch.await.unwrap_err();
    assert!(err.to_string().contains("connection reset"), "{err}");

    for waiter in [a, b] {
        let err = waiter.await.unwrap_err();
        assert!(err.is_transport_error(), "{err}");
    }
}

#[tokio::test]
async fn empty_batch_completes() {
    let asserter = Asserter::new();
    RpcClient::mocked(asserter.clone()).new_batch().await.unwrap();
    assert!(asserter.read_q().is_empty());
}

#[tokio::test]
async fn waiter_maps_and_deserializes_response() {
    let asserter = Asserter::new();
    let client = RpcClient::mocked(asserter.clone());
    asserter.push_success(&U64::from(7));
    asserter.push_success(&"not a number");

    let mut batch = client.new_batch();
    let mapped = batch
        .add_call::<_, U64>("eth_blockNumber", &())
        .unwrap()
        .map_resp(|n: U64| n.to::<u64>() * 2);
    let invalid: Waiter<U64> = batch.add_call("eth_blockNumber", &()).unwrap();
    batch.await.unwrap();

    assert_eq!(mapped.await.unwrap(), 14);
    let err: TransportError = invalid.await.unwrap_err();
    assert!(err.is_deser_error(), "{err}");
}

#[tokio::test]
async fn single_call_rejects_batch_reply() {
    let client = client_with(|req| {
        let RequestPacket::Single(req) = req else { panic!("expected a single request") };
        Ok(ResponsePacket::Batch(vec![echo(&req)]))
    });

    let err = client.request_noparams::<String>("a").await.unwrap_err();
    assert!(err.to_string().contains("received batch response from single request"), "{err}");
}
