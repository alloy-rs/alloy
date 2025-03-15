use crate::{Provider, RootProvider};
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network::{Ethereum, Network};
use alloy_transport::TransportResult;
use std::{future::Future, pin::Pin};

/// Future type Subscription struct that wraps client requests to `eth_subscribe`
/// Allows configuration of channel size
pub struct GetSubscription<P, R, N = Ethereum>
where
    P: RpcSend,
    R: RpcRecv,
    N: Network,
{
    root: RootProvider<N>,
    params: P,
    channel_size: Option<usize>,
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<P, R, N> GetSubscription<P, R, N>
where
    N: Network,
    P: RpcSend,
    R: RpcRecv,
{
    /// Creates a new [`GetSubscription`] instance
    pub fn new(root: RootProvider<N>, params: P) -> Self {
        Self { root, channel_size: None, params, _marker: std::marker::PhantomData }
    }

    /// Set the channel_size for the subscription stream.
    pub fn buffer(mut self, size: usize) -> Self {
        self.channel_size = Some(size);
        self
    }
}

impl<P, R, N> core::fmt::Debug for GetSubscription<P, R, N>
where
    N: Network,
    P: RpcSend,
    R: RpcRecv,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GetSubscription").field("channel_size", &self.channel_size).finish()
    }
}

#[cfg(feature = "pubsub")]
impl<P, R, N> std::future::IntoFuture for GetSubscription<P, R, N>
where
    N: Network,
    P: RpcSend + 'static,
    R: RpcRecv,
{
    type Output = TransportResult<alloy_pubsub::Subscription<R>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.root
                .pubsub_frontend()?
                .set_channel_size(self.channel_size.unwrap_or(16)); //default size

            let id = self.root.client().request("eth_subscribe", self.params).await?;
            self.root.get_subscription(id).await
        })
    }
}
