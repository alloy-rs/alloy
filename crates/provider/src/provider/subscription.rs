use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::B256;
use alloy_pubsub::{Subscription, SubscriptionReceiverTicket, SubscriptionRetentionPolicy};
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_transport::{TransportErrorKind, TransportResult};
use std::borrow::Cow;

/// A general-purpose subscription request builder
///
/// This struct allows configuring subscription parameters, channel size, and retention before
/// initiating a request to subscribe to Ethereum events. Typed subscriptions default to automatic
/// cleanup after their final local receiver is dropped.
pub struct GetSubscription<P, R>
where
    P: RpcSend,
    R: RpcRecv,
{
    client: WeakClient,
    call: RpcCall<P, B256>,
    channel_size: Option<usize>,
    retention_policy: SubscriptionRetentionPolicy,
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<P, R> GetSubscription<P, R>
where
    P: RpcSend,
    R: RpcRecv,
{
    /// Creates a new [`GetSubscription`] instance
    pub fn new(client: WeakClient, call: RpcCall<P, B256>) -> Self {
        Self {
            client,
            call,
            channel_size: None,
            retention_policy: SubscriptionRetentionPolicy::WhileReceivers,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the channel_size for the subscription stream.
    pub const fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = Some(size);
        self
    }

    /// Set the RPC method used to remove the server-side subscription.
    pub fn unsubscribe_method(mut self, method: impl Into<Cow<'static, str>>) -> Self {
        self.call.set_unsubscribe_method(method);
        self
    }

    /// Set when the server-side subscription is eligible for automatic cleanup.
    ///
    /// Typed subscriptions default to [`SubscriptionRetentionPolicy::WhileReceivers`].
    pub const fn retention_policy(mut self, policy: SubscriptionRetentionPolicy) -> Self {
        self.retention_policy = policy;
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
            .field("retention_policy", &self.retention_policy)
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

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move {
            let client = self
                .client
                .upgrade()
                .ok_or_else(|| TransportErrorKind::custom_str("client dropped"))?;
            client.pubsub_frontend().ok_or(TransportErrorKind::PubsubUnavailable)?;

            if let Some(size) = self.channel_size {
                if size == 0 {
                    return Err(alloy_json_rpc::RpcError::local_usage_str(
                        "subscription channel size must be non-zero",
                    ));
                }
                self.call.set_subscription_channel_size(size);
            }

            let (ticket, receiver) = SubscriptionReceiverTicket::channel();
            self.call.set_subscription_receiver_ticket(ticket);
            self.call.set_subscription_retention_policy(self.retention_policy);

            let id = self.call.await?;
            let subscription = receiver.await.map_err(|_| TransportErrorKind::backend_gone())?;
            debug_assert_eq!(&id, subscription.local_id());
            Ok(Subscription::from(subscription))
        })
    }
}
