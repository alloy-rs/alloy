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
//! two fillers, left and right. Ready fillers prepare concurrently, then apply their prepared
//! values from left to right. Synchronous filling also runs from left to right.
//!
//! [`Provider`]: crate::Provider

mod chain_id;
pub use chain_id::ChainIdFiller;

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

use crate::{
    provider::{forward, SendableTx},
    EthCall, Identity, PendingTransactionBuilder, Provider, ProviderLayer, SendableTxErr,
};
use alloy_json_rpc::RpcError;
use alloy_network::{AnyNetwork, Ethereum, Network};
use alloy_primitives::{Bytes, U64};
use alloy_transport::{TransportError, TransportResult};
use futures_utils_wasm::impl_future;
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

    /// Returns the filler names and their missing properties if this is [`Self::Missing`],
    /// or `None` if this is [`Self::Ready`] or [`Self::Finished`].
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
    /// synchronously.
    ///
    /// Errors stop preparation and are returned when the call or gas estimate is awaited.
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
                    ?tx, ?self.filler, loop_iterations = count,
                    ERROR
                );
                return Err(TransportError::local_usage_str(ERROR));
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
    /// # use alloy_primitives::{Address, U256};
    /// # use alloy_provider::{Provider, ProviderBuilder};
    /// # use alloy_rpc_types_eth::TransactionRequest;
    /// # use alloy_network::{NetworkTransactionBuilder, TransactionBuilder};
    ///
    /// # #[cfg(feature = "anvil-node")]
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Do not add a wallet: this example needs the filled request, not a signed envelope.
    ///     let provider = ProviderBuilder::new().connect_anvil();
    ///     let from = provider
    ///         .get_accounts()
    ///         .await?
    ///         .into_iter()
    ///         .next()
    ///         .expect("Anvil provides funded accounts");
    ///
    ///     let tx_request = TransactionRequest::default()
    ///         .with_from(from)
    ///         .with_to(Address::ZERO)
    ///         .with_value(U256::from(1000));
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
    /// # #[cfg(not(feature = "anvil-node"))]
    /// # fn example() {}
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

impl<F, P, N> forward::ForwardingProvider<N> for FillProvider<F, P, N>
where
    F: TxFiller<N>,
    P: Provider<N>,
    N: Network,
{
    type Inner = P;

    #[inline]
    fn inner_provider(&self) -> &P {
        &self.inner
    }

    // `send_transaction`, `send_tx_envelope` and `send_transaction_sync` keep the trait defaults,
    // which go through `self.send_transaction_internal` / `self.send_transaction_sync_internal`
    // and therefore through the fillers. Forwarding them would skip the fillers.

    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        Provider::send_transaction_internal(self, SendableTx::Builder(tx)).await
    }

    async fn send_tx_envelope(
        &self,
        tx: N::TxEnvelope,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        Provider::send_transaction_internal(self, SendableTx::Envelope(tx)).await
    }

    async fn send_transaction_sync(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::ReceiptResponse> {
        Provider::send_transaction_sync_internal(self, SendableTx::Builder(tx)).await
    }

    fn call<'req>(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        let mut tx = tx;
        if let Err(error) = self.filler.prepare_call_sync(&mut tx) {
            return EthCall::error(error);
        }
        self.inner.call(tx)
    }

    fn estimate_gas<'req>(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        let mut tx = tx;
        if let Err(error) = self.filler.prepare_call_sync(&mut tx) {
            return EthCall::error(error).map_resp(crate::utils::convert_u64);
        }
        self.inner.estimate_gas(tx)
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

    async fn fill_and_sign_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::TxEnvelope> {
        match self.fill(tx).await? {
            SendableTx::Envelope(envelope) => Ok(envelope),
            SendableTx::Builder(tx) => {
                if let FillerControlFlow::Missing(missing) = self.filler.status(&tx) {
                    let message = format!("missing properties: {missing:?}");
                    return Err(RpcError::local_usage_str(&message));
                }
                Err(RpcError::local_usage_str(
                    "no wallet configured, fillers did not produce a signed transaction",
                ))
            }
        }
    }
}

forward::impl_provider_forwarding!(
    impl[F, P, N] Provider<N> for FillProvider<F, P, N>
    where [F: TxFiller<N>, P: Provider<N>, N: Network]
);

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

#[cfg(test)]
mod forwarding_tests {
    use super::*;
    use crate::{ProviderBuilder, RootProvider};
    use alloy_consensus::TxEnvelope;
    use alloy_eips::{eip7928::BlockAccessList, BlockId, BlockNumberOrTag};
    use alloy_primitives::{BlockHash, B256};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::{FillTransaction, TransactionReceipt, TransactionRequest};
    use alloy_transport::mock::Asserter;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    /// An inner layer that records which of its methods are reached.
    #[derive(Clone, Debug)]
    struct Recorder<P> {
        inner: P,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl<P> Recorder<P> {
        fn record(&self, method: &'static str) {
            self.calls.lock().unwrap().push(method);
        }
    }

    #[async_trait]
    impl<P: Provider> Provider for Recorder<P> {
        fn root(&self) -> &RootProvider {
            self.inner.root()
        }

        async fn send_raw_transaction_sync(
            &self,
            encoded_tx: &[u8],
        ) -> TransportResult<TransactionReceipt> {
            self.record("send_raw_transaction_sync");
            self.inner.send_raw_transaction_sync(encoded_tx).await
        }

