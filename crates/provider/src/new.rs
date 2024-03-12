use crate::{
    chain::ChainStreamPoller,
    heart::{Heartbeat, HeartbeatHandle, PendingTransaction, PendingTransactionConfig},
    utils::{self, EstimatorFunction},
};
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{
    hex, Address, BlockHash, BlockNumber, Bytes, StorageKey, StorageValue, TxHash, B256, U256, U64,
};
use alloy_rpc_client::{ClientRef, PollerBuilder, RpcClient, WeakClient};
use alloy_rpc_trace_types::{
    geth::{GethDebugTracingOptions, GethTrace},
    parity::LocalizedTransactionTrace,
};
use alloy_rpc_types::{
    state::StateOverride, AccessListWithGasUsed, Block, BlockId, BlockNumberOrTag,
    EIP1186AccountProofResponse, FeeHistory, Filter, FilterChanges, Log, SyncStatus,
};
use alloy_transport::{BoxTransport, Transport, TransportErrorKind, TransportResult};
use serde_json::value::RawValue;
use std::{
    fmt,
    marker::PhantomData,
    sync::{Arc, OnceLock, Weak},
};

/// A [`Provider`] in a [`Weak`] reference.
pub type WeakProvider<P> = Weak<P>;

/// A borrowed [`Provider`].
pub type ProviderRef<'a, P> = &'a P;

/// A task that polls the provider with `eth_getFilterChanges`, returning a list of `R`.
///
/// See [`PollerBuilder`] for more details.
pub type FilterPollerBuilder<T, R> = PollerBuilder<T, (U256,), Vec<R>>;

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<N, T> {
    /// The inner state of the root provider.
    pub(crate) inner: Arc<RootProviderInner<N, T>>,
}

impl<N, T> Clone for RootProvider<N, T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<N, T: fmt::Debug> fmt::Debug for RootProvider<N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootProvider").field("client", &self.inner.client).finish_non_exhaustive()
    }
}

impl<N: Network, T: Transport> RootProvider<N, T> {
    /// Creates a new root provider from the given RPC client.
    pub fn new(client: RpcClient<T>) -> Self {
        Self { inner: Arc::new(RootProviderInner::new(client)) }
    }
}

impl<N: Network, T: Transport + Clone> RootProvider<N, T> {
    /// Boxes the inner client.
    ///
    /// This will create a new provider if this instance is not the only reference to the inner
    /// client.
    pub fn boxed(self) -> RootProvider<N, BoxTransport> {
        let inner = Arc::unwrap_or_clone(self.inner);
        RootProvider { inner: Arc::new(inner.boxed()) }
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
pub(crate) struct RootProviderInner<N, T> {
    client: RpcClient<T>,
    heart: OnceLock<HeartbeatHandle>,
    _network: PhantomData<N>,
}

impl<N, T> Clone for RootProviderInner<N, T> {
    fn clone(&self) -> Self {
        Self { client: self.client.clone(), heart: self.heart.clone(), _network: PhantomData }
    }
}

impl<N, T> RootProviderInner<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { client, heart: OnceLock::new(), _network: PhantomData }
    }

    fn weak_client(&self) -> WeakClient<T> {
        self.client.get_weak()
    }

    fn client_ref(&self) -> ClientRef<'_, T> {
        self.client.get_ref()
    }
}

impl<N, T: Transport + Clone> RootProviderInner<N, T> {
    fn boxed(self) -> RootProviderInner<N, BoxTransport> {
        RootProviderInner { client: self.client.boxed(), heart: self.heart, _network: PhantomData }
    }
}

// todo: adjust docs
// todo: reorder
/// Provider is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Provider<N, Http>`.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[auto_impl::auto_impl(&, &mut, Rc, Arc, Box)]
pub trait Provider<N: Network, T: Transport + Clone = BoxTransport>: Send + Sync {
    /// Returns the RPC client used to send requests.
    fn client(&self) -> ClientRef<'_, T>;

