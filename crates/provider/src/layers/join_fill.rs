use crate::{PendingTransactionBuilder, Provider, ProviderLayer, RootProvider};
use alloy_network::{Ethereum, Network};
use alloy_transport::{Transport, TransportResult};
use async_trait::async_trait;
use futures::try_join;
use std::{future::Future, marker::PhantomData};

/// The control flow for a filler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillerControlFlow {
    /// The filler is missing a required property.
    Missing(Vec<&'static str>),
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
        if other.is_ready() || self.is_ready() {
            return Self::Ready;
        }

        if let (Self::Missing(mut a), Self::Missing(b)) = (self, other) {
            a.extend(b);
            return Self::Missing(a);
        }
        unreachable!()
    }

    /// Returns true if the filler is missing a required property.
    pub fn as_missing(&self) -> Option<&[&'static str]> {
        match self {
            Self::Missing(missing) => Some(missing),
            _ => None,
        }
    }

    /// Returns `true` if the filler is missing information required to fill in
    /// the transaction request.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(_))
    }

    /// Returns `true` if the filler is ready to fill in the transaction
    /// request.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Returns `true` if the filler is finished filling in the transaction
    /// request.
    pub fn is_finished(&self) -> bool {
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
pub trait TxFiller<N: Network = Ethereum>: Clone + Send + Sync {
    /// The properties that this filler retrieves from the RPC. to fill in the
    /// TransactionRequest.
    type Fillable: Send + Sync + 'static;

    /// Joins this filler with another filler to compose multiple fillers.
    fn join_with<T>(self, other: T) -> JoinFill<Self, T, N>
    where
        T: TxFiller<N>,
    {
        JoinFill::new(self, other)
    }

    /// Return a control-flow enum indicating whether the filler is ready to
    /// fill in the transaction request, or if it is missing required
    /// properties.
    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow;

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
    ) -> impl Future<Output = TransportResult<Self::Fillable>> + Send
    where
        P: Provider<T, N>,
        T: Transport + Clone;

    /// Fills in the transaction request with the fillable properties.
    fn fill(&self, fillable: Self::Fillable, tx: &mut N::TransactionRequest);
}

/// A layer that can fill in a `TransactionRequest` with additional information
/// by joining two [`TxFiller`]s. This  struct is itself a [`TxFiller`],
/// and can be nested to compose any number of fill layers.
#[derive(Debug, Clone)]
pub struct JoinFill<L, R, N> {
    left: L,
    right: R,
    _network: PhantomData<fn() -> N>,
}

impl<L, R, N> JoinFill<L, R, N>
where
    L: TxFiller<N>,
    R: TxFiller<N>,
    N: Network,
{
    /// Creates a new `JoinFill` with the given layers.
    pub fn new(left: L, right: R) -> Self {
        Self { left, right, _network: PhantomData }
    }

    /// Get a request for the left filler, if the left filler is ready.
    async fn prepare_left<P, T>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Option<L::Fillable>>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        if self.left.ready(tx) {
            self.left.prepare(provider, tx).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Get a prepare for the right filler, if the right filler is ready.
    async fn prepare_right<P, T>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Option<R::Fillable>>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        if self.right.ready(tx) {
            self.right.prepare(provider, tx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<L, R, N> TxFiller<N> for JoinFill<L, R, N>
where
    L: TxFiller<N>,
    R: TxFiller<N>,
    N: Network,
{
    type Fillable = (Option<L::Fillable>, Option<R::Fillable>);

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        self.left.status(tx).absorb(self.right.status(tx))
    }

    async fn prepare<P, T>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        try_join!(self.prepare_left(provider, tx), self.prepare_right(provider, tx))
    }

    fn fill(&self, to_fill: Self::Fillable, tx: &mut N::TransactionRequest) {
        if let Some(to_fill) = to_fill.0 {
            self.left.fill(to_fill, tx);
        };
        if let Some(to_fill) = to_fill.1 {
            self.right.fill(to_fill, tx);
        };
    }
}

impl<L, R, P, T, N> ProviderLayer<P, T, N> for JoinFill<L, R, N>
where
    L: TxFiller<N>,
    R: TxFiller<N>,
    P: Provider<T, N>,
    T: alloy_transport::Transport + Clone,
    N: Network,
{
    type Provider = FillProvider<JoinFill<L, R, N>, P, T, N>;
    fn layer(&self, inner: P) -> Self::Provider {
        FillProvider::new(inner, self.clone())
    }
}

/// A [`Provider`] that applies one or more [`TxFiller`]s.
///
/// Fills arbitrary properties in a transaction request by composing multiple
/// fill layers.

#[derive(Debug, Clone)]
pub struct FillProvider<F, P, T, N>
where
    F: TxFiller<N>,
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    inner: P,
    filler: F,
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
    ) -> FillProvider<JoinFill<F, Other, N>, P, T, N> {
        self.filler.join_with(other).layer(self.inner)
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

    async fn send_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        while self.filler.status(&tx).is_ready() {
            let fillable = self.filler.prepare(self.root(), &tx).await?;

            // CONSIDER: should we have some sort of break condition or max loops here to account
            // for misimplemented fillers that are always ready and never finished?

            self.filler.fill(fillable, &mut tx);
        }
        // CONSIDER: should we error if the filler is not finished and also not ready?

        self.inner.send_transaction(tx).await
    }
}
