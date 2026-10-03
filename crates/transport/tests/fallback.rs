//! Failure paths, transport selection and builder options of the [`FallbackService`].
#![allow(missing_docs)]

use alloy_json_rpc::{
    ErrorPayload, Id, Request, RequestPacket, Response, ResponsePacket, ResponsePayload,
};
use alloy_transport::{
    layers::{FallbackLayer, FallbackService},
    TransportError, TransportErrorKind, TransportFut, TransportResult,
};
use serde_json::value::RawValue;
use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
};
use tower::{Layer, Service};

const NO_TRANSPORTS: &str = "fail:No transports available for fallback service";

#[derive(Clone, Copy, Debug)]
enum Outcome {
    Value(&'static str),
    RpcErr(&'static str),
    Fail(&'static str),
}

#[derive(Clone, Debug)]
struct Node {
    delay: Duration,
    outcome: Outcome,
    calls: Arc<AtomicUsize>,
}

impl Node {
    fn new(delay_ms: u64, outcome: Outcome) -> Self {
        Self { delay: Duration::from_millis(delay_ms), outcome, calls: Default::default() }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Service<RequestPacket> for Node {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let (delay, outcome) = (self.delay, self.outcome);
        Box::pin(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let id = req.as_single().expect("single request").id().clone();
            let payload = match outcome {
                Outcome::Value(value) => ResponsePayload::Success(
                    RawValue::from_string(serde_json::to_string(value).unwrap()).unwrap(),
                ),
                Outcome::RpcErr(message) => ResponsePayload::Failure(ErrorPayload {
                    code: -32000,
                    message: message.into(),
                    data: None,
                }),
                Outcome::Fail(message) => return Err(TransportErrorKind::custom_str(message)),
            };
            Ok(ResponsePacket::Single(Response { id, payload }))
        })
    }
}

fn request(method: &'static str) -> RequestPacket {
    Request::new(method, Id::Number(1), ()).serialize().unwrap().into()
}

fn describe(res: TransportResult<ResponsePacket>) -> String {
    match res {
        Ok(ResponsePacket::Single(Response {
            payload: ResponsePayload::Success(value), ..
        })) => {
            format!("ok:{}", serde_json::from_str::<String>(value.get()).unwrap())
        }
        Ok(ResponsePacket::Single(Response { payload: ResponsePayload::Failure(err), .. })) => {
            format!("rpc:{}", err.message)
        }
        Ok(ResponsePacket::Batch(_)) => panic!("unexpected batch response"),
        Err(err) => format!("fail:{err}"),
    }
}

#[tokio::test(start_paused = true)]
async fn fallback_returns_first_success_or_last_error() {
    use Outcome::{Fail, RpcErr, Value};

    const PARALLEL: &str = "eth_call";
    const SEQUENTIAL: &str = "eth_sendRawTransactionSync";

    type Case = (&'static str, &'static str, usize, Vec<(u64, Outcome)>, &'static str, Vec<usize>);
    let cases: Vec<Case> = vec![
        (
            "parallel, first fails",
            PARALLEL,
            3,
            vec![(0, Fail("a")), (1000, Value("b"))],
            "ok:b",
            vec![1, 1],
        ),
        (
            "parallel, json-rpc error wins",
            PARALLEL,
            3,
            vec![(1000, RpcErr("a")), (2000, Value("b"))],
            "rpc:a",
            vec![1, 1],
        ),
        (
            "parallel, all fail",
            PARALLEL,
            3,
            vec![(1000, Fail("a")), (0, Fail("b"))],
            "fail:a",
            vec![1, 1],
        ),
        (
            "parallel, fewer active than configured",
            PARALLEL,
            1,
            vec![(0, Fail("a")), (0, Value("b")), (0, Value("c"))],
            "fail:a",
            vec![1, 0, 0],
        ),
        (
            "sequential, falls through",
            SEQUENTIAL,
            3,
            vec![(0, Fail("a")), (0, Value("b")), (0, Value("c"))],
            "ok:b",
            vec![1, 1, 0],
        ),
        (
            "sequential, json-rpc error wins",
            SEQUENTIAL,
            3,
            vec![(0, RpcErr("a")), (0, Value("b"))],
            "rpc:a",
            vec![1, 0],
        ),
        (
            "sequential, all fail",
            SEQUENTIAL,
            3,
            vec![(0, Fail("a")), (0, Fail("b"))],
            "fail:b",
            vec![1, 1],
        ),
        (
            "sequential, fewer active than configured",
            SEQUENTIAL,
            2,
            vec![(0, Fail("a")), (0, Fail("b")), (0, Value("c"))],
            "fail:b",
            vec![1, 1, 0],
        ),
        ("parallel, no transports", PARALLEL, 3, vec![], NO_TRANSPORTS, vec![]),
        ("sequential, no transports", SEQUENTIAL, 3, vec![], NO_TRANSPORTS, vec![]),
        ("no active transports", PARALLEL, 0, vec![(0, Value("a"))], NO_TRANSPORTS, vec![0]),
    ];

    for (case, method, active, nodes, expected, expected_calls) in cases {
        let nodes: Vec<_> =
            nodes.into_iter().map(|(delay, outcome)| Node::new(delay, outcome)).collect();
        let mut service = FallbackService::new(nodes.clone(), active);

        assert_eq!(describe(service.call(request(method)).await), expected, "{case}");
        assert_eq!(nodes.iter().map(Node::calls).collect::<Vec<_>>(), expected_calls, "{case}");
    }
}

#[tokio::test(start_paused = true)]
async fn sequential_requests_prefer_transports_that_succeeded() {
    let failing = Node::new(0, Outcome::Fail("a"));
    let healthy = Node::new(0, Outcome::Value("b"));
    let mut service = FallbackService::new(vec![failing.clone(), healthy.clone()], 2);

    assert_eq!(describe(service.call(request("eth_sendRawTransactionSync")).await), "ok:b");
    let failing_calls = failing.calls();

    assert_eq!(describe(service.call(request("eth_sendRawTransactionSync")).await), "ok:b");
    assert_eq!(failing.calls(), failing_calls);
    assert_eq!(healthy.calls(), 2);
}

#[tokio::test(start_paused = true)]
async fn fallback_builder_options() {
    type Build = fn(Vec<Node>) -> FallbackService<Node>;

    let cases: [(&str, Build, &str, &str, [usize; 3]); 10] = [
        (
            "default, regular method",
            |n| FallbackLayer::default().layer(n),
            "eth_call",
            "ok:b",
            [1, 1, 1],
        ),
        (
            "default, raw tx sync",
            |n| FallbackLayer::default().layer(n),
            "eth_sendRawTransactionSync",
            "ok:b",
            [1, 1, 0],
        ),
        (
            "default, tx sync",
            |n| FallbackLayer::default().layer(n),
            "eth_sendTransactionSync",
            "ok:b",
            [1, 1, 0],
        ),
        (
            "one active transport",
            |n| FallbackLayer::default().with_active_transport_count(NonZeroUsize::MIN).layer(n),
            "eth_call",
            "fail:a",
            [1, 0, 0],
        ),
        (
            "added sequential method",
            |n| FallbackLayer::default().with_sequential_method("eth_call").layer(n),
            "eth_call",
            "ok:b",
            [1, 1, 0],
        ),
        (
            "added sequential method keeps defaults",
            |n| FallbackLayer::default().with_sequential_method("eth_call").layer(n),
            "eth_sendRawTransactionSync",
            "ok:b",
            [1, 1, 0],
        ),
        (
            "replaced sequential methods",
            |n| {
                FallbackLayer::default()
                    .with_sequential_methods(["eth_call".to_string()].into_iter().collect())
                    .layer(n)
            },
            "eth_sendRawTransactionSync",
            "ok:b",
            [1, 1, 1],
        ),
        (
            "no sequential methods",
            |n| FallbackLayer::default().without_sequential_methods().layer(n),
            "eth_sendRawTransactionSync",
            "ok:b",
            [1, 1, 1],
        ),
        (
            "service appends sequential method",
            |n| FallbackService::new(n, 3).append_sequential_method("eth_call"),
            "eth_call",
            "ok:b",
            [1, 1, 0],
        ),
        (
            "service append keeps defaults",
            |n| FallbackService::new(n, 3).append_sequential_method("eth_call"),
            "eth_sendRawTransactionSync",
            "ok:b",
            [1, 1, 0],
        ),
    ];

    for (case, build, method, expected, expected_calls) in cases {
        let nodes = vec![
            Node::new(0, Outcome::Fail("a")),
            Node::new(1000, Outcome::Value("b")),
            Node::new(2000, Outcome::Value("c")),
        ];
        let mut service = build(nodes.clone());

        assert_eq!(describe(service.call(request(method)).await), expected, "{case}");
        assert_eq!(nodes.iter().map(Node::calls).collect::<Vec<_>>(), expected_calls, "{case}");
    }
}
