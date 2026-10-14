//! The [`TraceParentLayer`] on a client transport.
#![cfg(all(feature = "traceparent", not(target_family = "wasm")))]
#![allow(missing_docs)]

use alloy_json_rpc::{Id, Request, RequestPacket};
use alloy_transport::{BoxTransport, TransportFut};
use alloy_transport_http::TraceParentLayer;
use opentelemetry::{
    propagation::{text_map_propagator::FieldIter, Extractor, Injector, TextMapPropagator},
    Context,
};
use std::sync::{Arc, Mutex};
use tower::{Layer, Service};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

/// Injects a fixed `traceparent` header.
#[derive(Debug)]
struct FixedTraceParent(Vec<String>);

impl TextMapPropagator for FixedTraceParent {
    fn inject_context(&self, _cx: &Context, injector: &mut dyn Injector) {
        injector.set("traceparent", TRACEPARENT.to_owned());
    }

    fn extract_with_context(&self, cx: &Context, _extractor: &dyn Extractor) -> Context {
        cx.clone()
    }

    fn fields(&self) -> FieldIter<'_> {
        FieldIter::new(&self.0)
    }
}

#[test]
fn trace_parent_layer_wraps_a_client_transport() {
    opentelemetry::global::set_text_map_propagator(FixedTraceParent(vec!["traceparent".into()]));

    let seen = Arc::new(Mutex::new(None));
    let inner = {
        let seen = seen.clone();
        tower::service_fn(move |req: RequestPacket| {
            *seen.lock().unwrap() =
                req.headers().get("traceparent").map(|v| v.to_str().unwrap().to_owned());
            // The header is observed when the request is handed over, the response is not needed.
            Box::pin(std::future::pending()) as TransportFut<'static>
        })
    };

    // Client transports are boxed, which requires the layered service to be `Clone`.
    let mut transport = BoxTransport::new(TraceParentLayer.layer(inner));
    let request = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
    drop(transport.call(request.into()));

    assert_eq!(seen.lock().unwrap().as_deref(), Some(TRACEPARENT));
}
