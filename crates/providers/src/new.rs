use crate::{
    chain::ChainStreamPoller,
    heart::{Heartbeat, HeartbeatHandle, PendingTransaction, WatchConfig},
    utils,
    utils::EstimatorFunction,
};
use alloy_network::Network;
use alloy_primitives::{
    hex, Address, BlockHash, BlockNumber, Bytes, StorageKey, StorageValue, TxHash, B256, U256, U64,
};
use alloy_rpc_client::{ClientRef, RpcClient, WeakClient};
use alloy_rpc_trace_types::{
    geth::{GethDebugTracingOptions, GethTrace},
    parity::LocalizedTransactionTrace,
};
use alloy_rpc_types::{
    state::StateOverride, AccessListWithGasUsed, Block, BlockId, BlockNumberOrTag,
    EIP1186AccountProofResponse, FeeHistory, Filter, Log, SyncStatus,
};
use alloy_transport::{BoxTransport, Transport, TransportErrorKind, TransportResult};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock, Weak},
};

/// A [`Provider`] in a [`Weak`] reference.
pub type WeakProvider<P> = Weak<P>;

/// A borrowed [`Provider`].
pub type ProviderRef<'a, P> = &'a P;

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<N, T> {
    /// The inner state of the root provider.
    pub(crate) inner: Arc<RootProviderInner<N, T>>,
}

impl<N: Network, T: Transport> RootProvider<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { inner: Arc::new(RootProviderInner::new(client)) }
    }
}

