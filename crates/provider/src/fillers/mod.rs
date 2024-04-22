//! Transaction Fillers
//!
//! Fillers decorate a [`Provider`], filling transaction details before they
//! are sent to the network. Fillers are used to set the nonce, gas price, gas
//! limit, and other transaction details, and are called before any other layer.
//!
//! [`Provider`]: crate::Provider

mod chain_id;
pub use chain_id::ChainIdFiller;

mod signer;
pub use signer::SignerFiller;

mod nonce;
pub use nonce::NonceFiller;

mod gas;
pub use gas::GasFiller;

mod join_fill;
pub use join_fill::JoinFill;

use crate::{
    provider::SendableTx, PendingTransactionBuilder, Provider, ProviderLayer, RootProvider,
};
use alloy_json_rpc::RpcError;
use alloy_network::{Ethereum, Network};
use alloy_transport::{Transport, TransportResult};
use async_trait::async_trait;
use futures_utils_wasm::impl_future;
use std::marker::PhantomData;

/// The control flow for a filler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FillerControlFlow {
    /// The filler is missing a required property.
    ///
    /// To allow joining fillers while preserving their associated missing
    /// lists, this variant contains a list of `(name, missing)` tuples. When
    /// absorbing another control flow, if both are missing, the missing lists
    /// are combined.
    Missing(Vec<(&'static str, &'static [&'static str])>),
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
    pub fn missing(name: &'static str, missing: &'static [&'static str]) -> Self {
        Self::Missing(vec![(name, missing)])
    }

    /// Returns true if the filler is missing a required property.
    pub fn as_missing(&self) -> Option<&[(&'static str, &'static [&'static str])]> {
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
/// - **Missing**: The filler is missing a required property to fill in the
///  transaction request. [`TxFiller::status`] should return
/// [`FillerControlFlow::Missing`].
/// with a list of the missing properties.
/// - **Ready**: The filler is ready to fill in the transaction request.
/// [`TxFiller::status`] should return [`FillerControlFlow::Ready`].
/// - **Finished**: The filler has filled in all properties that it can fill.
/// [`TxFiller::status`] should return [`FillerControlFlow::Finished`].
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

    /// Returns `true` if the filler is should continnue filling.
    fn continue_filling(&self, tx: &SendableTx<N>) -> bool {
        tx.as_builder().map(|tx| self.status(tx).is_ready()).unwrap_or_default()
    }

    /// Returns `true` if the filler is ready to fill in the transaction request.
    fn ready(&self, tx: &N::TransactionRequest) -> bool {
        self.status(tx).is_ready()
    }

    /// Returns `true` if the filler is finished filling in the transaction request.
    fn finished(&self, tx: &N::TransactionRequest) -> bool {
        self.status(tx).is_finished()
    }

    /// Prepares fillable properties, potentially by making an RPC request.
    fn prepare<P, T>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> impl_future!(<Output = TransportResult<Self::Fillable>>)
    where
        P: Provider<T, N>,
        T: Transport + Clone;

    /// Fills in the transaction request with the fillable properties.
    fn fill(
        &self,
        fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> impl_future!(<Output = TransportResult<SendableTx<N>>>);

    /// Prepares and fills the transaction request with the fillable properties.
    fn prepare_and_fill<P, T>(
        &self,
        provider: &P,
        tx: SendableTx<N>,
    ) -> impl_future!(<Output = TransportResult<SendableTx<N>>>)
    where
        P: Provider<T, N>,
        T: Transport + Clone,
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
pub struct FillProvider<F, P, T, N>
where
    F: TxFiller<N>,
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    pub(crate) inner: P,
    pub(crate) filler: F,
    _pd: PhantomData<fn() -> (T, N)>,
}

impl<F, P, T, N> FillProvider<F, P, T, N>
where
    F: TxFiller<N>,
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    /// Creates a new `FillProvider` with the given filler and inner provider.
    pub fn new(inner: P, filler: F) -> Self {
        Self { inner, filler, _pd: PhantomData }
    }

    /// Joins a filler to this provider
    pub fn join_with<Other: TxFiller<N>>(
        self,
        other: Other,
    ) -> FillProvider<JoinFill<F, Other>, P, T, N> {
        self.filler.join_with(other).layer(self.inner)
    }

    /// Fills the transaction request, using the configured fillers
    pub async fn fill(&self, tx: N::TransactionRequest) -> TransportResult<SendableTx<N>>
    where
        N::TxEnvelope: Clone,
    {
        self.filler.prepare_and_fill(self, SendableTx::Builder(tx)).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<F, P, T, N> Provider<T, N> for FillProvider<F, P, T, N>
where
    F: TxFiller<N>,
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    fn root(&self) -> &RootProvider<T, N> {
        self.inner.root()
    }

    async fn send_transaction_internal(
        &self,
        mut tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        let mut count = 0;

        while self.filler.continue_filling(&tx) {
            tx = self.filler.prepare_and_fill(&self.inner, tx).await?;

            count += 1;
            if count >= 20 {
                panic!(
                    "Tx filler loop detected. This indicates a bug in some filler implementation. Please file an issue containing your tx filler set."
                );
            }
        }

        if let Some(builder) = tx.as_builder() {
            if let FillerControlFlow::Missing(missing) = self.filler.status(builder) {
                // TODO: improve this.
                // blocked by #431
                let message = format!("missing properties: {:?}", missing);
                return Err(RpcError::local_usage_str(&message));
            }
        }

        // Errors in tx building happen further down the stack.
        self.inner.send_transaction_internal(tx).await
    }
}
