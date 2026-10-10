#[cfg(feature = "pubsub")]
use crate::GetSubscription;
use crate::{
    EthCall, EthCallMany, EthGetBlock, FilterPollerBuilder, PendingTransaction,
    PendingTransactionBuilder, PendingTransactionConfig, PendingTransactionError, Provider,
    ProviderCall, ProviderLayer, RootProvider, RpcWithBlock, SendableTx,
};
use alloy_eips::{eip7928::BlockAccessList, BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcRecv;
use alloy_network::Network;
use alloy_primitives::{
    Address, BlockHash, BlockNumber, Bytes, StorageKey, StorageValue, TxHash, B256, U128, U256, U64,
};
use alloy_rpc_client::NoParams;
#[cfg(feature = "pubsub")]
use alloy_rpc_types_eth::pubsub::{Params, SubscriptionKind};
use alloy_rpc_types_eth::{
    erc4337::TransactionConditional,
    simulate::{SimulatePayload, SimulatedBlock},
    AccessListResult, Bundle, EIP1186AccountProofResponse, EthCallResponse, FeeHistory,
    FillTransaction, Filter, FilterChanges, Index, Log, StorageValuesRequest,
    StorageValuesResponse, SyncStatus,
};
use alloy_transport::TransportResult;
use async_trait::async_trait;
use serde_json::value::RawValue;
use std::{borrow::Cow, marker::PhantomData};

/// A layer that sets a default [`BlockId`] for RPC methods that support block parameters.
///
/// This layer affects the following methods:
/// - `eth_call`
/// - `eth_estimateGas`
/// - `eth_simulateV1`
/// - `eth_createAccessList`
/// - `eth_getAccountInfo`
/// - `eth_getAccount`
/// - `eth_getBalance`
/// - `eth_getCode`
/// - `eth_getProof`
/// - `eth_getStorageAt`
/// - `eth_getTransactionCount`
///
/// All other [`Provider`] methods are forwarded to the inner provider unchanged, so that
/// layers stacked inside this one (for example a [`CacheProvider`](crate::layers::CacheProvider))
/// keep their overrides. Previously these methods fell back to the trait default, which calls the
/// root client directly and silently bypassed the inner provider (the same class of bug addressed
/// for `FillProvider` in #4333 and for `BlockIdLayer` in #4245).
#[derive(Debug, Clone, Copy)]
pub struct BlockIdLayer {
    block_id: BlockId,
}

impl BlockIdLayer {
    /// Creates a new layer with the given block ID.
    pub const fn new(block_id: BlockId) -> Self {
        Self { block_id }
    }
}

impl From<BlockId> for BlockIdLayer {
    fn from(block_id: BlockId) -> Self {
        Self::new(block_id)
    }
}

impl<P, N> ProviderLayer<P, N> for BlockIdLayer
where
    P: Provider<N>,
    N: Network,
{
    type Provider = BlockIdProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        BlockIdProvider::new(inner, self.block_id)
    }
}

/// A provider that uses a configured default [`BlockId`].
#[derive(Clone, Debug)]
pub struct BlockIdProvider<P, N = alloy_network::Ethereum> {
    inner: P,
    block_id: BlockId,
    _marker: PhantomData<N>,
}

impl<P: Provider<N>, N: Network> BlockIdProvider<P, N> {
    /// Creates a new provider with the given block ID.
    pub const fn new(inner: P, block_id: BlockId) -> Self {
        Self { inner, block_id, _marker: PhantomData }
    }
}

#[async_trait]
impl<P: Provider<N>, N: Network> Provider<N> for BlockIdProvider<P, N> {
    #[inline(always)]
    fn root(&self) -> &RootProvider<N> {
        self.inner.root()
    }

    fn get_accounts(&self) -> ProviderCall<NoParams, Vec<Address>> {
        self.inner.get_accounts()
    }

    fn get_blob_base_fee(&self) -> ProviderCall<NoParams, U128, u128> {
        self.inner.get_blob_base_fee()
    }

    fn get_block_number(&self) -> ProviderCall<NoParams, U64, BlockNumber> {
        self.inner.get_block_number()
    }

    fn call(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        self.inner.call(tx).block(self.block_id)
    }