impl<N: Network, T: Transport + Clone> RootProvider<N, T> {
    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction> {
        // TODO: Make this configurable.
        let cfg = WatchConfig::new(tx_hash);
        self.get_heart().watch_tx(cfg).await.map_err(|_| TransportErrorKind::backend_gone())
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

    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction>;

    /// Get the last block number available.
    async fn get_block_number(&self) -> TransportResult<BlockNumber> {
        self.client().prepare("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    /// Gets the transaction count of the corresponding address.
    async fn get_transaction_count(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> TransportResult<alloy_primitives::U256> {
        self.client().prepare("eth_getTransactionCount", (address, tag.unwrap_or_default())).await
    }

    /// Get a block by its number.
    ///
    /// TODO: Network associate
    async fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        hydrate: bool,
    ) -> TransportResult<Option<Block>> {
        self.client().prepare("eth_getBlockByNumber", (number, hydrate)).await
    }

    // todo eip-1559 and blobs as well
    async fn populate_gas(
        &self,
        tx: &mut N::TransactionRequest,
        block: Option<BlockId>,
    ) -> TransportResult<()> {
        let _ = self.estimate_gas(&*tx, block).await;

        todo!()
        // gas.map(|gas| tx.set_gas_limit(gas.try_into().unwrap()))
    }

    /// Broadcasts a transaction, returning a [`PendingTransaction`] that resolves once the
    /// transaction has been confirmed.
    async fn send_transaction(
        &self,
        tx: &N::TransactionRequest,
    ) -> TransportResult<PendingTransaction> {
        let tx_hash = self.client().prepare("eth_sendTransaction", (tx,)).await?;
        self.new_pending_transaction(tx_hash).await
    }

    /// Broadcasts a transaction's raw RLP bytes, returning a [`PendingTransaction`] that resolves
    /// once the transaction has been confirmed.
    async fn send_raw_transaction(&self, rlp_bytes: &[u8]) -> TransportResult<PendingTransaction> {
        let rlp_hex = hex::encode(rlp_bytes);
        let tx_hash = self.client().prepare("eth_sendRawTransaction", (rlp_hex,)).await?;
        self.new_pending_transaction(tx_hash).await
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
    async fn get_logs(&self, filter: Filter) -> TransportResult<Vec<Log>> {
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

    // todo: move to extension trait
    #[cfg(feature = "anvil")]
    async fn set_code(&self, address: Address, code: &'static str) -> TransportResult<()> {
        self.client().prepare("anvil_setCode", (address, code)).await
    }

    async fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
        block: Option<BlockId>,
    ) -> TransportResult<EIP1186AccountProofResponse> {
        self.client().prepare("eth_getProof", (address, keys, block.unwrap_or_default())).await
    }

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
    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace> {
        self.client().prepare("debug_traceTransaction", (hash, trace_options)).await
    }

    // todo: move to extension trait
    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().prepare("trace_block", (block,)).await
    }
}

/// Extension trait for raw RPC requests.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait RawProvider<N: Network, T: Transport + Clone = BoxTransport>: Provider<N, T> {
    /// Sends a raw JSON-RPC request.
    async fn raw_request<P, R>(&self, method: &'static str, params: P) -> TransportResult<R>
    where
        P: Serialize + Send + Sync + Clone,
        R: Serialize + DeserializeOwned + Send + Sync + Unpin + 'static,
        Self: Sync,
    {
        let res: R = self.client().prepare(method, &params).await?;
        Ok(res)
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
    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction> {
        RootProvider::new_pending_transaction(self, tx_hash).await
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

    #[inline]
    async fn new_pending_transaction(&self, _tx_hash: B256) -> TransportResult<PendingTransaction> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpProvider;
    use alloy_network::Ethereum;
    use alloy_node_bindings::{Anvil, AnvilInstance};
    use alloy_primitives::{address, b256, bytes};
    use alloy_rpc_types::request::TransactionRequest;
    use alloy_transport_http::Http;
    use reqwest::Client;

    struct _ObjectSafe<N: Network>(dyn Provider<N>);

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    fn anvil_provider() -> (HttpProvider<Ethereum>, AnvilInstance) {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        (RootProvider::<Ethereum, _>::new(RpcClient::new(http, true)), anvil)
    }

    #[tokio::test]
    async fn test_send_tx() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(U256::from(20e9)),
            gas: Some(U256::from(21000)),
            ..Default::default()
        };
        let pending_tx = provider.send_transaction(&tx).await.expect("failed to send tx");
        let hash1 = pending_tx.tx_hash;
        let hash2 = pending_tx.await.expect("failed to await pending tx");
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn gets_block_number() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let num = provider.get_block_number().await.unwrap();
        assert_eq!(0, num)
    }

    #[tokio::test]
    async fn gets_block_number_with_raw_req() {
        use super::RawProvider;

        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let num: U64 = provider.raw_request("eth_blockNumber", ()).await.unwrap();
        assert_eq!(0, num.to::<u64>())
    }

    #[tokio::test]
    async fn gets_transaction_count() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

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
        let (provider, _anvil) = anvil_provider();

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
        let (provider, _anvil) = anvil_provider();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        let hash = block.header.hash.unwrap();
        let block: Block = provider
            .raw_request::<(alloy_primitives::FixedBytes<32>, bool), Block>(
                "eth_getBlockByHash",
                (hash, true),
            )
            .await
            .unwrap();
        assert_eq!(block.header.hash.unwrap(), hash);
    }

    #[tokio::test]
    async fn gets_block_by_number_full() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_block_by_number() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_client_version() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

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
        let (provider, _anvil) = anvil_provider();

        // Set the code
        let addr = alloy_primitives::Address::with_last_byte(16);
        provider.set_code(addr, "0xbeef").await.unwrap();
        let _code = provider
            .get_code_at(addr, BlockId::Number(alloy_rpc_types::BlockNumberOrTag::Latest))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn gets_storage_at() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let addr = alloy_primitives::Address::with_last_byte(16);
        let storage = provider.get_storage_at(addr, U256::ZERO, None).await.unwrap();
        assert_eq!(storage, U256::ZERO);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_transaction_by_hash() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

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
        let (provider, _anvil) = anvil_provider();

        let filter = Filter::new()
            .at_block_hash(b256!(
                "b20e6f35d4b46b3c4cd72152faec7143da851a0dc281d390bdd50f58bfbdb5d3"
            ))
            .event_signature(b256!(
                "e1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"
            ));
        let logs = provider.get_logs(filter).await.unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_tx_receipt() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

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
        let (provider, _anvil) = anvil_provider();

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
        let (provider, _anvil) = anvil_provider();

        let receipts = provider.get_block_receipts(BlockNumberOrTag::Latest).await.unwrap();
        assert!(receipts.is_some());
    }

    #[tokio::test]
    async fn gets_block_traces() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

        let traces = provider.trace_block(BlockNumberOrTag::Latest).await.unwrap();
        assert_eq!(traces.len(), 0);
    }

    #[tokio::test]
    async fn sends_raw_transaction() {
        init_tracing();
        let (provider, _anvil) = anvil_provider();

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
