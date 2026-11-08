//! Ranking of untried and failing transports in the [`FallbackService`].
#![allow(missing_docs)]

use alloy_json_rpc::{Id, Request, RequestPacket, Response, ResponsePacket, ResponsePayload};
use alloy_transport::{layers::FallbackService, TransportErrorKind, TransportFut};
use serde_json::value::RawValue;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tower::{util::ServiceFn, Service};

/// A transport that answers its first `successes` requests and fails afterwards.
fn node(
    calls: &Arc<AtomicUsize>,
    successes: usize,
) -> ServiceFn<impl FnMut(RequestPacket) -> TransportFut<'static> + Clone + Send + Sync + 'static> {
    let calls = calls.clone();
    tower::service_fn(move |req: RequestPacket| {
        let call = calls.fetch_add(1, Ordering::SeqCst);
        let id = req.as_single().unwrap().id().clone();
        Box::pin(async move {
            if call >= successes {
                return Err(TransportErrorKind::custom_str("down"));
            }
            Ok(ResponsePacket::Single(Response {
                id,
                payload: ResponsePayload::Success(RawValue::from_string("\"0x1\"".into()).unwrap()),
            }))
        }) as TransportFut<'static>
    })
}

fn request() -> RequestPacket {
    Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap().into()
}

#[tokio::test]
async fn failed_transport_is_not_preferred_over_untried_one() {
    let calls = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let mut service =
        FallbackService::new(vec![node(&calls[0], 0), node(&calls[1], usize::MAX)], 1);

    assert!(service.call(request()).await.is_err());
    assert!(service.call(request()).await.is_ok(), "the untried transport should be used next");
    assert_eq!(calls[0].load(Ordering::SeqCst), 1);
    assert_eq!(calls[1].load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn transport_that_stopped_succeeding_falls_below_untried_one() {
    let calls = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let mut service =
        FallbackService::new(vec![node(&calls[0], 1), node(&calls[1], usize::MAX)], 1);

    assert!(service.call(request()).await.is_ok());
    let mut failures = 0;
    while service.call(request()).await.is_err() {
        failures += 1;
        assert!(failures <= 20, "the untried transport is never used");
    }
    assert_eq!(calls[1].load(Ordering::SeqCst), 1);
}
