use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::Provider,
    Identity, SendableTx,
};
use alloy_network::{Ethereum, Network};
use alloy_transport::TransportResult;
use std::{fmt::Debug, marker::PhantomData};

/// A stack of [`TxFiller`]'s.
#[derive(Debug, Clone)]
pub struct Fillers<T, N = Ethereum> {
    /// The [`FillerTuple`] stores the tuple of [`TxFiller`]s
    fillers: FillerTuple<T, N>,
}

impl<N: Network> Default for Fillers<Identity, N> {
    fn default() -> Self {
        Self { fillers: FillerTuple::new(Identity) }
    }
}

impl<T, N: Network> Fillers<T, N> {
    /// Instatiate a new [`Fillers`] stack with
    pub fn new(filler: T) -> Self {
        Self { fillers: FillerTuple::new(filler) }
    }

    /// Push a new [`TxFiller`] onto the stack
    pub fn push<F: TxFiller<N>>(self, filler: F) -> Fillers<T::Pushed, N>
    where
        T: TuplePush<F, N>,
        FillerTuple<T::Pushed, N>: From<(T, F)>,
    {
        Fillers { fillers: FillerTuple::from((self.fillers.inner, filler)) }
    }

    /// Change the [`Network`] that is associated with the fillers
    pub fn network<Net: Network>(self) -> Fillers<T, Net> {
        Fillers { fillers: self.fillers.network::<Net>() }
    }

    /// Access the inner [`FillerTuple`].
    ///
    /// Useful for implementing custom [`Provider`]s that require access to the inner tuple.
    /// e.g. [`crate::WalletProvider`]
    pub fn fillers(&self) -> &FillerTuple<T, N> {
        &self.fillers
    }

    /// Mutable access to the inner [`FillerTuple`]
    ///
    /// Useful for implementing custom [`Provider`]s that require mutable access to the inner tuple.
    /// e.g. [`crate::WalletProvider`]
    pub fn fillers_mut(&mut self) -> &mut FillerTuple<T, N> {
        &mut self.fillers
    }
}

/// A trait that enables pushing new fillers onto a filler stack.
///
/// Useful for building a stack of fillers when the filler type is unknown in [`ProviderBuilder`].
///
/// See usage in [`ProviderBuilder::filler`] and [`ProviderBuilder::wallet`].
///
///
/// [`ProviderBuilder`]: crate::builder::ProviderBuilder
/// [`ProviderBuilder::filler`]: crate::builder::ProviderBuilder::filler
/// [`ProviderBuilder::wallet`]: crate::builder::ProviderBuilder::wallet
pub trait Pushable<F: TxFiller<N>, N: Network> {
    /// The resulting type after pushing the [`TxFiller`] onto the stack
    type Pushed;

    /// Push a new filler onto the stack
    ///
    /// ## Returns
    ///
    /// A [`Fillers`] instance with the pushed filler
    fn push(self, filler: F) -> Fillers<Self::Pushed, N>
    where
        Self: Sized;
}

impl<T, F: TxFiller<N>, N: Network> Pushable<F, N> for Fillers<T, N>
where
    T: TuplePush<F, N>,
    FillerTuple<T::Pushed, N>: From<(T, F)>,
{
    type Pushed = T::Pushed;

    fn push(self, filler: F) -> Fillers<Self::Pushed, N>
    where
        Self: Sized,
    {
        self.push(filler)
    }
}

impl<F: TxFiller<N>, N: Network> Pushable<F, N> for crate::Identity {
    type Pushed = (F,);

    fn push(self, filler: F) -> Fillers<Self::Pushed, N>
    where
        Self: Sized,
    {
        Fillers::new((filler,))
    }
}

/// A trait that changes the network associated with the [`Fillers`] stack.
///
/// Useful for changing the network of the [`Provider`] being built using
/// [`ProviderBuilder::network`].
///
/// [`ProviderBuilder::network`]: crate::builder::ProviderBuilder::network
pub trait FillerNetwork<N> {
    /// The current tuple of fillers in the stack.
    ///
    /// e.g. `(GasFiller, NonceFiller, ChainIdFiller)`
    ///
    /// OR in case of [`crate:Identity`]: [`Empty`]
    type CurrentFillers;

