use alloy_json_rpc::RequestPacket;
use tower::{Layer, Service};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// A layer to propagate trace context using W3C `traceparent` header standard.
///
/// This layer injects the `traceparent` header into outgoing requests, enabling
/// distributed tracing across services that support the W3C Trace Context
/// specification.
#[derive(Debug, Default, Clone, Copy)]
pub struct TraceParentLayer;

impl<S> Layer<S> for TraceParentLayer {
    type Service = TraceParentService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceParentService { inner }
    }
}

/// A service that injects trace context into requests using W3C `traceparent`
/// header standard.
///
/// This service wraps another service and adds the `traceparent` header to each
/// outgoing request, allowing for trace context propagation.
#[derive(Debug)]
pub struct TraceParentService<S> {
    inner: S,
}

impl<S> Service<RequestPacket> for TraceParentService<S>
where
    S: Service<RequestPacket> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: RequestPacket) -> Self::Future {
        // Insert the header into the LAST request in the batch. This ensures
        // that this will override any other traceparents.
        if let Some(req) = req.requests_mut().last_mut() {
            let mut injector = opentelemetry_http::HeaderInjector(req.headers_mut());

            let ctx = tracing::Span::current().context();

            opentelemetry::global::get_text_map_propagator(|propagator| {
                propagator.inject_context(&ctx, &mut injector)
            });
        }

        self.inner.call(req)
    }
}