    /// Returns a [`Weak`] RPC client used to send requests.
    fn weak_client(&self) -> WeakClient<T>;

    /// Watch for the confirmation of a single pending transaction with the given configuration.
    ///
    /// Note that this is handled internally rather than calling any specific RPC method.
    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> TransportResult<PendingTransaction>;

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
    /// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider<N>) -> Result<(), Box<dyn std::error::Error>> {
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
    /// Get the next 5 pending transactions:
    ///
    /// ```no_run
    /// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider<N>) -> Result<(), Box<dyn std::error::Error>> {
    /// use futures::StreamExt;
    ///
    /// let poller = provider.watch_pending_transactions().await?;
    /// let mut stream = poller.into_stream().flat_map(futures::stream::iter).take(5);
    /// while let Some(tx_hash) = stream.next().await {
    ///    println!("pending transaction: {tx_hash}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn watch_pending_transactions(&self) -> TransportResult<FilterPollerBuilder<T, B256>> {
        let id = self.new_pending_transactions_filter().await?;
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
    /// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider<N>) -> Result<(), Box<dyn std::error::Error>> {
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
    ///    println!("{log:#?}");
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
        self.client().prepare("eth_newBlockFilter", ()).await
    }

    /// Notify the provider that we are interested in new blocks.
    ///
    /// Returns the ID to use with [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// See also [`watch_pending_transactions`](Self::watch_pending_transactions) to configure a
    /// poller.
    async fn new_pending_transactions_filter(&self) -> TransportResult<U256> {
        self.client().prepare("eth_newPendingTransactionFilter", ()).await
    }

    /// Notify the provider that we are interested in logs that match the given filter.
    ///
    /// Returns the ID to use with [`eth_getFilterChanges`](Self::get_filter_changes).
    ///
    /// See also [`watch_logs`](Self::watch_logs) to configure a poller.
    async fn new_filter(&self, filter: &Filter) -> TransportResult<U256> {
        self.client().prepare("eth_newFilter", (filter,)).await
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
        self.client().prepare("eth_getFilterChanges", (id,)).await
    }

    /// Get a list of values that have been added since the last poll.
    ///
    /// This returns an enum over all possible return values. You probably want to use
    /// [`get_filter_changes`](Self::get_filter_changes) instead.
    async fn get_filter_changes_dyn(&self, id: U256) -> TransportResult<FilterChanges> {
        self.client().prepare("eth_getFilterChanges", (id,)).await
    }

    /// Get the last block number available.
    async fn get_block_number(&self) -> TransportResult<BlockNumber> {
        self.client().prepare("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    /// Gets the transaction count of the corresponding address.
    async fn get_transaction_count(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> TransportResult<U256> {
        self.client().prepare("eth_getTransactionCount", (address, tag.unwrap_or_default())).await
    }

    /// Get a block by its number.
    // TODO: Network associate
    async fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        hydrate: bool,
    ) -> TransportResult<Option<Block>> {
        self.client().prepare("eth_getBlockByNumber", (number, hydrate)).await
    }

    /// Populates the legacy gas price field of the given transaction request.
    async fn populate_gas(
        &self,
        tx: &mut N::TransactionRequest,
        block: Option<BlockId>,
    ) -> TransportResult<()> {
        let gas = self.estimate_gas(&*tx, block).await;

        gas.map(|gas| tx.set_gas_limit(gas))
    }

    /// Populates the EIP-1559 gas price fields of the given transaction request.
    async fn populate_gas_eip1559(
        &self,
        tx: &mut N::TransactionRequest,
        estimator: Option<EstimatorFunction>,
    ) -> TransportResult<()> {
        let gas = self.estimate_eip1559_fees(estimator).await;

        gas.map(|(max_fee_per_gas, max_priority_fee_per_gas)| {
            tx.set_max_fee_per_gas(max_fee_per_gas);
            tx.set_max_priority_fee_per_gas(max_priority_fee_per_gas);
        })
    }

    /// Broadcasts a transaction to the network.
    ///
    /// Returns a type that can be used to configure how and when to await the transaction's
    /// confirmation.
    ///
    /// # Examples
    ///
    /// See [`PendingTransactionBuilder`](crate::PendingTransactionBuilder) for more examples.
    ///
    /// ```no_run
    /// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider<N>, tx: N::TransactionRequest) -> Result<(), Box<dyn std::error::Error>> {
    /// let tx_hash = provider.send_transaction(tx)
    ///     .await?
    ///     .with_confirmations(2)
    ///     .with_timeout(Some(std::time::Duration::from_secs(60)))
    /// #   .with_provider(&provider) // TODO
    ///     .watch()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionConfig> {
        let tx_hash = self.client().prepare("eth_sendTransaction", (tx,)).await?;
        Ok(PendingTransactionConfig::new(tx_hash))
    }

    /// Broadcasts a raw transaction RLP bytes to the network.
    ///
    /// See [`send_transaction`](Self::send_transaction) for more details.
    async fn send_raw_transaction(
        &self,
        rlp_bytes: &[u8],
    ) -> TransportResult<PendingTransactionConfig> {
        let rlp_hex = hex::encode(rlp_bytes);
        let tx_hash = self.client().prepare("eth_sendRawTransaction", (rlp_hex,)).await?;
        Ok(PendingTransactionConfig::new(tx_hash))
    }

    /// Gets the balance of the account at the specified tag, which defaults to latest.
    async fn get_balance(&self, address: Address, tag: Option<BlockId>) -> TransportResult<U256> {
        self.client()
            .prepare(
                "eth_getBalance",
                (address, tag.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest))),
            )
            .await
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
        self.client().prepare("eth_getBlockByHash", (hash, full)).await
    }

    /// Gets the client version of the chain client().
    async fn get_client_version(&self) -> TransportResult<String> {
        self.client().prepare("web3_clientVersion", ()).await
    }

    /// Gets the chain ID.
    async fn get_chain_id(&self) -> TransportResult<U64> {
        self.client().prepare("eth_chainId", ()).await
    }

    /// Gets the network ID. Same as `eth_chainId`.
    async fn get_net_version(&self) -> TransportResult<U64> {
        self.client().prepare("net_version", ()).await
    }

    /// Gets the specified storage value from [Address].
    async fn get_storage_at(
        &self,
        address: Address,
        key: U256,
        tag: Option<BlockId>,
    ) -> TransportResult<StorageValue> {
        self.client().prepare("eth_getStorageAt", (address, key, tag.unwrap_or_default())).await
    }

    /// Gets the bytecode located at the corresponding [Address].
    async fn get_code_at(&self, address: Address, tag: BlockId) -> TransportResult<Bytes> {
        self.client().prepare("eth_getCode", (address, tag)).await
    }

    /// Gets a transaction by its [TxHash].
    async fn get_transaction_by_hash(
        &self,
        hash: TxHash,
    ) -> TransportResult<N::TransactionResponse> {
        self.client().prepare("eth_getTransactionByHash", (hash,)).await
    }

    /// Retrieves a [`Vec<Log>`] with the given [Filter].
    async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
        self.client().prepare("eth_getLogs", (filter,)).await
    }

    /// Gets the accounts in the remote node. This is usually empty unless you're using a local
    /// node.
    async fn get_accounts(&self) -> TransportResult<Vec<Address>> {
        self.client().prepare("eth_accounts", ()).await
    }

    /// Gets the current gas price.
    async fn get_gas_price(&self) -> TransportResult<U256> {
        self.client().prepare("eth_gasPrice", ()).await
    }

    /// Gets a transaction receipt if it exists, by its [TxHash].
    async fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> TransportResult<Option<N::ReceiptResponse>> {
        self.client().prepare("eth_getTransactionReceipt", (hash,)).await
    }

    /// Returns a collection of historical gas information [FeeHistory] which
    /// can be used to calculate the EIP1559 fields `maxFeePerGas` and `maxPriorityFeePerGas`.
    async fn get_fee_history(
        &self,
        block_count: U256,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory> {
        self.client().prepare("eth_feeHistory", (block_count, last_block, reward_percentiles)).await
    }

    /// Gets the selected block [BlockNumberOrTag] receipts.
    async fn get_block_receipts(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Option<Vec<N::ReceiptResponse>>> {
        self.client().prepare("eth_getBlockReceipts", (block,)).await
    }

    /// Gets an uncle block through the tag [BlockId] and index [U64].
    async fn get_uncle(&self, tag: BlockId, idx: U64) -> TransportResult<Option<Block>> {
        match tag {
            BlockId::Hash(hash) => {
                self.client().prepare("eth_getUncleByBlockHashAndIndex", (hash, idx)).await
            }
            BlockId::Number(number) => {
                self.client().prepare("eth_getUncleByBlockNumberAndIndex", (number, idx)).await
            }
        }
    }

    /// Gets syncing info.
    async fn syncing(&self) -> TransportResult<SyncStatus> {
        self.client().prepare("eth_syncing", ()).await
    }

    /// Execute a smart contract call with a transaction request, without publishing a transaction.
    async fn call(
        &self,
        tx: &N::TransactionRequest,
        block: Option<BlockId>,
    ) -> TransportResult<Bytes> {
        self.client().prepare("eth_call", (tx, block.unwrap_or_default())).await
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
        block: Option<BlockId>,
        state: StateOverride,
    ) -> TransportResult<Bytes> {
        self.client().prepare("eth_call", (tx, block.unwrap_or_default(), state)).await
    }

    /// Estimate the gas needed for a transaction.
    async fn estimate_gas(
        &self,
        tx: &N::TransactionRequest,
        block: Option<BlockId>,
    ) -> TransportResult<U256> {
        if let Some(block_id) = block {
            self.client().prepare("eth_estimateGas", (tx, block_id)).await
        } else {
            self.client().prepare("eth_estimateGas", (tx,)).await
        }
    }

    /// Estimates the EIP1559 `maxFeePerGas` and `maxPriorityFeePerGas` fields.
    ///
    /// Receives an optional [EstimatorFunction] that can be used to modify
    /// how to estimate these fees.
    async fn estimate_eip1559_fees(
        &self,
        estimator: Option<EstimatorFunction>,
    ) -> TransportResult<(U256, U256)> {
        let base_fee_per_gas = match self.get_block_by_number(BlockNumberOrTag::Latest, false).await
        {
            Ok(Some(block)) => match block.header.base_fee_per_gas {
                Some(base_fee_per_gas) => base_fee_per_gas,
                None => return Err(TransportErrorKind::custom_str("EIP-1559 not activated")),
            },

            Ok(None) => return Err(TransportErrorKind::custom_str("Latest block not found")),

            Err(err) => return Err(err),
        };

        let fee_history = match self
            .get_fee_history(
                U256::from(utils::EIP1559_FEE_ESTIMATION_PAST_BLOCKS),
                BlockNumberOrTag::Latest,
                &[utils::EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await
        {
            Ok(fee_history) => fee_history,
            Err(err) => return Err(err),
        };

        // use the provided fee estimator function, or fallback to the default implementation.
        let (max_fee_per_gas, max_priority_fee_per_gas) = if let Some(es) = estimator {
            es(base_fee_per_gas, fee_history.reward.unwrap_or_default())
        } else {
            utils::eip1559_default_estimator(
                base_fee_per_gas,
                fee_history.reward.unwrap_or_default(),
            )
        };

        Ok((max_fee_per_gas, max_priority_fee_per_gas))
    }

    /// Get the account and storage values of the specified account including the merkle proofs.
    ///
    /// This call can be used to verify that the data has not been tampered with.
    async fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
        block: Option<BlockId>,
    ) -> TransportResult<EIP1186AccountProofResponse> {
        self.client().prepare("eth_getProof", (address, keys, block.unwrap_or_default())).await
    }

    /// Create an [EIP-2930] access list.
    ///
    /// [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    async fn create_access_list(
        &self,
        request: &N::TransactionRequest,
        block: Option<BlockId>,
    ) -> TransportResult<AccessListWithGasUsed> {
        self.client().prepare("eth_createAccessList", (request, block.unwrap_or_default())).await
    }

    // todo: move to extension trait
    /// Parity trace transaction.
    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().prepare("trace_transaction", (hash,)).await
    }

    // todo: move to extension trait
    /// Trace the given transaction.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace> {
        self.client().prepare("debug_traceTransaction", (hash, trace_options)).await
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
        self.client().prepare("trace_block", (block,)).await
    }
}

