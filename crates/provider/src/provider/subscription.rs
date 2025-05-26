use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::B256;
use alloy_pubsub::Subscription;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_transport::{TransportErrorKind, TransportResult};

/// A general-purpose subscription request builder
///
/// This struct allows configuring subscription parameters and channel size
/// before initiating a request to subscribe to Ethereum events.
pub struct GetSubscription<P, R>
where
    P: RpcSend,
    R: RpcRecv,
{
    client: WeakClient,
    call: RpcCall<P, B256>,
    channel_size: Option<usize>,
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<P, R> GetSubscription<P, R>
where
    P: RpcSend,
    R: RpcRecv,
{
    /// Creates a new [`GetSubscription`] instance
    pub fn new(client: WeakClient, call: RpcCall<P, B256>) -> Self {
        Self { client, call, channel_size: None, _marker: std::marker::PhantomData }
    }

    /// Set the channel_size for the subscription stream.
    pub const fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = Some(size);
        self
    }
}

impl<P, R> core::fmt::Debug for GetSubscription<P, R>
where
    P: RpcSend,
    R: RpcRecv,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GetSubscription")
            .field("channel_size", &self.channel_size)
            .field("call", &self.call)
            .finish()
    }
}

impl<P, R> std::future::IntoFuture for GetSubscription<P, R>
where
    P: RpcSend + 'static,
    R: RpcRecv,
{
    type Output = TransportResult<alloy_pubsub::Subscription<R>>;
    type IntoFuture = futures_utils_wasm::BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let client = self
                .client
                .upgrade()
                .ok_or_else(|| TransportErrorKind::custom_str("client dropped"))?;
            let pubsub = client.pubsub_frontend().ok_or(TransportErrorKind::PubsubUnavailable)?;

            // Set config channel size if any
            if let Some(size) = self.channel_size {
                pubsub.set_channel_size(size);
            }

            let id = self.call.await?;

            pubsub.get_subscription(id).await.map(Subscription::from)
        })
    }
}
