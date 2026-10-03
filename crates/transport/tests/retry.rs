//! Retry decisions, limits and backoff of the [`RetryBackoffLayer`].
#![allow(missing_docs)]

use alloy_json_rpc::{
    ErrorPayload, Id, Request, RequestPacket, Response, ResponsePacket, ResponsePayload,
};
use alloy_transport::{
    layers::{RateLimitRetryPolicy, RetryBackoffLayer, RetryPolicy},
    RpcError, TransportError, TransportErrorKind, TransportFut, TransportResult,
};
use serde_json::{json, value::RawValue, Value};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};
use tower::{Layer, Service};

/// One reply per attempt: an error payload for each failing member of the packet, or a transport
/// error for the whole packet.
type Reply = TransportResult<Vec<Option<ErrorPayload>>>;

type Calls = Arc<Mutex<Vec<RequestPacket>>>;

fn scripted(
    replies: impl IntoIterator<Item = Reply>,
) -> (
    impl Service<
            RequestPacket,
            Response = ResponsePacket,
            Error = TransportError,
            Future = TransportFut<'static>,
        > + Clone
        + Send
        + 'static,
    Calls,
) {
    let replies = Arc::new(Mutex::new(replies.into_iter().collect::<VecDeque<_>>()));
    let calls = Calls::default();
    let log = calls.clone();
    let service = tower::service_fn(move |req: RequestPacket| {
        log.lock().unwrap().push(req.clone());
        let reply = replies.lock().unwrap().pop_front().expect("unexpected request");
        let res = reply.map(|payloads| respond(&req, payloads));
        Box::pin(async move { res }) as TransportFut<'static>
    });
    (service, calls)
}

fn respond(req: &RequestPacket, payloads: Vec<Option<ErrorPayload>>) -> ResponsePacket {
    let mut responses = req.requests().iter().zip(payloads).map(|(req, error)| Response {
        id: req.id().clone(),
        payload: match error {
            Some(error) => ResponsePayload::Failure(error),
            None => ResponsePayload::Success(RawValue::from_string("\"0x1\"".into()).unwrap()),
        },
    });
    match req {
        RequestPacket::Single(_) => ResponsePacket::Single(responses.next().unwrap()),
        RequestPacket::Batch(_) => ResponsePacket::Batch(responses.collect()),
    }
}

fn payload(code: i64, message: &'static str) -> ErrorPayload {
    ErrorPayload { code, message: message.into(), data: None }
}

fn payload_with_data(code: i64, message: &'static str, data: Value) -> ErrorPayload {
    ErrorPayload {
        code,
        message: message.into(),
        data: Some(RawValue::from_string(data.to_string()).unwrap()),
    }
}

fn ok() -> Reply {
    Ok(vec![None])
}

fn rpc_err(code: i64, message: &'static str) -> Reply {
    Ok(vec![Some(payload(code, message))])
}

fn single() -> RequestPacket {
    Request::new("eth_call", Id::Number(1), ()).serialize().unwrap().into()
}

fn batch() -> RequestPacket {
    (1..=3).map(|id| Request::new("eth_call", Id::Number(id), ()).serialize().unwrap()).collect()
}

/// Reduces a result to what a caller observes: success, or the message of the first error,
/// whether it arrives as an error payload inside the packet or as an error.
fn outcome(res: &TransportResult<ResponsePacket>) -> Result<(), String> {
    match res {
        Ok(packet) => packet.as_error().map_or(Ok(()), |err| Err(err.to_string())),
        Err(err) => Err(err.to_string()),
    }
}

fn first_error_code(res: &TransportResult<ResponsePacket>) -> Option<i64> {
    match res {
        Ok(packet) => packet.first_error_code(),
        Err(err) => err.as_error_resp().map(|err| err.code),
    }
}

