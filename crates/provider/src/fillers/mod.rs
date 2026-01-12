//! Transaction fillers.
//!
//! Fillers decorate a [`Provider`], filling transaction details before they
//! are sent to the network, like nonces, gas limits, and gas prices.
//!
//! Fillers are called before any other layer in the provider.
//!
//! # Implementing a filler
//!
//! Fillers implement the [`TxFiller`] trait. Before a filler is called, [`TxFiller::status`] is
//! called to determine whether the filler has any work to do. If this function returns
//! [`FillerControlFlow::Ready`], the filler will be called.
//!
//! # Composing fillers
//!
//! To layer fillers, a utility filler is provided called [`JoinFill`], which is a composition of
//! two fillers, left and right. The left filler is called before the right filler.
//!
//! [`Provider`]: crate::Provider

mod chain_id;
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::{
    Address, BlockHash, BlockNumber, StorageKey, StorageValue, TxHash, B256, U128, U256,
};
use alloy_rpc_client::NoParams;
#[cfg(feature = "pubsub")]
use alloy_rpc_types_eth::pubsub::{Params, SubscriptionKind};
use alloy_rpc_types_eth::{Bundle, Index, SyncStatus};
pub use chain_id::ChainIdFiller;
use std::borrow::Cow;

mod wallet;
pub use wallet::WalletFiller;

mod nonce;
pub use nonce::{CachedNonceManager, NonceFiller, NonceManager, SimpleNonceManager};

mod gas;
pub use gas::{
    BlobGasEstimator, BlobGasEstimatorFn, BlobGasEstimatorFunction, BlobGasFiller, GasFillable,
    GasFiller,
};

mod join_fill;
pub use join_fill::JoinFill;
use tracing::error;

#[cfg(feature = "pubsub")]
use crate::GetSubscription;
use crate::{
    provider::SendableTx, EthCall, EthCallMany, EthGetBlock, FilterPollerBuilder, Identity,
    PendingTransaction, PendingTransactionBuilder, PendingTransactionConfig,
    PendingTransactionError, Provider, ProviderCall, ProviderLayer, RootProvider, RpcWithBlock,
    SendableTxErr,
};
use alloy_json_rpc::RpcError;
use alloy_network::{AnyNetwork, Ethereum, Network};
use alloy_primitives::{Bytes, U64};
use alloy_rpc_types_eth::{
    erc4337::TransactionConditional,
    simulate::{SimulatePayload, SimulatedBlock},
    AccessListResult, EIP1186AccountProofResponse, EthCallResponse, FeeHistory, Filter,
    FilterChanges, Log,
};
use alloy_transport::{TransportError, TransportResult};
use async_trait::async_trait;
use futures_utils_wasm::impl_future;
use serde_json::value::RawValue;
use std::marker::PhantomData;

/// The recommended filler, a preconfigured set of layers handling gas estimation, nonce
/// management, and chain-id fetching.
pub type RecommendedFiller =
    JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>;

/// Error type for failures in the `fill_envelope` function.
#[derive(Debug, thiserror::Error)]
pub enum FillEnvelopeError<T> {
    /// A transport error occurred during the filling process.
    #[error("transport error during filling: {0}")]
    Transport(TransportError),

    /// The transaction is not ready to be converted to an envelope.
    #[error("transaction not ready: {0}")]
    NotReady(SendableTxErr<T>),
}

/// The control flow for a filler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FillerControlFlow {
    /// The filler is missing a required property.
    ///
    /// To allow joining fillers while preserving their associated missing
    /// lists, this variant contains a list of `(name, missing)` tuples. When
    /// absorbing another control flow, if both are missing, the missing lists
    /// are combined.
    Missing(Vec<(&'static str, Vec<&'static str>)>),
    /// The filler is ready to fill in the transaction request.
    Ready,
    /// The filler has filled in all properties that it can fill.
    Finished,
}

impl FillerControlFlow {
    /// Absorb the control flow of another filler.
    ///
    /// # Behavior:
    /// - If either is finished, return the unfinished one
    /// - If either is ready, return ready.
    /// - If both are missing, return missing.
    pub fn absorb(self, other: Self) -> Self {
        if other.is_finished() {
            return self;
        }

        if self.is_finished() {
            return other;
        }

        if other.is_ready() || self.is_ready() {
            return Self::Ready;
        }

        if let (Self::Missing(mut a), Self::Missing(b)) = (self, other) {
            a.extend(b);
            return Self::Missing(a);
        }

        unreachable!()
    }

