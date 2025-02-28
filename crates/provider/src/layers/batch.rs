use crate::{Provider, ProviderLayer};
use alloy_network::Network;
use std::time::Duration;

/// A layer that batches multiple requests into a single request.
#[non_exhaustive]
pub struct BatchLayer {
    delay: Duration,
}

impl BatchLayer {
    pub const fn new() -> Self {
        Self { delay: Duration::from_millis(1) }
    }

    pub const fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

impl<P, N> ProviderLayer<P, N> for BatchLayer
where
    P: Provider<N>,
    N: Network,
{
    type Provider = P;

    fn layer(&self, inner: P) -> Self::Provider {
        inner
    }
}

pub struct BatchProvider<P, N> {
    inner: P,
    delay: Duration,
    _pd: PhantomData<N>,
}

impl<P, N> BatchProvider<P, N> {
    fn new(inner: P, delay: Duration) -> Self {
        Self { inner, delay, _pd: PhantomData }
    }
}

impl<P, N> Provider<N> for BatchProvider<P, N>
where
    P: Provider<N>,
    N: Network,
{
}
