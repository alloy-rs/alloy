use crate::{Provider, RootProvider};
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network::{Ethereum, Network};
use alloy_transport::TransportResult;
use std::{borrow::Cow, future::Future, pin::Pin};

/// A general-purpose subscription request builder
///
/// This struct allows configuring subscription parameters and channel size
/// before initiating a request to subscribe to Ethereum events.
pub struct GetSubscription<P, R, N = Ethereum>
where
    P: RpcSend,
    R: RpcRecv,
    N: Network,
{
    root: RootProvider<N>,
    method: Cow<'static, str>,
    params: Option<P>,
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
    pub fn new(
        root: RootProvider<N>,
        method: impl Into<Cow<'static, str>>,
        params: Option<P>,
    ) -> Self {
        Self {
            root,
            method: method.into(),
            channel_size: None,
            params,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the channel_size for the subscription stream.
    pub fn channel_size(mut self, size: usize) -> Self {
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
        f.debug_struct("GetSubscription")
            .field("channel_size", &self.channel_size)
            .field("method", &self.method)
            .finish()
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
            let pubsub = self.root.pubsub_frontend()?;

            // Set config channel size if any
            if let Some(size) = self.channel_size {
                pubsub.set_channel_size(size);
            }

            // Handle params and no-params case separately
            let id = if let Some(params) = self.params {
                let mut call = self.root.client().request(self.method, params);
                call.set_is_subscription();
                call.await?
            } else {
                let mut call = self.root.client().request_noparams(self.method);
                call.set_is_subscription();
                call.await?
            };

            self.root.get_subscription(id).await
        })
    }
}
