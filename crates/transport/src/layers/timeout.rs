use crate::{TransportError, TransportErrorKind, TransportFut};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use std::{
    task::{Context, Poll},
    time::Duration,
};
use tower::{Layer, Service};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::timeout;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::timeout;

/// A transport layer that fails a request with [`TransportErrorKind::Timeout`] when the inner
/// service does not respond within a fixed duration.
///
/// The deadline covers one call to the inner service. Added after a
/// [`RetryBackoffLayer`](super::RetryBackoffLayer) on a client builder, it bounds each attempt;
/// added before it, it bounds the request with all of its retries.
///
/// A timeout only stops waiting for the response: it does not cancel a request the server already
/// received.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    /// Creates a layer that fails requests after `timeout`.
    pub const fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimeoutService::new(inner, self.timeout)
    }
}

/// A service that fails a request with [`TransportErrorKind::Timeout`] when the inner service
/// does not respond within a fixed duration. See [`TimeoutLayer`].
#[derive(Debug, Clone)]
pub struct TimeoutService<S> {
    inner: S,
    timeout: Duration,
}

impl<S> TimeoutService<S> {
    /// Wraps `inner` so that its requests fail after `timeout`.
    pub const fn new(inner: S, timeout: Duration) -> Self {
        Self { inner, timeout }
    }
}

impl<S> Service<RequestPacket> for TimeoutService<S>
where
    S: Service<
        RequestPacket,
        Response = ResponsePacket,
        Future = TransportFut<'static>,
        Error = TransportError,
    >,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: RequestPacket) -> Self::Future {
        let response = self.inner.call(request);
        let duration = self.timeout;
        Box::pin(async move {
            timeout(duration, response).await.map_err(|_| TransportErrorKind::timeout(duration))?
        })
    }
}

#[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request, Response, ResponsePayload};
    use serde_json::value::RawValue;

    /// Responds after `delay`, or never when `delay` is `None`.
    #[derive(Clone)]
    struct DelayedService(Option<Duration>);

    impl Service<RequestPacket> for DelayedService {
        type Response = ResponsePacket;
        type Error = TransportError;
        type Future = TransportFut<'static>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: RequestPacket) -> Self::Future {
            let delay = self.0;
            Box::pin(async move {
                match delay {
                    Some(delay) => tokio::time::sleep(delay).await,
                    None => std::future::pending().await,
                }
                let result = RawValue::from_string("\"0x1\"".to_owned()).unwrap();
                Ok(ResponsePacket::Single(Response {
                    id: Id::Number(0),
                    payload: ResponsePayload::Success(result),
                }))
            })
        }
    }

    fn request() -> RequestPacket {
        RequestPacket::Single(Request::new("eth_chainId", Id::Number(0), ()).serialize().unwrap())
    }

    #[tokio::test]
    async fn fails_request_without_response() {
        let mut service = TimeoutLayer::new(Duration::from_millis(50)).layer(DelayedService(None));
        let err = service.call(request()).await.unwrap_err();
        assert!(matches!(
            err.as_transport_err(),
            Some(TransportErrorKind::Timeout(timeout)) if *timeout == Duration::from_millis(50)
        ));
        assert_eq!(err.to_string(), "timed out after 50ms");
    }

    #[tokio::test]
    async fn passes_response_within_timeout() {
        let mut service = TimeoutLayer::new(Duration::from_secs(10))
            .layer(DelayedService(Some(Duration::from_millis(10))));
        let response = service.call(request()).await.unwrap();
        assert!(response.is_success());
    }
}