    /// Change the network associated with the [`Fillers`] stack.
    fn network<Net: Network>(self) -> Fillers<Self::CurrentFillers, Net>;
}

impl<N: Network> FillerNetwork<N> for crate::Identity {
    type CurrentFillers = Self;

    fn network<Net: Network>(self) -> Fillers<Self::CurrentFillers, Net> {
        Fillers::default()
    }
}

/// A trait for tuples that can have types pushed to them
pub trait TuplePush<T, N = Ethereum> {
    /// The resulting type after pushing T
    type Pushed;
}

/// Wrapper type for a tuple of [`TxFiller`]s
#[derive(Debug, Clone)]
pub struct FillerTuple<T, N = Ethereum> {
    inner: T,
    _network: PhantomData<N>,
}

impl<T, N: Network> FillerTuple<T, N> {
    pub(crate) fn new(inner: T) -> Self {
        Self { inner, _network: PhantomData }
    }

    /// Change the [`Network`] associated with the [`FillerTuple`] stack.
    ///
    /// Used in conjunction with [`Fillers::network`] to change the network of the entire stack.
    fn network<Net: Network>(self) -> FillerTuple<T, Net> {
        FillerTuple { inner: self.inner, _network: PhantomData }
    }

    /// Get a reference to the inner tuple
    ///
    /// This is public for use in [`Provider`] implementations that require access to the inner
    /// tuple. e.g [`crate::WalletProvider`].
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner tuple
    ///
    /// This is public for use in [`Provider`] implementations that require mutable access to the
    /// inner tuple. e.g [`crate::WalletProvider`].
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

// Implement TuplePush for Empty
impl<T: TxFiller<N>, N: Network> TuplePush<T, N> for Identity {
    type Pushed = (T,);
}

impl<T, N: Network> TxFiller<N> for Fillers<T, N>
where
    T: Clone + Debug + Send + Sync,
    FillerTuple<T, N>: TxFiller<N>,
{
    type Fillable = <FillerTuple<T, N> as TxFiller<N>>::Fillable;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        self.fillers.status(tx)
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        self.fillers.fill_sync(tx)
    }

    fn prepare_call_sync(
        &self,
        tx: &mut <N as Network>::TransactionRequest,
    ) -> TransportResult<()> {
        self.fillers.prepare_call_sync(tx)
    }

    fn prepare<P: Provider<N>>(
        &self,
        provider: &P,
        tx: &<N as Network>::TransactionRequest,
    ) -> alloy_transport::impl_future!(<Output = TransportResult<Self::Fillable>>) {
        self.fillers.prepare(provider, tx)
    }

    fn fill(
        &self,
        fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> alloy_transport::impl_future!(<Output = TransportResult<SendableTx<N>>>) {
        self.fillers.fill(fillable, tx)
    }
}

#[cfg(test)]
mod tests {

    use alloy_network::{Ethereum, EthereumWallet};
    use alloy_signer_local::PrivateKeySigner;

    use crate::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, Fillers, GasFiller, NonceFiller,
            RecommendedFiller, RecommendedFillers, WalletFiller,
        },
        layers::AnvilProvider,
        ProviderBuilder, RootProvider,
    };

    #[test]
    fn test_filler_stack() {
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

        let recommend: RecommendedFiller = Ethereum::recommended_fillers();

        let _full_stack = recommend.push(WalletFiller::new(EthereumWallet::from(pk)));
    }

    type RecommendedWalletFillers =
        (GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller, WalletFiller<EthereumWallet>);

    #[test]
    fn test_provider_builder() {
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

        type AnvilWalletFiller =
            FillProvider<Fillers<RecommendedWalletFillers, Ethereum>, AnvilProvider<RootProvider>>;

        // Basic works
        let _provider = ProviderBuilder::new().on_anvil();

        // With wallet
        let _provider: AnvilWalletFiller = ProviderBuilder::new().wallet(pk).on_anvil();

        // With anvil wallet
        let _provider: AnvilWalletFiller = ProviderBuilder::new().on_anvil_with_wallet();

        // With anvil wallet and config
        let _provider: AnvilWalletFiller =
            ProviderBuilder::new().on_anvil_with_wallet_and_config(|a| a.block_time(1)).unwrap();
    }
}