    fn call_many<'req>(
        &self,
        bundles: &'req [Bundle],
    ) -> EthCallMany<'req, N, Vec<Vec<EthCallResponse>>> {
        self.inner.call_many(bundles)
    }

    fn simulate<'req>(
        &self,
        payload: &'req SimulatePayload,
    ) -> RpcWithBlock<&'req SimulatePayload, Vec<SimulatedBlock<N::BlockResponse>>> {
        self.inner.simulate(payload).block_id(self.block_id)
    }

    fn get_chain_id(&self) -> ProviderCall<NoParams, U64, u64> {
        self.inner.get_chain_id()
    }

    fn create_access_list<'a>(
        &self,
        request: &'a N::TransactionRequest,
    ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult> {
        self.inner.create_access_list(request).block_id(self.block_id)
    }

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        self.inner.estimate_gas(tx).block(self.block_id)
    }

    async fn get_fee_history(
        &self,
        block_count: u64,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory> {
        self.inner.get_fee_history(block_count, last_block, reward_percentiles).await
    }

    fn get_gas_price(&self) -> ProviderCall<NoParams, U128, u128> {
        self.inner.get_gas_price()
    }

    fn get_account_info(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, alloy_rpc_types_eth::AccountInfo> {
        self.inner.get_account_info(address).block_id(self.block_id)
    }

    fn get_account(&self, address: Address) -> RpcWithBlock<Address, alloy_consensus::TrieAccount> {
        self.inner.get_account(address).block_id(self.block_id)
    }

    fn get_balance(&self, address: Address) -> RpcWithBlock<Address, U256, U256> {
        self.inner.get_balance(address).block_id(self.block_id)
    }

    fn get_block(&self, block: BlockId) -> EthGetBlock<N::BlockResponse> {
        self.inner.get_block(block)
    }

    fn get_block_by_hash(&self, hash: BlockHash) -> EthGetBlock<N::BlockResponse> {
        self.inner.get_block_by_hash(hash)
    }

    fn get_block_by_number(&self, number: BlockNumberOrTag) -> EthGetBlock<N::BlockResponse> {
        self.inner.get_block_by_number(number)
    }

    async fn get_block_transaction_count_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<u64>> {
        self.inner.get_block_transaction_count_by_hash(hash).await
    }

    async fn get_block_transaction_count_by_number(
        &self,
        block_number: BlockNumberOrTag,
    ) -> TransportResult<Option<u64>> {
        self.inner.get_block_transaction_count_by_number(block_number).await
    }

    fn get_block_receipts(
        &self,
        block: BlockId,
    ) -> ProviderCall<(BlockId,), Option<Vec<N::ReceiptResponse>>> {
        self.inner.get_block_receipts(block)
    }

    async fn get_block_access_list(
        &self,
        block: BlockId,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.inner.get_block_access_list(block).await
    }

    async fn get_block_access_list_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.inner.get_block_access_list_by_hash(hash).await
    }

    async fn get_block_access_list_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.inner.get_block_access_list_by_number(number).await
    }

    async fn get_block_access_list_raw(&self, block: BlockId) -> TransportResult<Option<Bytes>> {
        self.inner.get_block_access_list_raw(block).await
    }

    async fn get_header(&self, block: BlockId) -> TransportResult<Option<N::HeaderResponse>> {
        self.inner.get_header(block).await
    }

    async fn get_header_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<N::HeaderResponse>> {
        self.inner.get_header_by_hash(hash).await
    }

    async fn get_header_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<Option<N::HeaderResponse>> {
        self.inner.get_header_by_number(number).await
    }

    fn get_code_at(&self, address: Address) -> RpcWithBlock<Address, Bytes> {
        self.inner.get_code_at(address).block_id(self.block_id)
    }

    async fn watch_blocks(&self) -> TransportResult<FilterPollerBuilder<B256>> {
        self.inner.watch_blocks().await
    }

    async fn watch_pending_transactions(&self) -> TransportResult<FilterPollerBuilder<B256>> {
        self.inner.watch_pending_transactions().await
    }

    async fn watch_logs(&self, filter: &Filter) -> TransportResult<FilterPollerBuilder<Log>> {
        self.inner.watch_logs(filter).await
    }

    async fn watch_full_pending_transactions(
        &self,
    ) -> TransportResult<FilterPollerBuilder<N::TransactionResponse>> {
        self.inner.watch_full_pending_transactions().await
    }

    async fn get_filter_changes_dyn(&self, id: U256) -> TransportResult<FilterChanges> {
        self.inner.get_filter_changes_dyn(id).await
    }

    async fn get_filter_logs(&self, id: U256) -> TransportResult<Vec<Log>> {
        self.inner.get_filter_logs(id).await
    }

    async fn uninstall_filter(&self, id: U256) -> TransportResult<bool> {
        self.inner.uninstall_filter(id).await
    }

    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> Result<PendingTransaction, PendingTransactionError> {
        self.inner.watch_pending_transaction(config).await
    }

    async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
        self.inner.get_logs(filter).await
    }

    fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
    ) -> RpcWithBlock<(Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        self.inner.get_proof(address, keys).block_id(self.block_id)
    }

    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> RpcWithBlock<(Address, U256), StorageValue> {
        self.inner.get_storage_at(address, key).block_id(self.block_id)
    }

    fn get_storage_values(
        &self,
        requests: StorageValuesRequest,
    ) -> RpcWithBlock<(StorageValuesRequest,), StorageValuesResponse> {
        self.inner.get_storage_values(requests).block_id(self.block_id)
    }

    fn get_transaction_by_hash(
        &self,
        hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::TransactionResponse>> {
        self.inner.get_transaction_by_hash(hash)
    }

    fn get_transaction_by_sender_nonce(
        &self,
        sender: Address,
        nonce: u64,
    ) -> ProviderCall<(Address, U64), Option<N::TransactionResponse>> {
        self.inner.get_transaction_by_sender_nonce(sender, nonce)
    }

    fn get_transaction_by_block_hash_and_index(
        &self,
        block_hash: B256,
        index: usize,
    ) -> ProviderCall<(B256, Index), Option<N::TransactionResponse>> {
        self.inner.get_transaction_by_block_hash_and_index(block_hash, index)
    }

    fn get_raw_transaction_by_block_hash_and_index(
        &self,
        block_hash: B256,
        index: usize,
    ) -> ProviderCall<(B256, Index), Option<Bytes>> {
        self.inner.get_raw_transaction_by_block_hash_and_index(block_hash, index)
    }

    fn get_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: usize,
    ) -> ProviderCall<(BlockNumberOrTag, Index), Option<N::TransactionResponse>> {
        self.inner.get_transaction_by_block_number_and_index(block_number, index)
    }

    fn get_raw_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: usize,
    ) -> ProviderCall<(BlockNumberOrTag, Index), Option<Bytes>> {
        self.inner.get_raw_transaction_by_block_number_and_index(block_number, index)
    }

    fn get_raw_transaction_by_hash(&self, hash: TxHash) -> ProviderCall<(TxHash,), Option<Bytes>> {
        self.inner.get_raw_transaction_by_hash(hash)
    }

    fn get_transaction_count(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64> {
        self.inner.get_transaction_count(address).block_id(self.block_id)
    }

    fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::ReceiptResponse>> {
        self.inner.get_transaction_receipt(hash)
    }

    async fn get_uncle(&self, tag: BlockId, idx: u64) -> TransportResult<Option<N::BlockResponse>> {
        self.inner.get_uncle(tag, idx).await
    }

    async fn get_uncle_count(&self, tag: BlockId) -> TransportResult<u64> {
        self.inner.get_uncle_count(tag).await
    }

    fn get_max_priority_fee_per_gas(&self) -> ProviderCall<NoParams, U128, u128> {
        self.inner.get_max_priority_fee_per_gas()
    }

    async fn new_block_filter(&self) -> TransportResult<U256> {
        self.inner.new_block_filter().await
    }

    async fn new_filter(&self, filter: &Filter) -> TransportResult<U256> {
        self.inner.new_filter(filter).await
    }

    async fn new_pending_transactions_filter(&self, full: bool) -> TransportResult<U256> {
        self.inner.new_pending_transactions_filter(full).await
    }

    async fn send_raw_transaction(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_raw_transaction(encoded_tx).await
    }

    async fn send_raw_transaction_sync(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<N::ReceiptResponse> {
        self.inner.send_raw_transaction_sync(encoded_tx).await
    }

    async fn send_raw_transaction_conditional(
        &self,
        encoded_tx: &[u8],
        conditional: TransactionConditional,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_raw_transaction_conditional(encoded_tx, conditional).await
    }

    async fn send_transaction_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_transaction_internal(tx).await
    }

    async fn send_transaction_sync_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<N::ReceiptResponse> {
        self.inner.send_transaction_sync_internal(tx).await
    }

    async fn sign_transaction(&self, tx: N::TransactionRequest) -> TransportResult<Bytes> {
        self.inner.sign_transaction(tx).await
    }

    async fn fill_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<FillTransaction<N::TxEnvelope>>
    where
        N::TxEnvelope: RpcRecv,
    {
        self.inner.fill_transaction(tx).await
    }

    async fn fill_and_sign_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::TxEnvelope> {
        self.inner.fill_and_sign_transaction(tx).await
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_blocks(&self) -> GetSubscription<(SubscriptionKind,), N::HeaderResponse> {
        self.inner.subscribe_blocks()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_pending_transactions(&self) -> GetSubscription<(SubscriptionKind,), B256> {
        self.inner.subscribe_pending_transactions()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_full_pending_transactions(
        &self,
    ) -> GetSubscription<(SubscriptionKind, Params), N::TransactionResponse> {
        self.inner.subscribe_full_pending_transactions()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_logs(&self, filter: &Filter) -> GetSubscription<(SubscriptionKind, Params), Log> {
        self.inner.subscribe_logs(filter)
    }

    #[cfg(feature = "pubsub")]
    async fn unsubscribe(&self, id: B256) -> TransportResult<()> {
        self.inner.unsubscribe(id).await
    }

    fn syncing(&self) -> ProviderCall<NoParams, SyncStatus> {
        self.inner.syncing()
    }

    fn get_client_version(&self) -> ProviderCall<NoParams, String> {
        self.inner.get_client_version()
    }

    fn get_sha3(&self, data: &[u8]) -> ProviderCall<(String,), B256> {
        self.inner.get_sha3(data)
    }

    fn get_net_version(&self) -> ProviderCall<NoParams, U64, u64> {
        self.inner.get_net_version()
    }

    async fn raw_request_dyn(
        &self,
        method: Cow<'static, str>,
        params: &RawValue,
    ) -> TransportResult<Box<RawValue>> {
        self.inner.raw_request_dyn(method, params).await
    }

    fn transaction_request(&self) -> N::TransactionRequest {
        self.inner.transaction_request()
    }
}

#[cfg(test)]
mod tests {
    use super::BlockIdProvider;
    use crate::{Provider, ProviderCall, RootProvider};
    use alloy_eips::BlockId;
    use alloy_network::Ethereum;
    use alloy_primitives::{Address, BlockNumber, U128, U64};
    use alloy_rpc_client::{NoParams, RpcClient};
    use alloy_rpc_types_eth::{Filter, Log};
    use alloy_transport::{mock::Asserter, TransportResult};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    /// An inner provider that records which of its methods are reached, so we can assert that the
    /// `BlockIdProvider` wrapper forwards non-block methods instead of bypassing it.
    #[derive(Clone, Debug)]
    struct Recorder {
        inner: RootProvider<Ethereum>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Recorder {
        fn record(&self, method: &'static str) {
            self.calls.lock().unwrap().push(method);
        }
    }

    #[async_trait]
    impl Provider<Ethereum> for Recorder {
        fn root(&self) -> &RootProvider<Ethereum> {
            self.inner.root()
        }

        fn get_accounts(&self) -> ProviderCall<NoParams, Vec<Address>> {
            self.record("get_accounts");
            self.inner.get_accounts()
        }

        fn get_blob_base_fee(&self) -> ProviderCall<NoParams, U128, u128> {
            self.record("get_blob_base_fee");
            self.inner.get_blob_base_fee()
        }

        fn get_block_number(&self) -> ProviderCall<NoParams, U64, BlockNumber> {
            self.record("get_block_number");
            self.inner.get_block_number()
        }

        fn get_chain_id(&self) -> ProviderCall<NoParams, U64, u64> {
            self.record("get_chain_id");
            self.inner.get_chain_id()
        }

        async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
            self.record("get_logs");
            self.inner.get_logs(filter).await
        }
    }

    #[tokio::test]
    async fn forwards_non_block_methods_to_inner() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recorder = Recorder {
            inner: RootProvider::new(RpcClient::mocked(Asserter::new())),
            calls: calls.clone(),
        };
        let provider = BlockIdProvider::new(recorder, BlockId::latest());

        // The empty mock fails every request; only the fact that the call reached the inner
        // recorder matters.
        let _ = provider.get_accounts().await;
        let _ = provider.get_blob_base_fee().await;
        let _ = provider.get_block_number().await;
        let _ = provider.get_chain_id().await;
        let _ = provider.get_logs(&Filter::default()).await;

        assert_eq!(
            *calls.lock().unwrap(),
            ["get_accounts", "get_blob_base_fee", "get_block_number", "get_chain_id", "get_logs"]
        );
    }
}
