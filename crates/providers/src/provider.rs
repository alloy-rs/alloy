//! Alloy main Provider abstraction.

use crate::utils::{self, EstimatorFunction};
use alloy_primitives::{Address, BlockHash, Bytes, StorageKey, StorageValue, TxHash, U256, U64};
use alloy_rpc_client::{ClientBuilder, RpcClient};
use alloy_rpc_trace_types::{
    geth::{GethDebugTracingOptions, GethTrace},
    parity::LocalizedTransactionTrace,
};
use alloy_rpc_types::{
    AccessListWithGasUsed, Block, BlockId, BlockNumberOrTag, CallRequest,
    EIP1186AccountProofResponse, FeeHistory, Filter, Log, SyncStatus, Transaction,
    TransactionReceipt,
};
use alloy_transport::{BoxTransport, Transport, TransportErrorKind, TransportResult};
use alloy_transport_http::Http;
use auto_impl::auto_impl;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ClientError {
    #[error("Could not parse URL")]
    ParseError,
    #[error("Unsupported Tag")]
    UnsupportedBlockIdError,
}

/// An abstract provider for interacting with the [Ethereum JSON RPC
/// API](https://github.com/ethereum/wiki/wiki/JSON-RPC). Must be instantiated
/// with a transport which implements the [Transport] trait.
#[derive(Debug)]
pub struct Provider<T: Transport = BoxTransport> {
    inner: RpcClient<T>,
    from: Option<Address>,
}

/// Temporary Provider trait to be used until the new Provider trait with
/// the Network abstraction is stable.
/// Once the new Provider trait is stable, this trait will be removed.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[auto_impl(&, &mut, Rc, Arc, Box)]
pub trait TempProvider: Send + Sync {
    /// Gets the transaction count of the corresponding address.
    async fn get_transaction_count(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> TransportResult<alloy_primitives::U256>;

    /// Gets the last block number available.
    async fn get_block_number(&self) -> TransportResult<u64>;

    /// Gets the balance of the account at the specified tag, which defaults to latest.
    async fn get_balance(&self, address: Address, tag: Option<BlockId>) -> TransportResult<U256>;

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
    ) -> TransportResult<Option<Block>>;