    /// Creates a new `Missing` control flow.
    pub fn missing(name: &'static str, missing: Vec<&'static str>) -> Self {
        Self::Missing(vec![(name, missing)])
    }

    /// Returns true if the filler is missing a required property.
    pub fn as_missing(&self) -> Option<&[(&'static str, Vec<&'static str>)]> {
        match self {
            Self::Missing(missing) => Some(missing),
            _ => None,
        }
    }

    /// Returns `true` if the filler is missing information required to fill in
    /// the transaction request.
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(_))
    }

    /// Returns `true` if the filler is ready to fill in the transaction
    /// request.
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Returns `true` if the filler is finished filling in the transaction
    /// request.
    pub const fn is_finished(&self) -> bool {
        matches!(self, Self::Finished)
    }
}

/// A layer that can fill in a `TransactionRequest` with additional information.
///
/// ## Lifecycle Notes
///
/// The [`FillerControlFlow`] determines the lifecycle of a filler. Fillers
/// may be in one of three states:
/// - **Missing**: The filler is missing a required property to fill in the transaction request.
///   [`TxFiller::status`] should return [`FillerControlFlow::Missing`]. with a list of the missing
///   properties.
/// - **Ready**: The filler is ready to fill in the transaction request. [`TxFiller::status`] should
///   return [`FillerControlFlow::Ready`].
/// - **Finished**: The filler has filled in all properties that it can fill. [`TxFiller::status`]
///   should return [`FillerControlFlow::Finished`].
#[doc(alias = "TransactionFiller")]
pub trait TxFiller<N: Network = Ethereum>: Clone + Send + Sync + std::fmt::Debug {
    /// The properties that this filler retrieves from the RPC. to fill in the
    /// TransactionRequest.
    type Fillable: Send + Sync + 'static;

    /// Joins this filler with another filler to compose multiple fillers.
    fn join_with<T>(self, other: T) -> JoinFill<Self, T>
    where
        T: TxFiller<N>,
    {
        JoinFill::new(self, other)
    }

    /// Return a control-flow enum indicating whether the filler is ready to
    /// fill in the transaction request, or if it is missing required
    /// properties.
    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow;

    /// Returns `true` if the filler should continue filling.
    fn continue_filling(&self, tx: &SendableTx<N>) -> bool {
        tx.as_builder().is_some_and(|tx| self.status(tx).is_ready())
    }

    /// Returns `true` if the filler is ready to fill in the transaction request.
    fn ready(&self, tx: &N::TransactionRequest) -> bool {
        self.status(tx).is_ready()
    }

    /// Returns `true` if the filler is finished filling in the transaction request.
    fn finished(&self, tx: &N::TransactionRequest) -> bool {
        self.status(tx).is_finished()
    }

    /// Performs any synchronous filling. This should be called before
    /// [`TxFiller::prepare`] and [`TxFiller::fill`] to fill in any properties
    /// that can be filled synchronously.
    fn fill_sync(&self, tx: &mut SendableTx<N>);

    /// Prepares fillable properties, potentially by making an RPC request.
    fn prepare<P: Provider<N>>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> impl_future!(<Output = TransportResult<Self::Fillable>>);

