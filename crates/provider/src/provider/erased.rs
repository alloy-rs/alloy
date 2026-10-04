#[cfg(feature = "pubsub")]
use super::get_block::SubFullBlocks;
use super::{
    EthCallMany, EthGetBlock, FilterPollerBuilder, WatchBlocks, WatchBlocksFrom,
    WatchCanonicalBlocksFrom, WatchCanonicalLogsFrom, WatchHeaders, WatchLogsFrom,
};
#[cfg(feature = "pubsub")]
use crate::GetSubscription;
use crate::{
    heart::PendingTransactionError,
    utils::{Eip1559Estimation, Eip1559Estimator},
    EthCall, PendingTransaction, PendingTransactionBuilder, PendingTransactionConfig, Provider,
    ProviderCall, RootProvider, RpcWithBlock, SendableTx,
};
use alloy_eips::eip7928::BlockAccessList;
use alloy_json_rpc::RpcRecv;
use alloy_network::{Ethereum, Network};
use alloy_primitives::{
    Address, BlockHash, BlockNumber, Bytes, StorageKey, StorageValue, TxHash, B256, U128, U256, U64,
};
use alloy_rpc_client::{ClientRef, NoParams, WeakClient};
#[cfg(feature = "pubsub")]
use alloy_rpc_types_eth::pubsub::{Params, SubscriptionKind};
use alloy_rpc_types_eth::{
    erc4337::TransactionConditional,
    simulate::{SimulatePayload, SimulatedBlock},
    AccessListResult, BlockId, BlockNumberOrTag, Bundle, EIP1186AccountProofResponse,
    EthCallResponse, FeeHistory, FillTransaction, Filter, FilterChanges, Index, Log,
    StorageValuesRequest, StorageValuesResponse, SyncStatus,
};
use alloy_transport::TransportResult;
use serde_json::value::RawValue;
use std::{borrow::Cow, sync::Arc};