#[tokio::test(start_paused = true)]
async fn retry_loop_outcomes() {
    type Case = (&'static str, u32, Vec<Reply>, Result<(), &'static str>, usize);
    let rate_limited = || rpc_err(429, "rate limited");
    let cases: [Case; 6] = [
        ("rate limit payload, then success", 3, vec![rate_limited(), ok()], Ok(()), 2),
        (
            "retryable transport error, then success",
            3,
            vec![Err(TransportErrorKind::http_error(503, String::new())), ok()],
            Ok(()),
            2,
        ),
        (
            "non-retryable payload",
            3,
            vec![rpc_err(-32000, "execution reverted")],
            Err("execution reverted"),
            1,
        ),
        (
            "non-retryable transport error",
            3,
            vec![Err(TransportErrorKind::custom_str("connection refused"))],
            Err("connection refused"),
            1,
        ),
        (
            "retries exhausted",
            2,
            vec![rate_limited(), rate_limited(), rate_limited()],
            Err("Max retries exceeded"),
            3,
        ),
        ("no retries allowed", 0, vec![rate_limited()], Err("Max retries exceeded"), 1),
    ];

    for (case, max_retries, replies, expected, expected_calls) in cases {
        let (service, calls) = scripted(replies);
        let mut service = RetryBackoffLayer::new(max_retries, 0, 10_000).layer(service);

        let res = service.call(single()).await;
        match (outcome(&res), expected) {
            (Ok(()), Ok(())) => {}
            (Err(err), Err(expected)) => assert!(err.contains(expected), "{case}: {err}"),
            (actual, expected) => panic!("{case}: expected {expected:?}, got {actual:?}"),
        }
        assert_eq!(calls.lock().unwrap().len(), expected_calls, "{case}");
    }
}

#[tokio::test(start_paused = true)]
async fn retry_waits_for_backoff_hint() {
    let infura = |seconds: Value| {
        Ok(vec![Some(payload_with_data(
            -32005,
            "project rate limit",
            json!({"rate": {"backoff_seconds": seconds}}),
        ))])
    };
    let cases: [(&str, Reply, Duration); 5] = [
        ("initial backoff without hint", rpc_err(-32005, "limit"), Duration::from_millis(100)),
        ("infura backoff seconds", infura(json!(3)), Duration::from_secs(3)),
        ("fractional backoff seconds round up", infura(json!(1.5)), Duration::from_secs(2)),
        (
            "message hint",
            rpc_err(-32005, "rate limited, try again in 250ms"),
            Duration::from_millis(250),
        ),
        (
            "retry-after header",
            Err(TransportErrorKind::http_error_with_retry_after(
                429,
                String::new(),
                Some(Duration::from_secs(7)),
            )),
            Duration::from_secs(7),
        ),
    ];

    for (case, first, wait) in cases {
        let (service, calls) = scripted([first, ok()]);
        let mut service = RetryBackoffLayer::new(1, 100, 10_000).layer(service);

        let mut fut = service.call(single());
        assert!(futures::poll!(&mut fut).is_pending(), "{case}");
        tokio::time::advance(wait - Duration::from_millis(1)).await;
        assert!(futures::poll!(&mut fut).is_pending(), "{case}");
        assert_eq!(calls.lock().unwrap().len(), 1, "{case}");

        tokio::time::advance(Duration::from_millis(1)).await;
        assert!(matches!(futures::poll!(&mut fut), Poll::Ready(Ok(_))), "{case}");
        assert_eq!(calls.lock().unwrap().len(), 2, "{case}");
    }
}

#[tokio::test(start_paused = true)]
async fn or_retry_policy_extends_rate_limit_policy() {
    let policy = RateLimitRetryPolicy::default()
        .or(|err: &TransportError| err.to_string().contains("connection reset"));
    let cases: [(Reply, usize); 3] = [
        (Err(TransportErrorKind::custom_str("connection reset")), 2),
        (rpc_err(429, "rate limited"), 2),
        (Err(TransportErrorKind::custom_str("connection refused")), 1),
    ];

    for (first, expected_calls) in cases {
        let (service, calls) = scripted([first, ok()]);
        let mut service =
            RetryBackoffLayer::new_with_policy(1, 0, 10_000, policy.clone()).layer(service);

        let res = service.call(single()).await;
        assert_eq!(res.is_ok(), expected_calls == 2, "{res:?}");
        assert_eq!(calls.lock().unwrap().len(), expected_calls);
    }

    let err = TransportErrorKind::http_error_with_retry_after(
        429,
        String::new(),
        Some(Duration::from_secs(7)),
    );
    assert_eq!(policy.backoff_hint(&err), Some(Duration::from_secs(7)));
}

