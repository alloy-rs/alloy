//! Ethereum JSON-RPC provider.

use crate::{
    chain::ChainStreamPoller,
    heart::{Heartbeat, HeartbeatHandle, PendingTransaction, PendingTransactionConfig},
    utils::{self, Eip1559Estimation, EstimatorFunction},
    PendingTransactionBuilder,
};
use alloy_eips::eip2718::Encodable2718;
use alloy_json_rpc::{RpcError, RpcParam, RpcReturn};
use alloy_network::{Ethereum, Network};
use alloy_primitives::{
    hex, Address, BlockHash, BlockNumber, Bytes, StorageKey, StorageValue, TxHash, B256, U128,
    U256, U64,
};
use alloy_rpc_client::{
    BuiltInConnectionString, ClientBuilder, ClientRef, PollerBuilder, RpcClient, WeakClient,
};
use alloy_rpc_types::{
    state::StateOverride, AccessListWithGasUsed, Block, BlockId, BlockNumberOrTag,
    EIP1186AccountProofResponse, FeeHistory, Filter, FilterChanges, Log, SyncStatus,
};
use alloy_rpc_types_trace::parity::{LocalizedTransactionTrace, TraceResults, TraceType};
use alloy_transport::{
    BoxTransport, BoxTransportConnect, Transport, TransportError, TransportErrorKind,
    TransportResult,
};
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    fmt,
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

#[cfg(feature = "reqwest")]
use alloy_transport_http::Http;

#[cfg(feature = "pubsub")]
use alloy_pubsub::{PubSubFrontend, Subscription};

/// A task that polls the provider with `eth_getFilterChanges`, returning a list of `R`.
///
/// See [`PollerBuilder`] for more details.
pub type FilterPollerBuilder<T, R> = PollerBuilder<T, (U256,), Vec<R>>;

/// A transaction that can be sent. This is either a builder or an envelope.
///
/// This type is used to allow for fillers to convert a builder into an envelope
/// without changing the user-facing API.
///
/// Users should NOT use this type directly. It should only be used as an
/// implementation detail of [`Provider::send_transaction_internal`].
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendableTx<N: Network> {
    /// A transaction that is not yet signed.
    Builder(N::TransactionRequest),
    /// A transaction that is signed and fully constructed.
    Envelope(N::TxEnvelope),
}

impl<N: Network> SendableTx<N> {
    /// Fallible cast to an unbuilt transaction request.
    pub fn as_mut_builder(&mut self) -> Option<&mut N::TransactionRequest> {
        match self {
            Self::Builder(tx) => Some(tx),
            _ => None,
        }
    }

    /// Fallible cast to an unbuilt transaction request.
    pub const fn as_builder(&self) -> Option<&N::TransactionRequest> {
        match self {
            Self::Builder(tx) => Some(tx),
            _ => None,
        }
    }

    /// Checks if the transaction is a builder.
    pub const fn is_builder(&self) -> bool {
        matches!(self, Self::Builder(_))
    }

    /// Check if the transaction is an envelope.
    pub const fn is_envelope(&self) -> bool {
        matches!(self, Self::Envelope(_))
    }

    /// Fallible cast to a built transaction envelope.
    pub const fn as_envelope(&self) -> Option<&N::TxEnvelope> {
        match self {
            Self::Envelope(tx) => Some(tx),
            _ => None,
        }
    }
}

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<T, N = Ethereum> {
    /// The inner state of the root provider.
    pub(crate) inner: Arc<RootProviderInner<T, N>>,
}

impl<T, N> Clone for RootProvider<T, N> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: fmt::Debug, N> fmt::Debug for RootProvider<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootProvider").field("client", &self.inner.client).finish_non_exhaustive()
    }
}

#[cfg(feature = "reqwest")]
impl<N: Network> RootProvider<Http<reqwest::Client>, N> {
    /// Creates a new HTTP root provider from the given URL.
    pub fn new_http(url: reqwest::Url) -> Self {
        Self::new(RpcClient::new_http(url))
    }
}

impl<T: Transport, N: Network> RootProvider<T, N> {
    /// Creates a new root provider from the given RPC client.
    pub fn new(client: RpcClient<T>) -> Self {
        Self { inner: Arc::new(RootProviderInner::new(client)) }
    }
}

impl<N: Network> RootProvider<BoxTransport, N> {
    /// Connects to a boxed transport with the given connector.
    pub async fn connect_boxed<C: BoxTransportConnect>(conn: C) -> Result<Self, TransportError> {
        let client = ClientBuilder::default().connect_boxed(conn).await?;
        Ok(Self::new(client))
    }

    /// Creates a new root provider from the provided connection details.
    pub async fn connect_builtin(s: &str) -> Result<Self, TransportError> {
        let conn: BuiltInConnectionString = s.parse()?;
        let client = ClientBuilder::default().connect_boxed(conn).await?;
        Ok(Self::new(client))
    }
}

impl<T: Transport + Clone, N: Network> RootProvider<T, N> {
    /// Boxes the inner client.
    ///
    /// This will create a new provider if this instance is not the only reference to the inner
    /// client.
    pub fn boxed(self) -> RootProvider<BoxTransport, N> {
        let inner = Arc::unwrap_or_clone(self.inner);
        RootProvider { inner: Arc::new(inner.boxed()) }
    }

    /// Gets the subscription corresponding to the given RPC subscription ID.
    #[cfg(feature = "pubsub")]
    pub async fn get_subscription<R: RpcReturn>(
        &self,
        id: U256,
    ) -> TransportResult<Subscription<R>> {
        self.pubsub_frontend()?.get_subscription(id).await.map(Subscription::from)
    }