    /// Gets a block by [BlockNumberOrTag], with full transactions or only hashes.
    async fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        full: bool,
    ) -> TransportResult<Option<Block>>;

    /// Gets the chain ID.
    async fn get_chain_id(&self) -> TransportResult<U64>;

    /// Gets the specified storage value from [Address].
    async fn get_storage_at(
        &self,
        address: Address,
        key: U256,
        tag: Option<BlockId>,
    ) -> TransportResult<StorageValue>;

    /// Gets the bytecode located at the corresponding [Address].
    async fn get_code_at(&self, address: Address, tag: BlockId) -> TransportResult<Bytes>;

    /// Gets a [Transaction] by its [TxHash].
    async fn get_transaction_by_hash(&self, hash: TxHash) -> TransportResult<Transaction>;

    /// Retrieves a [`Vec<Log>`] with the given [Filter].
    async fn get_logs(&self, filter: Filter) -> TransportResult<Vec<Log>>;

    /// Gets the accounts in the remote node. This is usually empty unless you're using a local
    /// node.
    async fn get_accounts(&self) -> TransportResult<Vec<Address>>;

    /// Gets the current gas price.
    async fn get_gas_price(&self) -> TransportResult<U256>;

    /// Gets a [TransactionReceipt] if it exists, by its [TxHash].
    async fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> TransportResult<Option<TransactionReceipt>>;

    /// Returns a collection of historical gas information [FeeHistory] which
    /// can be used to calculate the EIP1559 fields `maxFeePerGas` and `maxPriorityFeePerGas`.
    async fn get_fee_history(
        &self,
        block_count: U256,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory>;

    /// Gets the selected block [BlockNumberOrTag] receipts.
    async fn get_block_receipts(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Option<Vec<TransactionReceipt>>>;

    /// Gets an uncle block through the tag [BlockId] and index [U64].
    async fn get_uncle(&self, tag: BlockId, idx: U64) -> TransportResult<Option<Block>>;

    /// Gets syncing info.
    async fn syncing(&self) -> TransportResult<SyncStatus>;

    /// Execute a smart contract call with [CallRequest] without publishing a transaction.
    async fn call(&self, tx: CallRequest, block: Option<BlockId>) -> TransportResult<Bytes>;

    /// Estimate the gas needed for a transaction.
    async fn estimate_gas(&self, tx: CallRequest, block: Option<BlockId>) -> TransportResult<U256>;

    /// Sends an already-signed transaction.
    async fn send_raw_transaction(&self, tx: Bytes) -> TransportResult<TxHash>;

    /// Estimates the EIP1559 `maxFeePerGas` and `maxPriorityFeePerGas` fields.
    /// Receives an optional [EstimatorFunction] that can be used to modify
    /// how to estimate these fees.
    async fn estimate_eip1559_fees(
        &self,
        estimator: Option<EstimatorFunction>,
    ) -> TransportResult<(U256, U256)>;

    #[cfg(feature = "anvil")]
    async fn set_code(&self, address: Address, code: &'static str) -> TransportResult<()>;

    async fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
        block: Option<BlockId>,
    ) -> TransportResult<EIP1186AccountProofResponse>;

    async fn create_access_list(
        &self,
        request: CallRequest,
        block: Option<BlockId>,
    ) -> TransportResult<AccessListWithGasUsed>;

    /// Parity trace transaction.
    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace>;

    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    async fn raw_request<P, R>(&self, method: &'static str, params: P) -> TransportResult<R>
    where
        P: Serialize + Send + Sync + Clone,
        R: Serialize + DeserializeOwned + Send + Sync + Unpin + 'static,
        Self: Sync;
}

impl<T: Transport + Clone + Send + Sync> Provider<T> {
    pub fn new(transport: T) -> Self {
        Self {
            // todo(onbjerg): do we just default to false
            inner: RpcClient::new(transport, false),
            from: None,
        }
    }

    pub fn new_with_client(client: RpcClient<T>) -> Self {
        Self { inner: client, from: None }
    }

    pub fn with_sender(mut self, from: Address) -> Self {
        self.from = Some(from);
        self
    }

    pub fn inner(&self) -> &RpcClient<T> {
        &self.inner
    }
}

// todo: validate usage of BlockId vs BlockNumberOrTag vs Option<BlockId> etc.
// Simple JSON-RPC bindings.
// In the future, this will be replaced by a Provider trait,
// but as the interface is not stable yet, we define the bindings ourselves
// until we can use the trait and the client abstraction that will use it.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<T: Transport + Clone + Send + Sync> TempProvider for Provider<T> {
    /// Gets the transaction count of the corresponding address.
    async fn get_transaction_count(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> TransportResult<alloy_primitives::U256> {
        self.inner.prepare("eth_getTransactionCount", (address, tag.unwrap_or_default())).await
    }

    /// Gets the last block number available.
    /// Gets the last block number available.
    async fn get_block_number(&self) -> TransportResult<u64> {
        self.inner.prepare("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    /// Gets the balance of the account at the specified tag, which defaults to latest.
    async fn get_balance(&self, address: Address, tag: Option<BlockId>) -> TransportResult<U256> {
        self.inner
            .prepare(
                "eth_getBalance",
                (address, tag.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest))),
            )
            .await
    }

    /// Gets a block by its [BlockHash], with full transactions or only hashes.
    async fn get_block_by_hash(
        &self,
        hash: BlockHash,
        full: bool,
    ) -> TransportResult<Option<Block>> {
        self.inner.prepare("eth_getBlockByHash", (hash, full)).await
    }

    /// Gets a block by [BlockNumberOrTag], with full transactions or only hashes.
    async fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        full: bool,
    ) -> TransportResult<Option<Block>> {
        self.inner.prepare("eth_getBlockByNumber", (number, full)).await
    }

    /// Gets the chain ID.
    async fn get_chain_id(&self) -> TransportResult<U64> {
        self.inner.prepare("eth_chainId", ()).await
    }

    /// Gets the specified storage value from [Address].
    async fn get_storage_at(
        &self,
        address: Address,
        key: U256,
        tag: Option<BlockId>,
    ) -> TransportResult<StorageValue> {
        self.inner.prepare("eth_getStorageAt", (address, key, tag.unwrap_or_default())).await
    }

    /// Gets the bytecode located at the corresponding [Address].
    async fn get_code_at(&self, address: Address, tag: BlockId) -> TransportResult<Bytes> {
        self.inner.prepare("eth_getCode", (address, tag)).await
    }

    /// Gets a [Transaction] by its [TxHash].
    async fn get_transaction_by_hash(&self, hash: TxHash) -> TransportResult<Transaction> {
        self.inner.prepare("eth_getTransactionByHash", (hash,)).await
    }

    /// Retrieves a [`Vec<Log>`] with the given [Filter].
    async fn get_logs(&self, filter: Filter) -> TransportResult<Vec<Log>> {
        self.inner.prepare("eth_getLogs", (filter,)).await
    }

    /// Gets the accounts in the remote node. This is usually empty unless you're using a local
    /// node.
    async fn get_accounts(&self) -> TransportResult<Vec<Address>> {
        self.inner.prepare("eth_accounts", ()).await
    }

    /// Gets the current gas price.
    async fn get_gas_price(&self) -> TransportResult<U256> {
        self.inner.prepare("eth_gasPrice", ()).await
    }

    /// Gets a [TransactionReceipt] if it exists, by its [TxHash].
    async fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> TransportResult<Option<TransactionReceipt>> {
        self.inner.prepare("eth_getTransactionReceipt", (hash,)).await
    }

    /// Returns a collection of historical gas information [FeeHistory] which
    /// can be used to calculate the EIP1559 fields `maxFeePerGas` and `maxPriorityFeePerGas`.
    async fn get_fee_history(
        &self,
        block_count: U256,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory> {
        self.inner.prepare("eth_feeHistory", (block_count, last_block, reward_percentiles)).await
    }

    /// Gets the selected block [BlockNumberOrTag] receipts.
    async fn get_block_receipts(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Option<Vec<TransactionReceipt>>> {
        self.inner.prepare("eth_getBlockReceipts", (block,)).await
    }

    /// Gets an uncle block through the tag [BlockId] and index [U64].
    async fn get_uncle(&self, tag: BlockId, idx: U64) -> TransportResult<Option<Block>> {
        match tag {
            BlockId::Hash(hash) => {
                self.inner.prepare("eth_getUncleByBlockHashAndIndex", (hash, idx)).await
            }
            BlockId::Number(number) => {
                self.inner.prepare("eth_getUncleByBlockNumberAndIndex", (number, idx)).await
            }
        }
    }

    /// Gets syncing info.
    async fn syncing(&self) -> TransportResult<SyncStatus> {
        self.inner.prepare("eth_syncing", ()).await
    }

    /// Execute a smart contract call with [CallRequest] without publishing a transaction.
    async fn call(&self, tx: CallRequest, block: Option<BlockId>) -> TransportResult<Bytes> {
        self.inner.prepare("eth_call", (tx, block.unwrap_or_default())).await
    }

    /// Estimate the gas needed for a transaction.
    async fn estimate_gas(&self, tx: CallRequest, block: Option<BlockId>) -> TransportResult<U256> {
        if let Some(block_id) = block {
            self.inner.prepare("eth_estimateGas", (tx, block_id)).await
        } else {
            self.inner.prepare("eth_estimateGas", (tx,)).await
        }
    }

    /// Sends an already-signed transaction.
    async fn send_raw_transaction(&self, tx: Bytes) -> TransportResult<TxHash> {
        self.inner.prepare("eth_sendRawTransaction", (tx,)).await
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

    async fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
        block: Option<BlockId>,
    ) -> TransportResult<EIP1186AccountProofResponse> {
        self.inner.prepare("eth_getProof", (address, keys, block.unwrap_or_default())).await
    }

    async fn create_access_list(
        &self,
        request: CallRequest,
        block: Option<BlockId>,
    ) -> TransportResult<AccessListWithGasUsed> {
        self.inner.prepare("eth_createAccessList", (request, block.unwrap_or_default())).await
    }

    /// Parity trace transaction.
    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.inner.prepare("trace_transaction", (hash,)).await
    }

    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace> {
        self.inner.prepare("debug_traceTransaction", (hash, trace_options)).await
    }

    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.inner.prepare("trace_block", (block,)).await
    }

    /// Sends a raw request with the methods and params specified to the internal connection,
    /// and returns the result.
    async fn raw_request<P, R>(&self, method: &'static str, params: P) -> TransportResult<R>
    where
        P: Serialize + Send + Sync + Clone,
        R: Serialize + DeserializeOwned + Send + Sync + Unpin + 'static,
    {
        let res: R = self.inner.prepare(method, &params).await?;
        Ok(res)
    }

    #[cfg(feature = "anvil")]
    async fn set_code(&self, address: Address, code: &'static str) -> TransportResult<()> {
        self.inner.prepare("anvil_setCode", (address, code)).await
    }
}

impl TryFrom<&str> for Provider<Http<Client>> {
    type Error = ClientError;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = url.parse().map_err(|_e| ClientError::ParseError)?;
        let inner = ClientBuilder::default().reqwest_http(url);

        Ok(Self { inner, from: None })
    }
}

impl TryFrom<String> for Provider<Http<Client>> {
    type Error = ClientError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Provider::try_from(value.as_str())
    }
}

