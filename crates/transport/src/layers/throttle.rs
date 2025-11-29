use crate::{TransportError, TransportFut};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use governor::{
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{
    num::NonZeroU32,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// A rate limiter for throttling RPC requests.
type Throttle = RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

/// A Transport Layer responsible for throttling RPC requests.
#[derive(Debug)]
pub struct ThrottleLayer {
    /// Rate limiter used to throttle requests.
    pub throttle: Arc<Throttle>,
}

impl ThrottleLayer {
    /// Creates a new throttle layer with the specified requests per second.
    ///
    /// # Panics
    ///
    /// Panics if `requests_per_second` is 0.
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second)
                .expect("Request per second must be greater than 0"),
        )
        .allow_burst(NonZeroU32::new(1).unwrap());
        let throttle = Arc::new(RateLimiter::direct(quota));

        Self { throttle }
    }
}

/// A Tower Service used by the ThrottleLayer that is responsible for throttling rpc requests.
#[derive(Debug, Clone)]
pub struct ThrottleService<S> {
    /// The inner service
    inner: S,
    throttle: Arc<Throttle>,
}

impl<S> Layer<S> for ThrottleLayer {
    type Service = ThrottleService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ThrottleService { inner, throttle: self.throttle.clone() }
    }
}

impl<S> Service<RequestPacket> for ThrottleService<S>
where
    S: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>
        + Send
        + 'static
        + Clone,
    S::Future: Send + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: RequestPacket) -> Self::Future {
        let throttle = self.throttle.clone();
        let inner_clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner_clone);

        Box::pin(async move {
            throttle.until_ready().await;
            inner.call(request).await
        })
    }
}