/// Extension trait for Anvil specific JSON-RPC methods.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait AnvilProvider<N: Network, T: Transport + Clone = BoxTransport>: Provider<N, T> {
    /// Set the bytecode of a given account.
    async fn set_code(&self, address: Address, code: &'static str) -> TransportResult<()> {
        self.client().prepare("anvil_setCode", (address, code)).await
    }
}

impl<P, N: Network, T: Transport + Clone> AnvilProvider<N, T> for P where P: Provider<N, T> {}

/// Extension trait for raw RPC requests.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait RawProvider<N: Network, T: Transport + Clone = BoxTransport>: Provider<N, T> {
    /// Sends a raw JSON-RPC request.
    async fn raw_request<P, R>(&self, method: &'static str, params: P) -> TransportResult<R>
    where
        P: RpcParam,
        R: RpcReturn,
        Self: Sized,
    {
        self.client().prepare(method, &params).await
    }

    /// Sends a raw JSON-RPC request with type-erased parameters and return.
    async fn raw_request_dyn(
        &self,
        method: &'static str,
        params: &RawValue,
    ) -> TransportResult<Box<RawValue>> {
        self.client().prepare(method, params).await
    }
}

impl<P, N: Network, T: Transport + Clone> RawProvider<N, T> for P where P: Provider<N, T> {}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProvider<N, T> {
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

// Internal implementation for [`ChainStreamPoller`].
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProviderInner<N, T> {
    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.client_ref()
    }

    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.weak_client()
    }

    async fn watch_pending_transaction(
        &self,
        _config: PendingTransactionConfig,
    ) -> TransportResult<PendingTransaction> {
        unimplemented!()
    }
}