    /// Fills in the transaction request with the fillable properties.
    fn fill(
        &self,
        fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> impl_future!(<Output = TransportResult<SendableTx<N>>>);

    /// Fills in the transaction request and try to convert it to an envelope.
    fn fill_envelope(
        &self,
        fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> impl_future!(<Output = Result<N::TxEnvelope, FillEnvelopeError<N::TransactionRequest>>>)
    {
        async move {
            let tx = self.fill(fillable, tx).await.map_err(FillEnvelopeError::Transport)?;
            let envelope = tx.try_into_envelope().map_err(FillEnvelopeError::NotReady)?;
            Ok(envelope)
        }
    }

    /// Prepares and fills the transaction request with the fillable properties.
    fn prepare_and_fill<P>(
        &self,
        provider: &P,
        tx: SendableTx<N>,
    ) -> impl_future!(<Output = TransportResult<SendableTx<N>>>)
    where
        P: Provider<N>,
    {
        async move {
            if tx.is_envelope() {
                return Ok(tx);
            }

            let fillable =
                self.prepare(provider, tx.as_builder().expect("checked by is_envelope")).await?;

            self.fill(fillable, tx).await
        }
    }

    /// Prepares transaction request with necessary fillers required for eth_call operations
    /// asynchronously
    fn prepare_call(
        &self,
        tx: &mut N::TransactionRequest,
    ) -> impl_future!(<Output = TransportResult<()>>) {
        let _ = tx;
        // This is a no-op by default
        futures::future::ready(Ok(()))
    }

    /// Prepares transaction request with necessary fillers required for eth_call operations
    /// synchronously
    fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        let _ = tx;
        // No-op default
        Ok(())
    }
}

/// A [`Provider`] that applies one or more [`TxFiller`]s.
///
/// Fills arbitrary properties in a transaction request by composing multiple
/// fill layers. This struct should always be the outermost layer in a provider
/// stack, and this is enforced when using [`ProviderBuilder::filler`] to
/// construct this layer.
///
/// Users should NOT use this struct directly. Instead, use
/// [`ProviderBuilder::filler`] to construct and apply it to a stack.
///
/// [`ProviderBuilder::filler`]: crate::ProviderBuilder::filler
#[derive(Clone, Debug)]
pub struct FillProvider<F, P, N = Ethereum>
where
    F: TxFiller<N>,
    P: Provider<N>,
    N: Network,
{
    pub(crate) inner: P,
    pub(crate) filler: F,
    _pd: PhantomData<fn() -> N>,
}

impl<F, P, N> FillProvider<F, P, N>
where
    F: TxFiller<N>,
    P: Provider<N>,
    N: Network,
{
    /// Creates a new `FillProvider` with the given filler and inner provider.
    pub fn new(inner: P, filler: F) -> Self {
        Self { inner, filler, _pd: PhantomData }
    }

    /// Returns a reference to the filler.
    pub const fn filler(&self) -> &F {
        &self.filler
    }

    /// Returns a mutable reference to the filler.
    pub const fn filler_mut(&mut self) -> &mut F {
        &mut self.filler
    }

    /// Returns a reference to the inner provider.
    pub const fn inner(&self) -> &P {
        &self.inner
    }

    /// Returns a mutable reference to the inner provider.
    pub const fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Joins a filler to this provider
    pub fn join_with<Other: TxFiller<N>>(
        self,
        other: Other,
    ) -> FillProvider<JoinFill<F, Other>, P, N> {
        self.filler.join_with(other).layer(self.inner)
    }

    async fn fill_inner(&self, mut tx: SendableTx<N>) -> TransportResult<SendableTx<N>> {
        let mut count = 0;

        while self.filler.continue_filling(&tx) {
            self.filler.fill_sync(&mut tx);
            tx = self.filler.prepare_and_fill(&self.inner, tx).await?;

            count += 1;
            if count >= 20 {
                const ERROR: &str = "Tx filler loop detected. This indicates a bug in some filler implementation. Please file an issue containing this message.";
                error!(
                    ?tx, ?self.filler,
                    ERROR
                );
                panic!("{}, {:?}, {:?}", ERROR, &tx, &self.filler);
            }
        }
        Ok(tx)
    }

    /// Fills the transaction request, using the configured fillers
    ///
    /// # Example
    ///
    /// ```rust
    /// # use alloy_consensus::{TypedTransaction, SignableTransaction};
    /// # use alloy_primitives::{address, U256};
    /// # use alloy_provider::ProviderBuilder;
    /// # use alloy_rpc_types_eth::TransactionRequest;
    /// # use alloy_network::TransactionBuilder;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Create transaction request
    ///     let tx_request = TransactionRequest::default()
    ///         .with_from(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
    ///         .with_value(U256::from(1000));
    ///
    ///     let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    ///
    ///     // Fill transaction with provider data
    ///     let filled_tx = provider.fill(tx_request).await?;
    ///
    ///     // Build unsigned transaction
    ///     let typed_tx =
    ///         filled_tx.as_builder().expect("filled tx is a builder").clone().build_unsigned()?;
    ///
    ///     // Encode, e.g. for offline signing
    ///     let mut encoded = Vec::new();
    ///     typed_tx.encode_for_signing(&mut encoded);
    ///
    ///     // Decode unsigned transaction
    ///     let decoded = TypedTransaction::decode_unsigned(&mut encoded.as_slice())?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fill(&self, tx: N::TransactionRequest) -> TransportResult<SendableTx<N>> {
        self.fill_inner(SendableTx::Builder(tx)).await
    }

