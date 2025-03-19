use alloy_json_rpc::RpcRecv;
use alloy_pubsub::Subscription;
use alloy_rpc_client::WeakClient;
use alloy_rpc_types_eth::{
    pubsub::{Params, SubscriptionKind},
    Filter,
};
use alloy_transport::{TransportErrorKind, TransportResult};

/// A builder for `"eth_subscribe"`  requests.
///
/// This struct allows configuring subscription parameters and channel size
/// before initiating a request to subscribe to Ethereum events.
pub struct GetSubscription<R>
where
    R: RpcRecv,
{
    client: WeakClient,
    kind: SubscriptionKind,
    params: Params,
    channel_size: Option<usize>,
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<R> GetSubscription<R>
where
    R: RpcRecv,
{
    /// Creates a new [`GetSubscription`] instance
    ///
    /// By default, this sets the [`SubscriptionKind`] to [`SubscriptionKind::NewHeads`] and params
    /// to [`Params::None`].
    pub fn new(client: WeakClient) -> Self {
        Self {
            client,
            kind: SubscriptionKind::NewHeads,
            params: Params::None,
            channel_size: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the [`SubscriptionKind`]
    pub fn kind(mut self, kind: SubscriptionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the params for the subscription
    pub fn params(mut self, params: Params) -> Self {
        self.params = params;
        self
    }

    /// Create a [`SubscriptionKind::NewHeads`] subscription
    pub fn new_heads(client: WeakClient) -> Self {
        Self::new(client).kind(SubscriptionKind::NewHeads)
    }

    /// Create a [`SubscriptionKind::Logs`] subscription
    pub fn logs(client: WeakClient, filter: Filter) -> Self {
        Self::new(client).kind(SubscriptionKind::Logs).params(Params::Logs(Box::new(filter)))
    }

    /// Create a [`SubscriptionKind::Syncing`] subscription
    pub fn syncing(client: WeakClient) -> Self {
        Self::new(client).kind(SubscriptionKind::Syncing)
    }

    /// Create a [`SubscriptionKind::NewPendingTransactions`] subscription
    pub fn new_pending_transactions(client: WeakClient) -> Self {
        Self::new(client).kind(SubscriptionKind::NewPendingTransactions).params(Params::Bool(false))
    }

    /// Set the channel_size for the subscription stream.
    pub fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = Some(size);
        self
    }

    /// Applies only to [`SubscriptionKind::NewPendingTransactions`] requests.
    ///
    /// Set to `true` to receive the full pending transactions and not just the transaction hash.
    pub fn full_pending_txs(mut self, yes: bool) -> Self {
        self.params = Params::Bool(yes);
        self
    }
}

impl<R> core::fmt::Debug for GetSubscription<R>
where
    R: RpcRecv,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GetSubscription")
            .field("channel_size", &self.channel_size)
            .field("kind", &self.kind)
            .finish()
    }
}

impl<R> std::future::IntoFuture for GetSubscription<R>
where
    R: RpcRecv,
{
    type Output = TransportResult<alloy_pubsub::Subscription<R>>;
    type IntoFuture = futures_utils_wasm::BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let client =
                self.client.upgrade().ok_or(TransportErrorKind::custom_str("client dropped"))?;
            let pubsub = client.pubsub_frontend().ok_or(TransportErrorKind::PubsubUnavailable)?;

            let id = match self.kind {
                SubscriptionKind::NewHeads => {
                    client.request("eth_subscribe", ("newHeads",)).await?
                }
                SubscriptionKind::Logs => {
                    client.request("eth_subscribe", ("logs", self.params)).await?
                }
                SubscriptionKind::Syncing => client.request("eth_subscribe", ("syncing",)).await?,
                SubscriptionKind::NewPendingTransactions => {
                    client.request("eth_subscribe", ("newPendingTransactions", self.params)).await?
                }
            };

            // Set config channel size if any
            if let Some(size) = self.channel_size {
                pubsub.set_channel_size(size);
            }

            pubsub.get_subscription(id).await.map(Subscription::from)
        })
    }
}