/// A wrapper struct around a type erased [`Provider`].
///
/// This type will delegate all functions to the wrapped provider, with the exception of non
/// object-safe functions (e.g. functions requiring `Self: Sized`) which use the default trait
/// implementation.
///
/// This is a convenience type for `Arc<dyn Provider<N> + 'static>`.
#[derive(Clone)]
#[doc(alias = "BoxProvider")]
pub struct DynProvider<N = Ethereum>(Arc<dyn Provider<N> + 'static>);

impl<N: Network> DynProvider<N> {
    /// Creates a new [`DynProvider`] by erasing the type.
    ///
    /// This is the same as [`provider.erased()`](Provider::erased).
    pub fn new<P: Provider<N> + 'static>(provider: P) -> Self {
        Self(Arc::new(provider))
    }
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N: Network> Provider<N> for DynProvider<N> {
    fn root(&self) -> &RootProvider<N> {
        self.0.root()
    }

    fn client(&self) -> ClientRef<'_> {
        self.0.client()
    }

    fn weak_client(&self) -> WeakClient {
        self.0.weak_client()
    }

    fn erased(self) -> Self
    where
        Self: Sized + 'static,
    {
        self
    }

    fn get_accounts(&self) -> ProviderCall<NoParams, Vec<Address>> {
        self.0.get_accounts()
    }

    fn get_blob_base_fee(&self) -> ProviderCall<NoParams, U128, u128> {
        self.0.get_blob_base_fee()
    }

    fn get_block_number(&self) -> ProviderCall<NoParams, U64, BlockNumber> {
        self.0.get_block_number()
    }

    async fn get_block_number_by_id(
        &self,
        block_id: BlockId,
    ) -> TransportResult<Option<BlockNumber>> {
        self.0.get_block_number_by_id(block_id).await
    }

    fn call(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        self.0.call(tx)
    }

    fn call_many<'req>(
        &self,
        bundles: &'req [Bundle],
    ) -> EthCallMany<'req, N, Vec<Vec<EthCallResponse>>> {
        self.0.call_many(bundles)
    }

    fn simulate<'req>(
        &self,
        payload: &'req SimulatePayload,
    ) -> RpcWithBlock<&'req SimulatePayload, Vec<SimulatedBlock<N::BlockResponse>>> {
        self.0.simulate(payload)
    }

    fn get_chain_id(&self) -> ProviderCall<NoParams, U64, u64> {
        self.0.get_chain_id()
    }

    fn create_access_list<'a>(
        &self,
        request: &'a N::TransactionRequest,
    ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult> {
        self.0.create_access_list(request)
    }

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        self.0.estimate_gas(tx)
    }

    async fn estimate_eip1559_fees_with(
        &self,
        estimator: Eip1559Estimator,
    ) -> TransportResult<Eip1559Estimation> {
        self.0.estimate_eip1559_fees_with(estimator).await
    }

    async fn estimate_eip1559_fees(&self) -> TransportResult<Eip1559Estimation> {
        self.0.estimate_eip1559_fees().await
    }

    async fn get_fee_history(
        &self,
        block_count: u64,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory> {
        self.0.get_fee_history(block_count, last_block, reward_percentiles).await
    }

    fn get_gas_price(&self) -> ProviderCall<NoParams, U128, u128> {
        self.0.get_gas_price()
    }

    fn get_account_info(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, alloy_rpc_types_eth::AccountInfo> {
        self.0.get_account_info(address)
    }

    fn get_account(&self, address: Address) -> RpcWithBlock<Address, alloy_consensus::TrieAccount> {
        self.0.get_account(address)
    }

    fn get_balance(&self, address: Address) -> RpcWithBlock<Address, U256, U256> {
        self.0.get_balance(address)
    }

    fn get_block(&self, block: BlockId) -> EthGetBlock<N::BlockResponse> {
        self.0.get_block(block)
    }

    fn get_block_by_hash(&self, hash: BlockHash) -> EthGetBlock<N::BlockResponse> {
        self.0.get_block_by_hash(hash)
    }

    fn get_block_by_number(&self, number: BlockNumberOrTag) -> EthGetBlock<N::BlockResponse> {
        self.0.get_block_by_number(number)
    }

    async fn get_block_transaction_count_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<u64>> {
        self.0.get_block_transaction_count_by_hash(hash).await
    }

    async fn get_block_transaction_count_by_number(
        &self,
        block_number: BlockNumberOrTag,
    ) -> TransportResult<Option<u64>> {
        self.0.get_block_transaction_count_by_number(block_number).await
    }

    fn get_block_receipts(
        &self,
        block: BlockId,
    ) -> ProviderCall<(BlockId,), Option<Vec<N::ReceiptResponse>>> {
        self.0.get_block_receipts(block)
    }

    async fn get_block_access_list(
        &self,
        block: BlockId,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.0.get_block_access_list(block).await
    }

    async fn get_block_access_list_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.0.get_block_access_list_by_hash(hash).await
    }

    async fn get_block_access_list_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.0.get_block_access_list_by_number(number).await
    }

    async fn get_block_access_list_raw(&self, block: BlockId) -> TransportResult<Option<Bytes>> {
        self.0.get_block_access_list_raw(block).await
    }

    async fn get_header(&self, block: BlockId) -> TransportResult<Option<N::HeaderResponse>> {
        self.0.get_header(block).await
    }

    async fn get_header_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<N::HeaderResponse>> {
        self.0.get_header_by_hash(hash).await
    }

    async fn get_header_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<Option<N::HeaderResponse>> {
        self.0.get_header_by_number(number).await
    }

    fn get_code_at(&self, address: Address) -> RpcWithBlock<Address, Bytes> {
        self.0.get_code_at(address)
    }

    async fn watch_blocks(&self) -> TransportResult<FilterPollerBuilder<B256>> {
        self.0.watch_blocks().await
    }

    async fn watch_full_blocks(&self) -> TransportResult<WatchBlocks<N::BlockResponse>> {
        self.0.watch_full_blocks().await
    }

    async fn watch_headers(&self) -> TransportResult<WatchHeaders<N::HeaderResponse>> {
        self.0.watch_headers().await
    }

    async fn watch_pending_transactions(&self) -> TransportResult<FilterPollerBuilder<B256>> {
        self.0.watch_pending_transactions().await
    }

    async fn watch_logs(&self, filter: &Filter) -> TransportResult<FilterPollerBuilder<Log>> {
        self.0.watch_logs(filter).await
    }

    fn watch_blocks_from(&self, start_block: u64) -> WatchBlocksFrom<N> {
        self.0.watch_blocks_from(start_block)
    }

    fn watch_canonical_blocks_from(&self, start_block: u64) -> WatchCanonicalBlocksFrom<N> {
        self.0.watch_canonical_blocks_from(start_block)
    }

    fn watch_logs_from(&self, start_block: u64, filter: &Filter) -> WatchLogsFrom<N> {
        self.0.watch_logs_from(start_block, filter)
    }

    fn watch_canonical_logs_from(
        &self,
        start_block: u64,
        filter: &Filter,
    ) -> WatchCanonicalLogsFrom<N> {
        self.0.watch_canonical_logs_from(start_block, filter)
    }

    async fn watch_full_pending_transactions(
        &self,
    ) -> TransportResult<FilterPollerBuilder<N::TransactionResponse>> {
        self.0.watch_full_pending_transactions().await
    }

    async fn get_filter_changes_dyn(&self, id: U256) -> TransportResult<FilterChanges> {
        self.0.get_filter_changes_dyn(id).await
    }

    async fn get_filter_logs(&self, id: U256) -> TransportResult<Vec<Log>> {
        self.0.get_filter_logs(id).await
    }

    async fn uninstall_filter(&self, id: U256) -> TransportResult<bool> {
        self.0.uninstall_filter(id).await
    }

    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> Result<PendingTransaction, PendingTransactionError> {
        self.0.watch_pending_transaction(config).await
    }

    async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
        self.0.get_logs(filter).await
    }

    fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
    ) -> RpcWithBlock<(Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        self.0.get_proof(address, keys)
    }

    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> RpcWithBlock<(Address, U256), StorageValue> {
        self.0.get_storage_at(address, key)
    }

    fn get_storage_values(
        &self,
        requests: StorageValuesRequest,
    ) -> RpcWithBlock<(StorageValuesRequest,), StorageValuesResponse> {
        self.0.get_storage_values(requests)
    }

    fn get_transaction_by_hash(
        &self,
        hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::TransactionResponse>> {
        self.0.get_transaction_by_hash(hash)
    }

    fn get_transaction_by_sender_nonce(
        &self,
        sender: Address,
        nonce: u64,
    ) -> ProviderCall<(Address, U64), Option<N::TransactionResponse>> {
        self.0.get_transaction_by_sender_nonce(sender, nonce)
    }

    fn get_transaction_by_block_hash_and_index(
        &self,
        block_hash: B256,
        index: usize,
    ) -> ProviderCall<(B256, Index), Option<N::TransactionResponse>> {
        self.0.get_transaction_by_block_hash_and_index(block_hash, index)
    }

    fn get_raw_transaction_by_block_hash_and_index(
        &self,
        block_hash: B256,
        index: usize,
    ) -> ProviderCall<(B256, Index), Option<Bytes>> {
        self.0.get_raw_transaction_by_block_hash_and_index(block_hash, index)
    }

    fn get_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: usize,
    ) -> ProviderCall<(BlockNumberOrTag, Index), Option<N::TransactionResponse>> {
        self.0.get_transaction_by_block_number_and_index(block_number, index)
    }

    fn get_raw_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: usize,
    ) -> ProviderCall<(BlockNumberOrTag, Index), Option<Bytes>> {
        self.0.get_raw_transaction_by_block_number_and_index(block_number, index)
    }

    fn get_raw_transaction_by_hash(&self, hash: TxHash) -> ProviderCall<(TxHash,), Option<Bytes>> {
        self.0.get_raw_transaction_by_hash(hash)
    }

    fn get_transaction_count(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64> {
        self.0.get_transaction_count(address)
    }

    fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::ReceiptResponse>> {
        self.0.get_transaction_receipt(hash)
    }

    async fn get_uncle(&self, tag: BlockId, idx: u64) -> TransportResult<Option<N::BlockResponse>> {
        self.0.get_uncle(tag, idx).await
    }

    async fn get_uncle_count(&self, tag: BlockId) -> TransportResult<u64> {
        self.0.get_uncle_count(tag).await
    }

    fn get_max_priority_fee_per_gas(&self) -> ProviderCall<NoParams, U128, u128> {
        self.0.get_max_priority_fee_per_gas()
    }

    async fn new_block_filter(&self) -> TransportResult<U256> {
        self.0.new_block_filter().await
    }

    async fn new_filter(&self, filter: &Filter) -> TransportResult<U256> {
        self.0.new_filter(filter).await
    }

    async fn new_pending_transactions_filter(&self, full: bool) -> TransportResult<U256> {
        self.0.new_pending_transactions_filter(full).await
    }

    async fn send_raw_transaction(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.0.send_raw_transaction(encoded_tx).await
    }

    async fn send_raw_transaction_sync(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<N::ReceiptResponse> {
        self.0.send_raw_transaction_sync(encoded_tx).await
    }

    async fn send_raw_transaction_conditional(
        &self,
        encoded_tx: &[u8],
        conditional: TransactionConditional,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.0.send_raw_transaction_conditional(encoded_tx, conditional).await
    }

    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.0.send_transaction(tx).await
    }

    async fn send_tx_envelope(
        &self,
        tx: N::TxEnvelope,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.0.send_tx_envelope(tx).await
    }

    async fn send_transaction_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.0.send_transaction_internal(tx).await
    }

    async fn send_transaction_sync(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::ReceiptResponse> {
        self.0.send_transaction_sync(tx).await
    }

    async fn send_transaction_sync_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<N::ReceiptResponse> {
        self.0.send_transaction_sync_internal(tx).await
    }

    async fn sign_transaction(&self, tx: N::TransactionRequest) -> TransportResult<Bytes> {
        self.0.sign_transaction(tx).await
    }

    async fn fill_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<FillTransaction<N::TxEnvelope>>
    where
        N::TxEnvelope: RpcRecv,
    {
        self.0.fill_transaction(tx).await
    }

    async fn fill_and_sign_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::TxEnvelope> {
        self.0.fill_and_sign_transaction(tx).await
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_blocks(&self) -> GetSubscription<(SubscriptionKind,), N::HeaderResponse> {
        self.0.subscribe_blocks()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_full_blocks(&self) -> SubFullBlocks<N> {
        self.0.subscribe_full_blocks()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_pending_transactions(&self) -> GetSubscription<(SubscriptionKind,), B256> {
        self.0.subscribe_pending_transactions()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_full_pending_transactions(
        &self,
    ) -> GetSubscription<(SubscriptionKind, Params), N::TransactionResponse> {
        self.0.subscribe_full_pending_transactions()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_logs(&self, filter: &Filter) -> GetSubscription<(SubscriptionKind, Params), Log> {
        self.0.subscribe_logs(filter)
    }

    #[cfg(feature = "pubsub")]
    async fn unsubscribe(&self, id: B256) -> TransportResult<()> {
        self.0.unsubscribe(id).await
    }

    fn syncing(&self) -> ProviderCall<NoParams, SyncStatus> {
        self.0.syncing()
    }

    fn get_client_version(&self) -> ProviderCall<NoParams, String> {
        self.0.get_client_version()
    }

    fn get_sha3(&self, data: &[u8]) -> ProviderCall<(String,), B256> {
        self.0.get_sha3(data)
    }

    fn get_net_version(&self) -> ProviderCall<NoParams, U64, u64> {
        self.0.get_net_version()
    }

    async fn raw_request_dyn(
        &self,
        method: Cow<'static, str>,
        params: &RawValue,
    ) -> TransportResult<Box<RawValue>> {
        self.0.raw_request_dyn(method, params).await
    }

    fn transaction_request(&self) -> N::TransactionRequest {
        self.0.transaction_request()
    }
}

impl<N> std::fmt::Debug for DynProvider<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynProvider").field(&"<dyn Provider>").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    fn assert_provider<P: Provider + Sized + Clone + Unpin + 'static>(_: P) {}

    #[test]
    fn test_erased_provider() {
        let provider =
            ProviderBuilder::new().connect_http("http://localhost:8080".parse().unwrap()).erased();
        assert_provider(provider);
    }
}

#[cfg(test)]
mod forwarding_tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_consensus::{transaction::SignerRecoverable, Transaction, TxEnvelope};
    use alloy_eips::eip2718::Decodable2718;
    use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
    use alloy_network::TransactionBuilder;
    use alloy_primitives::{keccak256, Bloom};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_signer_local::PrivateKeySigner;
    use alloy_transport::TransportFut;
    use std::{collections::BTreeSet, sync::Mutex};

    /// Fails when the `Provider` trait gains an object-safe method that is not forwarded here.
    #[test]
    fn forwards_all_object_safe_methods() {
        fn method_names<'a>(src: &'a str, block_start: &str) -> BTreeSet<&'a str> {
            // `lines` keeps this independent of the line endings of the checkout.
            let names: BTreeSet<_> = src
                .lines()
                .skip_while(|line| !line.contains(block_start))
                .take_while(|line| *line != "}")
                .filter_map(|line| {
                    line.strip_prefix("    fn ").or_else(|| line.strip_prefix("    async fn "))
                })
                .map(|rest| rest.split(['(', '<']).next().unwrap())
                .collect();
            assert!(!names.is_empty(), "no methods found after `{block_start}`");
            names
        }

        // `Self: Sized` methods, which cannot be called through `dyn Provider`.
        let not_object_safe = [
            "builder",
            "get_filter_changes",
            "multicall",
            "raw_request",
            "subscribe",
            "subscribe_to",
        ];
        let trait_methods = method_names(include_str!("trait.rs"), "pub trait Provider<");
        let forwarded = method_names(include_str!("erased.rs"), "for DynProvider<N> {");
        let missing: Vec<_> = trait_methods
            .difference(&forwarded)
            .filter(|method| !not_object_safe.contains(method))
            .collect();
        assert!(missing.is_empty(), "DynProvider does not forward {missing:?}");
    }

    #[tokio::test]
    async fn send_transaction_sync_uses_erased_fillers_and_wallet() {
        let sent = Arc::new(Mutex::new(None));
        let service = {
            let sent = sent.clone();
            tower::service_fn(move |request: RequestPacket| {
                let sent = sent.clone();
                Box::pin(async move {
                    let RequestPacket::Single(request) = request else {
                        panic!("expected a single request");
                    };
                    let payload = match request.method() {
                        "eth_getTransactionCount" => ResponsePayload::Success(
                            RawValue::from_string("\"0x7\"".into()).unwrap(),
                        ),
                        "eth_sendRawTransactionSync" => {
                            let (raw,): (Bytes,) =
                                serde_json::from_str(request.params().unwrap().get()).unwrap();
                            let receipt = serde_json::json!({
                                "type": "0x2",
                                "status": "0x1",
                                "cumulativeGasUsed": "0x5208",
                                "logs": [],
                                "logsBloom": Bloom::ZERO,
                                "transactionHash": keccak256(&raw),
                                "gasUsed": "0x5208",
                                "effectiveGasPrice": "0x1",
                                "from": Address::ZERO,
                                "to": Address::ZERO,
                                "contractAddress": null,
                            });
                            *sent.lock().unwrap() = Some(raw);
                            ResponsePayload::Success(
                                serde_json::value::to_raw_value(&receipt).unwrap(),
                            )
                        }
                        method => ResponsePayload::internal_error_message(
                            format!("unexpected {method}").into(),
                        ),
                    };
                    Ok(ResponsePacket::Single(Response { id: request.id().clone(), payload }))
                }) as TransportFut<'static>
            })
        };

        let signer = PrivateKeySigner::random();
        let from = signer.address();
        let erased = ProviderBuilder::new()
            .wallet(signer)
            .connect_client(RpcClient::new(service, true))
            .erased();
        let provider = ProviderBuilder::default().with_chain_id(1).connect_provider(erased);

        let tx = TransactionRequest::default()
            .with_to(Address::ZERO)
            .with_gas_limit(21_000)
            .with_max_fee_per_gas(2)
            .with_max_priority_fee_per_gas(1);
        let receipt = provider.send_transaction_sync(tx).await.unwrap();

        let raw = sent.lock().unwrap().take().unwrap();
        let envelope = TxEnvelope::decode_2718(&mut raw.as_ref()).unwrap();
        assert_eq!(envelope.recover_signer().unwrap(), from);
        assert_eq!(envelope.nonce(), 7);
        assert_eq!(envelope.chain_id(), Some(1));
        assert_eq!(receipt.transaction_hash, *envelope.tx_hash());
    }
}