        async fn get_block_access_list(
            &self,
            block: BlockId,
        ) -> TransportResult<Option<BlockAccessList>> {
            self.record("get_block_access_list");
            self.inner.get_block_access_list(block).await
        }

        async fn get_block_access_list_by_hash(
            &self,
            hash: BlockHash,
        ) -> TransportResult<Option<BlockAccessList>> {
            self.record("get_block_access_list_by_hash");
            self.inner.get_block_access_list_by_hash(hash).await
        }

        async fn get_block_access_list_by_number(
            &self,
            number: BlockNumberOrTag,
        ) -> TransportResult<Option<BlockAccessList>> {
            self.record("get_block_access_list_by_number");
            self.inner.get_block_access_list_by_number(number).await
        }

        async fn get_block_access_list_raw(
            &self,
            block: BlockId,
        ) -> TransportResult<Option<Bytes>> {
            self.record("get_block_access_list_raw");
            self.inner.get_block_access_list_raw(block).await
        }

        async fn fill_transaction(
            &self,
            tx: TransactionRequest,
        ) -> TransportResult<FillTransaction<TxEnvelope>> {
            self.record("fill_transaction");
            self.inner.fill_transaction(tx).await
        }
    }

    #[tokio::test]
    async fn forwards_to_inner_layers() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let inner = Recorder {
            inner: RootProvider::new(RpcClient::mocked(Asserter::new())),
            calls: calls.clone(),
        };
        let provider = ProviderBuilder::new().connect_provider(inner);

        // The empty mock fails every request, only the calls reaching the inner layer matter.
        let _ = provider.send_raw_transaction_sync(&[]).await;
        let _ = provider.get_block_access_list(BlockId::latest()).await;
        let _ = provider.get_block_access_list_by_hash(B256::ZERO).await;
        let _ = provider.get_block_access_list_by_number(BlockNumberOrTag::Latest).await;
        let _ = provider.get_block_access_list_raw(BlockId::latest()).await;
        let _ = provider.fill_transaction(TransactionRequest::default()).await;

        assert_eq!(
            *calls.lock().unwrap(),
            [
                "send_raw_transaction_sync",
                "get_block_access_list",
                "get_block_access_list_by_hash",
                "get_block_access_list_by_number",
                "get_block_access_list_raw",
                "fill_transaction",
            ]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RootProvider;
    use alloy_rpc_types_eth::TransactionRequest;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone, Debug, Default)]
    struct LoopingFiller(Arc<AtomicUsize>);

    impl TxFiller for LoopingFiller {
        type Fillable = ();

        fn status(&self, _tx: &TransactionRequest) -> FillerControlFlow {
            FillerControlFlow::Ready
        }

        fn fill_sync(&self, _tx: &mut SendableTx<Ethereum>) {}

        async fn prepare<P: Provider>(
            &self,
            _provider: &P,
            _tx: &TransactionRequest,
        ) -> TransportResult<()> {
            Ok(())
        }

        async fn fill(
            &self,
            (): (),
            tx: SendableTx<Ethereum>,
        ) -> TransportResult<SendableTx<Ethereum>> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(tx)
        }
    }

    #[derive(Clone, Debug)]
    struct RejectingCallFiller;

    impl TxFiller for RejectingCallFiller {
        type Fillable = ();

        fn status(&self, _tx: &TransactionRequest) -> FillerControlFlow {
            FillerControlFlow::Finished
        }

        fn fill_sync(&self, _tx: &mut SendableTx<Ethereum>) {}

        async fn prepare<P: Provider>(
            &self,
            _provider: &P,
            _tx: &TransactionRequest,
        ) -> TransportResult<()> {
            Ok(())
        }

        async fn fill(
            &self,
            (): (),
            tx: SendableTx<Ethereum>,
        ) -> TransportResult<SendableTx<Ethereum>> {
            Ok(tx)
        }

        fn prepare_call_sync(&self, tx: &mut TransactionRequest) -> TransportResult<()> {
            tx.gas = Some(123);
            Err(RpcError::ErrorResp(alloy_json_rpc::ErrorPayload::invalid_request()))
        }
    }

    #[tokio::test]
    async fn call_preparation_errors_prevent_dispatch() {
        let asserter = alloy_transport::mock::Asserter::new();
        let inner =
            RootProvider::<Ethereum>::new(alloy_rpc_client::RpcClient::mocked(asserter.clone()));
        let provider = FillProvider::new(inner, RejectingCallFiller);

        let call = provider.call(TransactionRequest::default()).latest();
        for error in [
            call.clone().await.unwrap_err(),
            call.await.unwrap_err(),
            provider.estimate_gas(TransactionRequest::default()).await.unwrap_err(),
        ] {
            assert_eq!(error.as_error_resp().unwrap().code, -32600);
        }
        assert!(asserter.requests().is_empty());
    }

    #[tokio::test]
    async fn looping_filler_returns_local_usage_error() {
        let filler = LoopingFiller::default();
        let calls = filler.0.clone();
        let inner = RootProvider::<Ethereum>::new(alloy_rpc_client::RpcClient::mocked(
            alloy_transport::mock::Asserter::new(),
        ));
        let provider = FillProvider::new(inner, filler);
        let err = provider.fill(TransactionRequest::default()).await.unwrap_err();

        assert!(matches!(err, RpcError::LocalUsageError(_)));
        assert!(err.to_string().contains("Tx filler loop detected"));
        assert_eq!(calls.load(Ordering::Relaxed), 20);
    }
}