    /// Unsubscribes from the subscription corresponding to the given RPC subscription ID.
    #[cfg(feature = "pubsub")]
    pub fn unsubscribe(&self, id: U256) -> TransportResult<()> {
        self.pubsub_frontend()?.unsubscribe(id)
    }

    #[cfg(feature = "pubsub")]
    fn pubsub_frontend(&self) -> TransportResult<&PubSubFrontend> {
        let t = self.transport() as &dyn std::any::Any;
        t.downcast_ref::<PubSubFrontend>()
            .or_else(|| {
                t.downcast_ref::<BoxTransport>()
                    .and_then(|t| t.as_any().downcast_ref::<PubSubFrontend>())
            })
            .ok_or_else(TransportErrorKind::pubsub_unavailable)
    }

    #[cfg(feature = "pubsub")]
    fn transport(&self) -> &T {
        self.inner.client.transport()
    }

    #[inline]
    fn get_heart(&self) -> &HeartbeatHandle {
        self.inner.heart.get_or_init(|| {
            let poller = ChainStreamPoller::from_root(self);
            // TODO: Can we avoid `Box::pin` here?
            Heartbeat::new(Box::pin(poller.into_stream())).spawn()
        })
    }
}

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub(crate) struct RootProviderInner<T, N = Ethereum> {
    client: RpcClient<T>,
    heart: OnceLock<HeartbeatHandle>,
    _network: PhantomData<N>,
}

impl<T, N> Clone for RootProviderInner<T, N> {
    fn clone(&self) -> Self {
        Self { client: self.client.clone(), heart: self.heart.clone(), _network: PhantomData }
    }
}

impl<T, N> RootProviderInner<T, N> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { client, heart: OnceLock::new(), _network: PhantomData }
    }

    pub(crate) fn weak_client(&self) -> WeakClient<T> {
        self.client.get_weak()
    }

    pub(crate) fn client_ref(&self) -> ClientRef<'_, T> {
        self.client.get_ref()
    }
}

impl<T: Transport + Clone, N> RootProviderInner<T, N> {
    fn boxed(self) -> RootProviderInner<BoxTransport, N> {
        RootProviderInner { client: self.client.boxed(), heart: self.heart, _network: PhantomData }
    }
}