#[tokio::test(start_paused = true)]
async fn batch_retry_is_decided_by_first_error() {
    let rate_limited = || Some(payload(429, "rate limited"));
    let reverted = || Some(payload(-32000, "execution reverted"));
    let cases = [
        (vec![None, rate_limited(), reverted()], 2, None),
        (vec![None, reverted(), rate_limited()], 1, Some(-32000)),
    ];

    for (first, expected_calls, expected_error) in cases {
        let (service, calls) = scripted([Ok(first), Ok(vec![None, None, None])]);
        let mut service = RetryBackoffLayer::new(3, 0, 10_000).layer(service);

        let res = service.call(batch()).await;
        assert_eq!(first_error_code(&res), expected_error);

        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), expected_calls);
        for call in calls.iter() {
            let ids: Vec<_> = call.requests().iter().map(|req| req.id().clone()).collect();
            assert_eq!(ids, [Id::Number(1), Id::Number(2), Id::Number(3)]);
        }
    }
}

#[test]
fn rate_limit_policy_classification() {
    let deser = |text: &str| RpcError::DeserError {
        err: serde_json::from_str::<u8>("x").unwrap_err(),
        text: text.to_owned(),
    };
    let resp = |code, message| TransportError::ErrorResp(payload(code, message));
    let infura = |data: Value| {
        TransportError::ErrorResp(payload_with_data(-32005, "rate limited, try again in 4ms", data))
    };

    let cases: Vec<(TransportError, bool, Option<Duration>)> = vec![
        (resp(-32016, "rate limit exceeded"), true, None),
        (resp(-32016, "over quota"), false, None),
        (resp(-32012, "credits exhausted"), true, None),
        (resp(-32012, "boom"), false, None),
        (resp(-32055, "upstream unavailable"), true, None),
        (resp(-32000, "header not found"), true, None),
        (resp(-32000, "daily request count exceeded, request rate limited"), true, None),
        (resp(-32000, "rate exceeded"), true, None),
        (resp(-32000, "too many requests"), true, None),
        (resp(-32000, "credits limited to 6000/sec"), true, None),
        (resp(-32000, "exceeded request limit"), true, None),
        (resp(-32000, "maximum number of concurrent requests"), true, None),
        (resp(-32000, "execution reverted"), false, None),
        (infura(json!({"rate": {"backoff_seconds": 30}})), true, Some(Duration::from_secs(30))),
        (infura(json!({"rate": {"backoff_seconds": 1.5}})), true, Some(Duration::from_secs(2))),
        (infura(json!({"other": 1})), true, Some(Duration::from_millis(4))),
        (TransportErrorKind::missing_batch_response(Id::Number(1)), true, None),
        (TransportErrorKind::backend_gone(), false, None),
        (TransportErrorKind::pubsub_unavailable(), false, None),
        (TransportErrorKind::http_error(429, String::new()), true, None),
        (TransportErrorKind::http_error(503, String::new()), true, None),
        (TransportErrorKind::custom_str("HTTP 429 Too Many Requests"), true, None),
        (TransportErrorKind::custom_str("connection refused"), false, None),
        (TransportErrorKind::non_retryable_str("HTTP 429 Too Many Requests"), false, None),
        (RpcError::NullResp, true, None),
        (RpcError::SerError(serde_json::from_str::<u8>("x").unwrap_err()), false, None),
        (RpcError::LocalUsageError("no signer".into()), false, None),
        (deser(r#"{"code":429,"message":"slow down"}"#), true, None),
        (deser(r#"{"jsonrpc":"2.0","error":{"code":-32005,"message":"limit"}}"#), true, None),
        (deser(r#"{"code":-32000,"message":"execution reverted"}"#), false, None),
        (deser("<html>bad gateway</html>"), false, None),
    ];

    let policy = RateLimitRetryPolicy::default();
    for (err, retry, hint) in cases {
        assert_eq!(policy.should_retry(&err), retry, "{err}");
        assert_eq!(policy.backoff_hint(&err), hint, "{err}");
    }
}
