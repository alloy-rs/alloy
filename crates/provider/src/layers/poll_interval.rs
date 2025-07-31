use crate::{Provider, ProviderLayer};
use alloy_network::Network;
use std::time::Duration;

/// Sets the poll interval for the client.
///
/// This has no effect if the client is using a local transport.
#[derive(Debug, Clone, Copy)]
pub struct PollIntervalLayer {
    poll_interval: Duration,
}

impl PollIntervalLayer {
    /// Create a new `PollIntervalLayer` from the given duration.
    pub const fn new(poll_interval: Duration) -> Self {
        Self { poll_interval }
    }
}

impl<P, N> ProviderLayer<P, N> for PollIntervalLayer
where
    P: Provider<N>,
    N: Network,
{
    type Provider = P;
    fn layer(&self, inner: P) -> Self::Provider {
        if !inner.client().is_local() {
            inner.client().set_poll_interval(self.poll_interval);
        }
        inner
    }
}