// todo: adjust docs
// todo: reorder
/// Provider is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Provider<Http, N>`.
///
/// # Subscriptions
///
/// **IMPORTANT:** this is currently only available when `T` is `PubSubFrontend` or `BoxedClient`
/// over `PubSubFrontend` due to an internal limitation. This means that layering transports will
/// always disable subscription support. See [issue #296](https://github.com/alloy-rs/alloy/issues/296).
///
/// The provider supports `pubsub` subscriptions to new block headers and pending transactions. This
/// is only available on `pubsub` clients, such as Websockets or IPC.
///
/// For a polling alternatives available over HTTP, use the `watch_*` methods. However, be aware
/// that polling increases RPC usage drastically.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[auto_impl::auto_impl(&, &mut, Rc, Arc, Box)]
pub trait Provider<T: Transport + Clone = BoxTransport, N: Network = Ethereum>:
    Send + Sync
{
    /// Returns the root provider.
    fn root(&self) -> &RootProvider<T, N>;

    /// Returns the RPC client used to send requests.
    ///
    /// NOTE: this method should not be overridden.
    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.root().client()
    }

    /// Returns a [`Weak`](std::sync::Weak) RPC client used to send requests.
    ///
    /// NOTE: this method should not be overridden.
    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.root().weak_client()
    }

    /// Watch for the confirmation of a single pending transaction with the given configuration.
    ///
    /// Note that this is handled internally rather than calling any specific RPC method, and as
    /// such should not be overridden.
    #[inline]
    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> TransportResult<PendingTransaction> {
        self.root().watch_pending_transaction(config).await
    }

    /// Subscribe to a stream of new block headers.
    ///
    /// # Errors
    ///
    /// This method is only available on `pubsub` clients, such as Websockets or IPC, and will
    /// return a [`PubsubUnavailable`](TransportErrorKind::PubsubUnavailable) transport error if the
    /// client does not support it.
    ///
    /// For a polling alternative available over HTTP, use [`Provider::watch_blocks`].
    /// However, be aware that polling increases RPC usage drastically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let sub = provider.subscribe_blocks().await?;
    /// let mut stream = sub.into_stream().take(5);
    /// while let Some(block) = stream.next().await {
    ///    println!("new block: {block:#?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "pubsub")]
    async fn subscribe_blocks(&self) -> TransportResult<Subscription<Block>> {
        self.root().pubsub_frontend()?;
        let id = self.client().request("eth_subscribe", ("newHeads",)).await?;
        self.root().get_subscription(id).await
    }

    /// Subscribe to a stream of pending transaction hashes.
    ///
    /// # Errors
    ///
    /// This method is only available on `pubsub` clients, such as Websockets or IPC, and will
    /// return a [`PubsubUnavailable`](TransportErrorKind::PubsubUnavailable) transport error if the
    /// client does not support it.
    ///
    /// For a polling alternative available over HTTP, use [`Provider::watch_pending_transactions`].
    /// However, be aware that polling increases RPC usage drastically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let sub = provider.subscribe_pending_transactions().await?;
    /// let mut stream = sub.into_stream().take(5);
    /// while let Some(tx_hash) = stream.next().await {
    ///    println!("new pending transaction hash: {tx_hash}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "pubsub")]
    async fn subscribe_pending_transactions(&self) -> TransportResult<Subscription<B256>> {
        self.root().pubsub_frontend()?;
        let id = self.client().request("eth_subscribe", ("newPendingTransactions",)).await?;
        self.root().get_subscription(id).await
    }

    /// Subscribe to a stream of pending transaction bodies.
    ///
    /// # Support
    ///
    /// This endpoint is compatible only with Geth client version 1.11.0 or later.
    ///
    /// # Errors
    ///
    /// This method is only available on `pubsub` clients, such as Websockets or IPC, and will
    /// return a [`PubsubUnavailable`](TransportErrorKind::PubsubUnavailable) transport error if the
    /// client does not support it.
    ///
    /// For a polling alternative available over HTTP, use
    /// [`Provider::watch_full_pending_transactions`]. However, be aware that polling increases
    /// RPC usage drastically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let sub = provider.subscribe_full_pending_transactions().await?;
    /// let mut stream = sub.into_stream().take(5);
    /// while let Some(tx) = stream.next().await {
    ///    println!("{tx:#?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "pubsub")]
    async fn subscribe_full_pending_transactions(
        &self,
    ) -> TransportResult<Subscription<N::TransactionResponse>> {
        self.root().pubsub_frontend()?;
        let id = self.client().request("eth_subscribe", ("newPendingTransactions", true)).await?;
        self.root().get_subscription(id).await
    }

    /// Subscribe to a stream of logs matching given filter.
    ///
    /// # Errors
    ///
    /// This method is only available on `pubsub` clients, such as Websockets or IPC, and will
    /// return a [`PubsubUnavailable`](TransportErrorKind::PubsubUnavailable) transport error if the
    /// client does not support it.
    ///
    /// For a polling alternative available over HTTP, use
    /// [`Provider::watch_logs`]. However, be aware that polling increases
    /// RPC usage drastically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    /// use alloy_primitives::keccak256;
    /// use alloy_rpc_types::Filter;
    ///
    /// let signature = keccak256("Transfer(address,address,uint256)".as_bytes());
    ///
    /// let sub = provider.subscribe_logs(&Filter::new().event_signature(signature)).await?;
    /// let mut stream = sub.into_stream().take(5);
    /// while let Some(tx) = stream.next().await {
    ///    println!("{tx:#?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "pubsub")]
    async fn subscribe_logs(&self, filter: &Filter) -> TransportResult<Subscription<Log>> {
        self.root().pubsub_frontend()?;
        let id = self.client().request("eth_subscribe", ("logs", filter)).await?;
        self.root().get_subscription(id).await
    }

    /// Subscribe to an RPC event.
    #[cfg(feature = "pubsub")]
    #[auto_impl(keep_default_for(&, &mut, Rc, Arc, Box))]
    async fn subscribe<P, R>(&self, params: P) -> TransportResult<Subscription<R>>
    where
        P: RpcParam,
        R: RpcReturn,
        Self: Sized,
    {
        self.root().pubsub_frontend()?;
        let id = self.client().request("eth_subscribe", params).await?;
        self.root().get_subscription(id).await
    }

    /// Cancels a subscription given the subscription ID.
    #[cfg(feature = "pubsub")]
    async fn unsubscribe(&self, id: U256) -> TransportResult<()> {
        self.root().unsubscribe(id)
    }

    /// Watch for new blocks by polling the provider with
    /// [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// Returns a builder that is used to configure the poller. See [`PollerBuilder`] for more
    /// details.
    ///
    /// # Examples
    ///
    /// Get the next 5 blocks:
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let poller = provider.watch_blocks().await?;
    /// let mut stream = poller.into_stream().flat_map(futures::stream::iter).take(5);
    /// while let Some(block_hash) = stream.next().await {
    ///    println!("new block: {block_hash}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn watch_blocks(&self) -> TransportResult<FilterPollerBuilder<T, B256>> {
        let id = self.new_block_filter().await?;
        Ok(PollerBuilder::new(self.weak_client(), "eth_getFilterChanges", (id,)))
    }

    /// Watch for new pending transaction by polling the provider with
    /// [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// Returns a builder that is used to configure the poller. See [`PollerBuilder`] for more
    /// details.
    ///
    /// # Examples
    ///
    /// Get the next 5 pending transaction hashes:
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let poller = provider.watch_pending_transactions().await?;
    /// let mut stream = poller.into_stream().flat_map(futures::stream::iter).take(5);
    /// while let Some(tx_hash) = stream.next().await {
    ///    println!("new pending transaction hash: {tx_hash}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn watch_pending_transactions(&self) -> TransportResult<FilterPollerBuilder<T, B256>> {
        let id = self.new_pending_transactions_filter(false).await?;
        Ok(PollerBuilder::new(self.weak_client(), "eth_getFilterChanges", (id,)))
    }

    /// Watch for new pending transaction bodies by polling the provider with
    /// [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// Returns a builder that is used to configure the poller. See [`PollerBuilder`] for more
    /// details.
    ///
    /// # Support
    ///
    /// This endpoint might not be supported by all clients.
    ///
    /// # Examples
    ///
    /// Get the next 5 pending transaction bodies:
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let poller = provider.watch_full_pending_transactions().await?;
    /// let mut stream = poller.into_stream().flat_map(futures::stream::iter).take(5);
    /// while let Some(tx) = stream.next().await {
    ///    println!("new pending transaction: {tx:#?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn watch_full_pending_transactions(
        &self,
    ) -> TransportResult<FilterPollerBuilder<T, N::TransactionResponse>> {
        let id = self.new_pending_transactions_filter(true).await?;
        Ok(PollerBuilder::new(self.weak_client(), "eth_getFilterChanges", (id,)))
    }

    /// Watch for new logs using the given filter by polling the provider with
    /// [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// Returns a builder that is used to configure the poller. See [`PollerBuilder`] for more
    /// details.
    ///
    /// # Examples
    ///
    /// Get the next 5 USDC transfer logs:
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_primitives::{address, b256};
    /// use alloy_rpc_types::Filter;
    /// use futures::StreamExt;
    ///
    /// let address = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    /// let transfer_signature = b256!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    /// let filter = Filter::new().address(address).event_signature(transfer_signature);
    ///
    /// let poller = provider.watch_logs(&filter).await?;
    /// let mut stream = poller.into_stream().flat_map(futures::stream::iter).take(5);
    /// while let Some(log) = stream.next().await {
    ///    println!("new log: {log:#?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn watch_logs(&self, filter: &Filter) -> TransportResult<FilterPollerBuilder<T, Log>> {
        let id = self.new_filter(filter).await?;
        Ok(PollerBuilder::new(self.weak_client(), "eth_getFilterChanges", (id,)))
    }

    /// Notify the provider that we are interested in new blocks.
    ///
    /// Returns the ID to use with [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// See also [`watch_blocks`](Self::watch_blocks) to configure a poller.
    async fn new_block_filter(&self) -> TransportResult<U256> {
        self.client().request("eth_newBlockFilter", ()).await
    }

    /// Notify the provider that we are interested in new pending transactions.
    ///
    /// If `full` is `true`, the stream will consist of full transaction bodies instead of just the
    /// hashes. This not supported by all clients.
    ///
    /// Returns the ID to use with [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// See also [`watch_pending_transactions`](Self::watch_pending_transactions) to configure a
    /// poller.
    async fn new_pending_transactions_filter(&self, full: bool) -> TransportResult<U256> {
        // NOTE: We don't want to send `false` as the client might not support it.
        let param = if full { &[true][..] } else { &[] };
        self.client().request("eth_newPendingTransactionFilter", param).await
    }

    /// Notify the provider that we are interested in logs that match the given filter.
    ///
    /// Returns the ID to use with [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// See also [`watch_logs`](Self::watch_logs) to configure a poller.
    async fn new_filter(&self, filter: &Filter) -> TransportResult<U256> {
        self.client().request("eth_newFilter", (filter,)).await
    }

    /// Get a list of values that have been added since the last poll.
    ///
    /// The return value depends on what stream `id` corresponds to.
    /// See [`FilterChanges`] for all possible return values.
    #[auto_impl(keep_default_for(&, &mut, Rc, Arc, Box))]
    async fn get_filter_changes<R: RpcReturn>(&self, id: U256) -> TransportResult<Vec<R>>
    where
        Self: Sized,
    {
        self.client().request("eth_getFilterChanges", (id,)).await
    }

    /// Get a list of values that have been added since the last poll.
    ///
    /// This returns an enum over all possible return values. You probably want to use
    /// [`get_filter_changes`](Self::get_filter_changes) instead.
    async fn get_filter_changes_dyn(&self, id: U256) -> TransportResult<FilterChanges> {
        self.client().request("eth_getFilterChanges", (id,)).await
    }

    /// Get the last block number available.
    async fn get_block_number(&self) -> TransportResult<BlockNumber> {
        self.client().request("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    /// Gets the transaction count (AKA "nonce") of the corresponding address.
    #[doc(alias = "get_nonce")]
    #[doc(alias = "get_account_nonce")]
    async fn get_transaction_count(&self, address: Address, tag: BlockId) -> TransportResult<u64> {
        self.client()
            .request("eth_getTransactionCount", (address, tag))
            .await
            .map(|count: U64| count.to::<u64>())
    }

    /// Get a block by its number.
    // TODO: Network associate
    async fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        hydrate: bool,
    ) -> TransportResult<Option<Block>> {
        self.client().request("eth_getBlockByNumber", (number, hydrate)).await
    }

    /// Broadcasts a transaction to the network.
    ///
    /// Returns a type that can be used to configure how and when to await the
    /// transaction's confirmation.
    ///
    /// # Examples
    ///
    /// See [`PendingTransactionBuilder`](crate::PendingTransactionBuilder) for more examples.
    ///
    /// ```no_run
    /// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider, tx: alloy_rpc_types::transaction::TransactionRequest) -> Result<(), Box<dyn std::error::Error>> {
    /// let tx_hash = provider.send_transaction(tx)
    ///     .await?
    ///     .with_required_confirmations(2)
    ///     .with_timeout(Some(std::time::Duration::from_secs(60)))
    ///     .watch()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        self.send_transaction_internal(SendableTx::Builder(tx)).await
    }

    ///
    /// This method allows [`ProviderLayer`] and [`TxFiller`] to bulid the
    /// transaction and send it to the network without changing user-facing
    /// APIs. Generally implementors should NOT override this method.
    ///
    /// [`send_transaction`]: Self::send_transaction
    /// [`ProviderLayer`]: crate::ProviderLayer
    /// [`TxFiller`]: crate::TxFiller
    #[doc(hidden)]
    async fn send_transaction_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        match tx {
            SendableTx::Builder(mut tx) => {
                alloy_network::TransactionBuilder::prep_for_submission(&mut tx);
                let tx_hash = self.client().request("eth_sendTransaction", (tx,)).await?;
                Ok(PendingTransactionBuilder::new(self.root(), tx_hash))
            }
            SendableTx::Envelope(tx) => {
                let mut encoded_tx = vec![];
                tx.encode_2718(&mut encoded_tx);
                self.send_raw_transaction(&encoded_tx).await
            }
        }
    }

    /// Broadcasts a raw transaction RLP bytes to the network.
    ///
    /// See [`send_transaction`](Self::send_transaction) for more details.
    async fn send_raw_transaction(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        let rlp_hex = hex::encode_prefixed(encoded_tx);
        let tx_hash = self.client().request("eth_sendRawTransaction", (rlp_hex,)).await?;
        Ok(PendingTransactionBuilder::new(self.root(), tx_hash))
    }

    /// Gets the balance of the account at the specified tag, which defaults to latest.
    async fn get_balance(&self, address: Address, tag: BlockId) -> TransportResult<U256> {
        self.client().request("eth_getBalance", (address, tag)).await
    }

    /// Gets a block by either its hash, tag, or number, with full transactions or only hashes.
    async fn get_block(&self, id: BlockId, full: bool) -> TransportResult<Option<Block>> {
        match id {
            BlockId::Hash(hash) => self.get_block_by_hash(hash.into(), full).await,
            BlockId::Number(number) => self.get_block_by_number(number, full).await,
        }
    }

    /// Gets a block by its [BlockHash], with full transactions or only hashes.
    async fn get_block_by_hash(
        &self,
        hash: BlockHash,
        full: bool,
    ) -> TransportResult<Option<Block>> {
        self.client().request("eth_getBlockByHash", (hash, full)).await
    }

    /// Gets the client version of the chain client().
    async fn get_client_version(&self) -> TransportResult<String> {
        self.client().request("web3_clientVersion", ()).await
    }

    /// Gets the chain ID.
    async fn get_chain_id(&self) -> TransportResult<u64> {
        self.client().request("eth_chainId", ()).await.map(|id: U64| id.to::<u64>())
    }

    /// Gets the network ID. Same as `eth_chainId`.
    async fn get_net_version(&self) -> TransportResult<u64> {
        self.client().request("net_version", ()).await.map(|id: U64| id.to::<u64>())
    }

    /// Gets the specified storage value from [Address].
    async fn get_storage_at(
        &self,
        address: Address,
        key: U256,
        tag: BlockId,
    ) -> TransportResult<StorageValue> {
        self.client().request("eth_getStorageAt", (address, key, tag)).await
    }

    /// Gets the bytecode located at the corresponding [Address].
    async fn get_code_at(&self, address: Address, tag: BlockId) -> TransportResult<Bytes> {
        self.client().request("eth_getCode", (address, tag)).await
    }

    /// Gets a transaction by its [TxHash].
    async fn get_transaction_by_hash(
        &self,
        hash: TxHash,
    ) -> TransportResult<N::TransactionResponse> {
        self.client().request("eth_getTransactionByHash", (hash,)).await
    }

    /// Retrieves a [`Vec<Log>`] with the given [Filter].
    async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
        self.client().request("eth_getLogs", (filter,)).await
    }

    /// Gets the accounts in the remote node. This is usually empty unless you're using a local
    /// node.
    async fn get_accounts(&self) -> TransportResult<Vec<Address>> {
        self.client().request("eth_accounts", ()).await
    }

    /// Gets the current gas price in wei.
    async fn get_gas_price(&self) -> TransportResult<u128> {
        self.client().request("eth_gasPrice", ()).await.map(|price: U128| price.to::<u128>())
    }

    /// Returns a suggestion for the current `maxPriorityFeePerGas` in wei.
    async fn get_max_priority_fee_per_gas(&self) -> TransportResult<u128> {
        self.client()
            .request("eth_maxPriorityFeePerGas", ())
            .await
            .map(|fee: U128| fee.to::<u128>())
    }

    /// Returns the base fee per blob gas (blob gas price) in wei.
    async fn get_blob_base_fee(&self) -> TransportResult<u128> {
        self.client().request("eth_blobBaseFee", ()).await
    }

    /// Gets a transaction receipt if it exists, by its [TxHash].
    async fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> TransportResult<Option<N::ReceiptResponse>> {
        self.client().request("eth_getTransactionReceipt", (hash,)).await
    }

    /// Gets the selected block [BlockNumberOrTag] receipts.
    async fn get_block_receipts(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Option<Vec<N::ReceiptResponse>>> {
        self.client().request("eth_getBlockReceipts", (block,)).await
    }

    /// Gets an uncle block through the tag [BlockId] and index [u64].
    async fn get_uncle(&self, tag: BlockId, idx: u64) -> TransportResult<Option<Block>> {
        let idx = U64::from(idx);
        match tag {
            BlockId::Hash(hash) => {
                self.client()
                    .request("eth_getUncleByBlockHashAndIndex", (hash.block_hash, idx))
                    .await
            }
            BlockId::Number(number) => {
                self.client().request("eth_getUncleByBlockNumberAndIndex", (number, idx)).await
            }
        }
    }

    /// Gets the number of uncles for the block specified by the tag [BlockId].
    async fn get_uncle_count(&self, tag: BlockId) -> TransportResult<u64> {
        match tag {
            BlockId::Hash(hash) => self
                .client()
                .request("eth_getUncleCountByBlockHash", (hash.block_hash,))
                .await
                .map(|count: U64| count.to::<u64>()),
            BlockId::Number(number) => self
                .client()
                .request("eth_getUncleCountByBlockNumber", (number,))
                .await
                .map(|count: U64| count.to::<u64>()),
        }
    }

    /// Gets syncing info.
    async fn syncing(&self) -> TransportResult<SyncStatus> {
        self.client().request("eth_syncing", ()).await
    }

    /// Execute a smart contract call with a transaction request, without publishing a transaction.
    async fn call(&self, tx: &N::TransactionRequest, block: BlockId) -> TransportResult<Bytes> {
        self.client().request("eth_call", (tx, block)).await
    }

    /// Execute a smart contract call with a transaction request and state overrides, without
    /// publishing a transaction.
    ///
    /// # Note
    ///
    /// Not all client implementations support state overrides.
    async fn call_with_overrides(
        &self,
        tx: &N::TransactionRequest,
        block: BlockId,
        state: StateOverride,
    ) -> TransportResult<Bytes> {
        self.client().request("eth_call", (tx, block, state)).await
    }

    /// Returns a collection of historical gas information [FeeHistory] which
    /// can be used to calculate the EIP1559 fields `maxFeePerGas` and `maxPriorityFeePerGas`.
    /// `block_count` can range from 1 to 1024 blocks in a single request.
    async fn get_fee_history(
        &self,
        block_count: u64,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory> {
        self.client().request("eth_feeHistory", (block_count, last_block, reward_percentiles)).await
    }

    /// Estimate the gas needed for a transaction.
    async fn estimate_gas(
        &self,
        tx: &N::TransactionRequest,
        block: BlockId,
    ) -> TransportResult<u128> {
        self.client()
            .request("eth_estimateGas", (tx, block))
            .await
            .map(|gas: U128| gas.to::<u128>())
    }

    /// Estimates the EIP1559 `maxFeePerGas` and `maxPriorityFeePerGas` fields.
    ///
    /// Receives an optional [EstimatorFunction] that can be used to modify
    /// how to estimate these fees.
    async fn estimate_eip1559_fees(
        &self,
        estimator: Option<EstimatorFunction>,
    ) -> TransportResult<Eip1559Estimation> {
        let fee_history = self
            .get_fee_history(
                utils::EIP1559_FEE_ESTIMATION_PAST_BLOCKS,
                BlockNumberOrTag::Latest,
                &[utils::EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await?;

        // if the base fee of the Latest block is 0 then we need check if the latest block even has
        // a base fee/supports EIP1559
        let base_fee_per_gas = match fee_history.latest_block_base_fee() {
            Some(base_fee) if (base_fee != 0) => base_fee,
            _ => {
                // empty response, fetch basefee from latest block directly
                self.get_block_by_number(BlockNumberOrTag::Latest, false)
                    .await?
                    .ok_or(RpcError::NullResp)?
                    .header
                    .base_fee_per_gas
                    .ok_or(RpcError::UnsupportedFeature("eip1559"))?
            }
        };

        Ok(estimator.unwrap_or(utils::eip1559_default_estimator)(
            base_fee_per_gas,
            &fee_history.reward.unwrap_or_default(),
        ))
    }

    /// Get the account and storage values of the specified account including the merkle proofs.
    ///
    /// This call can be used to verify that the data has not been tampered with.
    async fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
        block: BlockId,
    ) -> TransportResult<EIP1186AccountProofResponse> {
        self.client().request("eth_getProof", (address, keys, block)).await
    }

    /// Create an [EIP-2930] access list.
    ///
    /// [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    async fn create_access_list(
        &self,
        request: &N::TransactionRequest,
        block: BlockId,
    ) -> TransportResult<AccessListWithGasUsed> {
        self.client().request("eth_createAccessList", (request, block)).await
    }

    /// Executes the given transaction and returns a number of possible traces.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn trace_call(
        &self,
        request: &N::TransactionRequest,
        trace_type: &[TraceType],
        block: BlockId,
    ) -> TransportResult<TraceResults> {
        self.client().request("trace_call", (request, trace_type, block)).await
    }

    /// Traces multiple transactions on top of the same block, i.e. transaction `n` will be executed
    /// on top of the given block with all `n - 1` transaction applied first.
    ///
    /// Allows tracing dependent transactions.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn trace_call_many(
        &self,
        request: &[(N::TransactionRequest, Vec<TraceType>)],
        block: BlockId,
    ) -> TransportResult<TraceResults> {
        self.client().request("trace_callMany", (request, block)).await
    }

    // todo: move to extension trait
    /// Parity trace transaction.
    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_transaction", (hash,)).await
    }

    // todo: move to extension trait
    /// Trace all transactions in the given block.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_block", (block,)).await
    }

    /* ------------------------------------------ anvil ----------------------------------------- */

    /// Set the bytecode of a given account.
    async fn set_code(&self, address: Address, code: &str) -> TransportResult<()> {
        self.client().request("anvil_setCode", (address, code)).await
    }

    /* ---------------------------------------- raw calls --------------------------------------- */

    /// Sends a raw JSON-RPC request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_rpc_types::BlockNumberOrTag;
    ///
    /// // No parameters: `()`
    /// let block_number = provider.raw_request("eth_blockNumber".into(), ()).await?;
    ///
    /// // One parameter: `(param,)` or `[param]`
    /// let block = provider.raw_request("eth_getBlockByNumber".into(), (BlockNumberOrTag::Latest,)).await?;
    ///
    /// // Two or more parameters: `(param1, param2, ...)` or `[param1, param2, ...]`
    /// let full_block = provider.raw_request("eth_getBlockByNumber".into(), (BlockNumberOrTag::Latest, true)).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn raw_request<P, R>(&self, method: Cow<'static, str>, params: P) -> TransportResult<R>
    where
        P: RpcParam,
        R: RpcReturn,
        Self: Sized,
    {
        self.client().request(method, &params).await
    }

    /// Sends a raw JSON-RPC request with type-erased parameters and return.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(provider: impl alloy_provider::Provider) -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_rpc_types::BlockNumberOrTag;
    ///
    /// // No parameters: `()`
    /// let params = serde_json::value::to_raw_value(&())?;
    /// let block_number = provider.raw_request_dyn("eth_blockNumber".into(), &params).await?;
    ///
    /// // One parameter: `(param,)` or `[param]`
    /// let params = serde_json::value::to_raw_value(&(BlockNumberOrTag::Latest,))?;
    /// let block = provider.raw_request_dyn("eth_getBlockByNumber".into(), &params).await?;
    ///
    /// // Two or more parameters: `(param1, param2, ...)` or `[param1, param2, ...]`
    /// let params = serde_json::value::to_raw_value(&(BlockNumberOrTag::Latest, true))?;
    /// let full_block = provider.raw_request_dyn("eth_getBlockByNumber".into(), &params).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn raw_request_dyn(
        &self,
        method: Cow<'static, str>,
        params: &RawValue,
    ) -> TransportResult<Box<RawValue>> {
        self.client().request(method, params).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<T: Transport + Clone, N: Network> Provider<T, N> for RootProvider<T, N> {
    #[inline]
    fn root(&self) -> &RootProvider<T, N> {
        self
    }

    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.inner.client_ref()
    }

    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.inner.weak_client()
    }

    #[inline]
    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> TransportResult<PendingTransaction> {
        self.get_heart().watch_tx(config).await.map_err(|_| TransportErrorKind::backend_gone())
    }
}