    /// Prepares a transaction request for eth_call operations using the configured fillers
    pub fn prepare_call(
        &self,
        mut tx: N::TransactionRequest,
    ) -> TransportResult<N::TransactionRequest> {
        self.filler.prepare_call_sync(&mut tx)?;
        Ok(tx)
    }
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl<F, P, N> Provider<N> for FillProvider<F, P, N>
where
    F: TxFiller<N>,
    P: Provider<N>,
    N: Network,
{
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

    fn call<'req>(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        let mut tx = tx;
        let _ = self.filler.prepare_call_sync(&mut tx);
        self.inner.call(tx)
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
        self.inner.simulate(payload)
    }

    fn get_chain_id(&self) -> ProviderCall<NoParams, U64, u64> {
        self.inner.get_chain_id()
    }

    fn create_access_list<'a>(
        &self,
        request: &'a N::TransactionRequest,
    ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult> {
        self.inner.create_access_list(request)
    }

    fn estimate_gas<'req>(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        let mut tx = tx;
        let _ = self.filler.prepare_call_sync(&mut tx);
        self.inner.estimate_gas(tx)
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
        self.inner.get_account_info(address)
    }

    fn get_account(&self, address: Address) -> RpcWithBlock<Address, alloy_consensus::TrieAccount> {
        self.inner.get_account(address)
    }

    fn get_balance(&self, address: Address) -> RpcWithBlock<Address, U256, U256> {
        self.inner.get_balance(address)
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

    fn get_code_at(&self, address: Address) -> RpcWithBlock<Address, Bytes> {
        self.inner.get_code_at(address)
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
        self.inner.get_proof(address, keys)
    }

    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> RpcWithBlock<(Address, U256), StorageValue> {
        self.inner.get_storage_at(address, key)
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
        self.inner.get_transaction_count(address)
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

    async fn send_raw_transaction_conditional(
        &self,
        encoded_tx: &[u8],
        conditional: TransactionConditional,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_raw_transaction_conditional(encoded_tx, conditional).await
    }

    async fn send_transaction_internal(
        &self,
        mut tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        tx = self.fill_inner(tx).await?;

        if let Some(builder) = tx.as_builder() {
            if let FillerControlFlow::Missing(missing) = self.filler.status(builder) {
                // TODO: improve this.
                // blocked by #431
                let message = format!("missing properties: {missing:?}");
                return Err(RpcError::local_usage_str(&message));
            }
        }

        // Errors in tx building happen further down the stack.
        self.inner.send_transaction_internal(tx).await
    }

    async fn send_transaction_sync_internal(
        &self,
        mut tx: SendableTx<N>,
    ) -> TransportResult<N::ReceiptResponse> {
        tx = self.fill_inner(tx).await?;

        if let Some(builder) = tx.as_builder() {
            if let FillerControlFlow::Missing(missing) = self.filler.status(builder) {
                let message = format!("missing properties: {missing:?}");
                return Err(RpcError::local_usage_str(&message));
            }
        }

        // Errors in tx building happen further down the stack.
        self.inner.send_transaction_sync_internal(tx).await
    }

    async fn sign_transaction(&self, tx: N::TransactionRequest) -> TransportResult<Bytes> {
        let tx = self.fill(tx).await?;
        let tx = tx.try_into_request().map_err(TransportError::local_usage)?;
        self.inner.sign_transaction(tx).await
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

/// A trait which may be used to configure default fillers for [Network] implementations.
pub trait RecommendedFillers: Network {
    /// Recommended fillers for this network.
    type RecommendedFillers: TxFiller<Self>;

    /// Returns the recommended filler for this provider.
    fn recommended_fillers() -> Self::RecommendedFillers;
}

impl RecommendedFillers for Ethereum {
    type RecommendedFillers =
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>;

    fn recommended_fillers() -> Self::RecommendedFillers {
        Default::default()
    }
}

impl RecommendedFillers for AnyNetwork {
    type RecommendedFillers =
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>;

    fn recommended_fillers() -> Self::RecommendedFillers {
        Default::default()
    }
}