#[cfg(test)]
#[allow(clippy::missing_const_for_fn)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, bytes};
    use alloy_rpc_types::request::TransactionRequest;

    extern crate self as alloy_provider;

    // NOTE: We cannot import the test-utils crate here due to a circular dependency.
    include!("../../internal-test-utils/src/providers.rs");

    #[tokio::test]
    async fn object_safety() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        // These blocks are not necessary.
        {
            let refdyn = &provider as &dyn Provider<Ethereum, Http<reqwest::Client>>;
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
            let refdyn = &provider as &dyn Provider<Ethereum, Http<reqwest::Client>>;
            let num = refdyn.get_block_number().await.unwrap();
            assert_eq!(0, num);
        }

        let boxed = provider.boxed();
        let num = boxed.get_block_number().await.unwrap();
        assert_eq!(0, num);

        let boxed_boxdyn = Box::new(boxed) as Box<dyn Provider<Ethereum>>;
        let num = boxed_boxdyn.get_block_number().await.unwrap();
        assert_eq!(0, num);
    }

    #[test]
    fn object_safety_types() {
        fn is_provider<N: Network, T: Transport + Clone, P: Provider<N, T>>() {}
        fn is_raw_provider<N: Network, T: Transport + Clone, P: RawProvider<N, T>>() {}

        is_provider::<_, _, Box<dyn Provider<Ethereum>>>();
        is_provider::<_, _, Box<dyn RawProvider<Ethereum>>>();
        is_raw_provider::<_, _, Box<dyn Provider<Ethereum>>>();
        is_raw_provider::<_, _, Box<dyn RawProvider<Ethereum>>>();
    }

    #[tokio::test]
    async fn test_send_tx() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(U256::from(20e9)),
            gas: Some(U256::from(21000)),
            ..Default::default()
        };

        let pending_tx = provider.send_transaction(tx.clone()).await.expect("failed to send tx");
        let hash1 = *pending_tx.tx_hash();
        let hash2 =
            pending_tx.with_provider(&provider).watch().await.expect("failed to await pending tx");
        assert_eq!(hash1, hash2);

        let pending_tx = provider.send_transaction(tx).await.expect("failed to send tx");
        let hash1 = *pending_tx.tx_hash();
        let hash2 = pending_tx
            .with_provider(provider)
            .get_receipt()
            .await
            .expect("failed to await pending tx")
            .unwrap()
            .transaction_hash
            .unwrap();
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
        use super::RawProvider;

        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num: U64 = provider.raw_request("eth_blockNumber", ()).await.unwrap();
        assert_eq!(0, num.to::<u64>())
    }

    #[tokio::test]
    async fn gets_transaction_count() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let count = provider
            .get_transaction_count(
                address!("328375e18E7db8F1CA9d9bA8bF3E9C94ee34136A"),
                Some(BlockNumberOrTag::Latest.into()),
            )
            .await
            .unwrap();
        assert_eq!(count, U256::from(0));
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
        use super::RawProvider;

        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        let hash = block.header.hash.unwrap();
        let block: Block = provider
            .raw_request::<(B256, bool), Block>("eth_getBlockByHash", (hash, true))
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
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_block_by_number() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
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
        let chain_id: u64 = 13371337;
        let anvil = Anvil::new().args(["--chain-id", chain_id.to_string().as_str()]).spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        let provider = RootProvider::<Ethereum, _>::new(RpcClient::new(http, true));

        let chain_id = provider.get_chain_id().await.unwrap();
        assert_eq!(chain_id, U64::from(chain_id));
    }

    #[tokio::test]
    async fn gets_network_id() {
        let chain_id: u64 = 13371337;
        let anvil = Anvil::new().args(["--chain-id", chain_id.to_string().as_str()]).spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        let provider = RootProvider::<Ethereum, _>::new(RpcClient::new(http, true));

        let chain_id = provider.get_net_version().await.unwrap();
        assert_eq!(chain_id, U64::from(chain_id));
    }

    #[tokio::test]
    #[cfg(feature = "anvil")]
    async fn gets_code_at() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        // Set the code
        let addr = Address::with_last_byte(16);
        provider.set_code(addr, "0xbeef").await.unwrap();
        let _code = provider
            .get_code_at(addr, BlockId::Number(alloy_rpc_types::BlockNumberOrTag::Latest))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn gets_storage_at() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let addr = Address::with_last_byte(16);
        let storage = provider.get_storage_at(addr, U256::ZERO, None).await.unwrap();
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
        assert_eq!(tx.block_number.unwrap(), U256::from(4571819));
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
            receipt.transaction_hash.unwrap(),
            b256!("5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95")
        );
    }

    #[tokio::test]
    async fn gets_fee_history() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let block_number = provider.get_block_number().await.unwrap();
        let fee_history = provider
            .get_fee_history(
                U256::from(utils::EIP1559_FEE_ESTIMATION_PAST_BLOCKS),
                BlockNumberOrTag::Number(block_number),
                &[utils::EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await
            .unwrap();
        assert_eq!(fee_history.oldest_block, U256::ZERO);
    }

    #[tokio::test]
    #[ignore] // Anvil has yet to implement the `eth_getBlockReceipts` method.
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
}