#[cfg(test)]
#[allow(clippy::missing_const_for_fn)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, bytes, TxKind};
    use alloy_rpc_types::request::TransactionRequest;

    extern crate self as alloy_provider;

    // NOTE: We cannot import the test-utils crate here due to a circular dependency.
    include!("../../internal-test-utils/src/providers.rs");

    #[cfg(feature = "reqwest")]
    #[tokio::test]
    async fn object_safety() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        // These blocks are not necessary.
        {
            let refdyn = &provider as &dyn Provider<Http<reqwest::Client>, _>;
            let num = refdyn.get_block_number().await.unwrap();
            assert_eq!(0, num);
        }

        // Clones the underlying provider too.
        {
            let clone_boxed = provider.clone().boxed();
            let num = clone_boxed.get_block_number().await.unwrap();
            assert_eq!(0, num);
        }

        // Note the `Http` arg, vs no arg (defaulting to `BoxedTransport`) below.
        {
            let refdyn = &provider as &dyn Provider<Http<reqwest::Client>, _>;
            let num = refdyn.get_block_number().await.unwrap();
            assert_eq!(0, num);
        }

        let boxed = provider.boxed();
        let num = boxed.get_block_number().await.unwrap();
        assert_eq!(0, num);

        let boxed_boxdyn = Box::new(boxed) as Box<dyn Provider<_>>;
        let num = boxed_boxdyn.get_block_number().await.unwrap();
        assert_eq!(0, num);
    }

    #[cfg(feature = "ws")]
    #[tokio::test]
    async fn subscribe_blocks_http() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil_with(|a| a.block_time(1));

        let err = provider.subscribe_blocks().await.unwrap_err();
        let alloy_json_rpc::RpcError::Transport(TransportErrorKind::PubsubUnavailable) = err else {
            panic!("{err:?}");
        };
    }

    #[cfg(feature = "ws")]
    #[tokio::test]
    async fn subscribe_blocks_ws() {
        use futures::stream::StreamExt;

        init_tracing();
        let anvil = Anvil::new().block_time(1).spawn();
        let ws = alloy_rpc_client::WsConnect::new(anvil.ws_endpoint());
        let client = RpcClient::connect_pubsub(ws).await.unwrap();
        let provider = RootProvider::<_, Ethereum>::new(client);

        let sub = provider.subscribe_blocks().await.unwrap();
        let mut stream = sub.into_stream().take(2);
        let mut n = 1;
        while let Some(block) = stream.next().await {
            assert_eq!(block.header.number.unwrap(), n);
            assert_eq!(block.transactions.hashes().len(), 0);
            n += 1;
        }
    }

    #[cfg(feature = "ws")]
    #[tokio::test]
    async fn subscribe_blocks_ws_boxed() {
        use futures::stream::StreamExt;

        init_tracing();
        let anvil = Anvil::new().block_time(1).spawn();
        let ws = alloy_rpc_client::WsConnect::new(anvil.ws_endpoint());
        let client = RpcClient::connect_pubsub(ws).await.unwrap();
        let provider = RootProvider::<_, Ethereum>::new(client);
        let provider = provider.boxed();

        let sub = provider.subscribe_blocks().await.unwrap();
        let mut stream = sub.into_stream().take(2);
        let mut n = 1;
        while let Some(block) = stream.next().await {
            assert_eq!(block.header.number.unwrap(), n);
            assert_eq!(block.transactions.hashes().len(), 0);
            n += 1;
        }
    }

    #[tokio::test]
    #[cfg(feature = "ws")]
    async fn subscribe_blocks_ws_remote() {
        use futures::stream::StreamExt;

        init_tracing();
        let url = "wss://eth-mainnet.g.alchemy.com/v2/viFmeVzhg6bWKVMIWWS8MhmzREB-D4f7";
        let ws = alloy_rpc_client::WsConnect::new(url);
        let Ok(client) = RpcClient::connect_pubsub(ws).await else { return };
        let provider = RootProvider::<_, Ethereum>::new(client);
        let sub = provider.subscribe_blocks().await.unwrap();
        let mut stream = sub.into_stream().take(1);
        while let Some(block) = stream.next().await {
            println!("New block {:?}", block);
            assert!(block.header.number.unwrap() > 0);
        }
    }

    #[tokio::test]
    async fn test_send_tx() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: Some(TxKind::Call(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        };

        let builder = provider.send_transaction(tx.clone()).await.expect("failed to send tx");
        let hash1 = *builder.tx_hash();
        let hash2 = builder.watch().await.expect("failed to await pending tx");
        assert_eq!(hash1, hash2);

        let builder = provider.send_transaction(tx).await.expect("failed to send tx");
        let hash1 = *builder.tx_hash();
        let hash2 =
            builder.get_receipt().await.expect("failed to await pending tx").transaction_hash;
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn gets_block_number() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = provider.get_block_number().await.unwrap();
        assert_eq!(0, num)
    }

    #[tokio::test]
    async fn gets_block_number_with_raw_req() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num: U64 = provider.raw_request("eth_blockNumber".into(), ()).await.unwrap();
        assert_eq!(0, num.to::<u64>())
    }

    #[tokio::test]
    async fn gets_transaction_count() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let count = provider
            .get_transaction_count(
                address!("328375e18E7db8F1CA9d9bA8bF3E9C94ee34136A"),
                BlockNumberOrTag::Latest.into(),
            )
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn gets_block_by_hash() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        let hash = block.header.hash.unwrap();
        let block = provider.get_block_by_hash(hash, true).await.unwrap().unwrap();
        assert_eq!(block.header.hash.unwrap(), hash);
    }

    #[tokio::test]
    async fn gets_block_by_hash_with_raw_req() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        let hash = block.header.hash.unwrap();
        let block: Block = provider
            .raw_request::<(B256, bool), Block>("eth_getBlockByHash".into(), (hash, true))
            .await
            .unwrap();
        assert_eq!(block.header.hash.unwrap(), hash);
    }

    #[tokio::test]
    async fn gets_block_by_number_full() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), num);
    }

    #[tokio::test]
    async fn gets_block_by_number() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), num);
    }

    #[tokio::test]
    async fn gets_client_version() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let version = provider.get_client_version().await.unwrap();
        assert!(version.contains("anvil"));
    }

    #[tokio::test]
    async fn gets_chain_id() {
        let dev_chain_id: u64 = 13371337;
        let anvil = Anvil::new().args(["--chain-id", dev_chain_id.to_string().as_str()]).spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        let provider = RootProvider::<_, Ethereum>::new(RpcClient::new(http, true));

        let chain_id = provider.get_chain_id().await.unwrap();
        assert_eq!(chain_id, dev_chain_id);
    }

    #[tokio::test]
    async fn gets_network_id() {
        let dev_chain_id: u64 = 13371337;
        let anvil = Anvil::new().args(["--chain-id", dev_chain_id.to_string().as_str()]).spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        let provider = RootProvider::<_, Ethereum>::new(RpcClient::new(http, true));

        let chain_id = provider.get_net_version().await.unwrap();
        assert_eq!(chain_id, dev_chain_id);
    }

    #[tokio::test]
    #[cfg(feature = "anvil")]
    async fn gets_code_at() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        // Set the code
        let addr = Address::with_last_byte(16);
        provider.set_code(addr, "0xbeef").await.unwrap();
        let _code = provider.get_code_at(addr, BlockId::default()).await.unwrap();
    }

    #[tokio::test]
    async fn gets_storage_at() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let addr = Address::with_last_byte(16);
        let storage = provider.get_storage_at(addr, U256::ZERO, BlockId::default()).await.unwrap();
        assert_eq!(storage, U256::ZERO);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_transaction_by_hash() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let tx = provider
            .get_transaction_by_hash(b256!(
                "5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95"
            ))
            .await
            .unwrap();
        assert_eq!(
            tx.block_hash.unwrap(),
            b256!("b20e6f35d4b46b3c4cd72152faec7143da851a0dc281d390bdd50f58bfbdb5d3")
        );
        assert_eq!(tx.block_number.unwrap(), 4571819);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_logs() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let filter = Filter::new()
            .at_block_hash(b256!(
                "b20e6f35d4b46b3c4cd72152faec7143da851a0dc281d390bdd50f58bfbdb5d3"
            ))
            .event_signature(b256!(
                "e1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"
            ));
        let logs = provider.get_logs(&filter).await.unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_tx_receipt() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let receipt = provider
            .get_transaction_receipt(b256!(
                "5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95"
            ))
            .await
            .unwrap();
        assert!(receipt.is_some());
        let receipt = receipt.unwrap();
        assert_eq!(
            receipt.transaction_hash,
            b256!("5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95")
        );
    }

    #[tokio::test]
    async fn gets_max_priority_fee_per_gas() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let _fee = provider.get_max_priority_fee_per_gas().await.unwrap();
    }

    #[tokio::test]
    async fn gets_fee_history() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let block_number = provider.get_block_number().await.unwrap();
        let fee_history = provider
            .get_fee_history(
                utils::EIP1559_FEE_ESTIMATION_PAST_BLOCKS,
                BlockNumberOrTag::Number(block_number),
                &[utils::EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await
            .unwrap();
        assert_eq!(fee_history.oldest_block, 0_u64);
    }

    #[tokio::test]
    async fn gets_block_receipts() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let receipts = provider.get_block_receipts(BlockNumberOrTag::Latest).await.unwrap();
        assert!(receipts.is_some());
    }

    #[tokio::test]
    async fn gets_block_traces() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let traces = provider.trace_block(BlockNumberOrTag::Latest).await.unwrap();
        assert_eq!(traces.len(), 0);
    }

    #[tokio::test]
    async fn sends_raw_transaction() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let pending = provider
            .send_raw_transaction(
                // Transfer 1 ETH from default EOA address to the Genesis address.
                bytes!("f865808477359400825208940000000000000000000000000000000000000000018082f4f5a00505e227c1c636c76fac55795db1a40a4d24840d81b40d2fe0cc85767f6bd202a01e91b437099a8a90234ac5af3cb7ca4fb1432e133f75f9a91678eaf5f487c74b").as_ref()
            )
            .await.unwrap();
        assert_eq!(
            pending.tx_hash().to_string(),
            "0x9dae5cf33694a02e8a7d5de3fe31e9d05ca0ba6e9180efac4ab20a06c9e598a3"
        );
    }

    #[tokio::test]
    async fn connect_boxed() {
        init_tracing();
        let (_provider, anvil) = spawn_anvil();

        let provider =
            RootProvider::<BoxTransport, Ethereum>::connect_builtin(anvil.endpoint().as_str())
                .await;

        match provider {
            Ok(provider) => {
                let num = provider.get_block_number().await.unwrap();
                assert_eq!(0, num);
            }
            Err(e) => {
                assert_eq!(
                    format!("{}",e),
                    "hyper not supported by BuiltinConnectionString. Please instantiate a hyper client manually"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_uncle_count() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let count = provider.get_uncle_count(BlockId::Number(0.into())).await.unwrap();
        assert_eq!(count, 0);
    }
}