impl<'a> TryFrom<&'a String> for Provider<Http<Client>> {
    type Error = ClientError;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Provider::try_from(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        provider::{Provider, TempProvider},
        utils,
    };
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, b256, bytes, U256, U64};
    use alloy_rpc_types::{Block, BlockNumberOrTag, Filter};

    #[tokio::test]
    async fn gets_block_number() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let num = provider.get_block_number().await.unwrap();
        assert_eq!(0, num)
    }

    #[tokio::test]
    async fn gets_block_number_with_raw_req() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let num: U64 = provider.raw_request("eth_blockNumber", ()).await.unwrap();
        assert_eq!(0, num.to::<u64>())
    }

    #[tokio::test]
    async fn gets_transaction_count() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
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
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        let hash = block.header.hash.unwrap();
        let block = provider.get_block_by_hash(hash, true).await.unwrap().unwrap();
        assert_eq!(block.header.hash.unwrap(), hash);
    }

    #[tokio::test]
    async fn gets_block_by_hash_with_raw_req() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
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
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_block_by_number() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider.get_block_by_number(tag, true).await.unwrap().unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_chain_id() {
        let chain_id: u64 = 13371337;
        let anvil = Anvil::new().args(["--chain-id", chain_id.to_string().as_str()]).spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let chain_id = provider.get_chain_id().await.unwrap();
        assert_eq!(chain_id, U64::from(chain_id));
    }

    #[tokio::test]
    #[cfg(feature = "anvil")]
    async fn gets_code_at() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        // Set the code
        let addr = alloy_primitives::Address::with_last_byte(16);
        provider.set_code(addr, "0xbeef").await.unwrap();
        let _code = provider
            .get_code_at(
                addr,
                crate::provider::BlockId::Number(alloy_rpc_types::BlockNumberOrTag::Latest),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn gets_storage_at() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let addr = alloy_primitives::Address::with_last_byte(16);
        let storage = provider.get_storage_at(addr, U256::ZERO, None).await.unwrap();
        assert_eq!(storage, U256::ZERO);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_transaction_by_hash() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
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
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
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
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
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
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
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
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let receipts = provider.get_block_receipts(BlockNumberOrTag::Latest).await.unwrap();
        assert!(receipts.is_some());
    }

    #[tokio::test]
    async fn gets_block_traces() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let traces = provider.trace_block(BlockNumberOrTag::Latest).await.unwrap();
        assert_eq!(traces.len(), 0);
    }

    #[tokio::test]
    async fn sends_raw_transaction() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::try_from(&anvil.endpoint()).unwrap();
        let tx_hash = provider
            .send_raw_transaction(
                // Transfer 1 ETH from default EOA address to the Genesis address.
                bytes!("f865808477359400825208940000000000000000000000000000000000000000018082f4f5a00505e227c1c636c76fac55795db1a40a4d24840d81b40d2fe0cc85767f6bd202a01e91b437099a8a90234ac5af3cb7ca4fb1432e133f75f9a91678eaf5f487c74b")
            )
            .await.unwrap();
        assert_eq!(
            tx_hash.to_string(),
            "0x9dae5cf33694a02e8a7d5de3fe31e9d05ca0ba6e9180efac4ab20a06c9e598a3"
        );
    }
}
