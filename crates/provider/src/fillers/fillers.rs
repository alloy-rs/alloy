use crate::{fillers::TxFiller, Identity};
use alloy_network::{Ethereum, Network};
use std::{fmt::Debug, marker::PhantomData};

/// A stack of [`TxFiller`]'s.
#[derive(Debug, Clone)]
pub struct Fillers<T, N = Ethereum> {
    /// Stores the tuple of [`TxFiller`]s
    ///
    /// e.g `(GasFiller, NonceFiller, ChainIdFiller)`
    inner: T,
    _pd: PhantomData<N>,
}

impl<N: Network> Default for Fillers<Identity, N> {
    fn default() -> Self {
        Self { inner: Identity, _pd: PhantomData }
    }
}

impl<T, N: Network> Fillers<T, N> {
    /// Instatiate a new [`Fillers`] stack with tuple of [`TxFiller`]s
    ///
    /// ## Example
    ///
    /// ```ignore
    /// let filler_stack = Fillers::new((GasFiller, NonceFiller, ChainIdFiller));
    /// ```
    pub(crate) fn new(filler: T) -> Self {
        Self { inner: filler, _pd: PhantomData }
    }

    /// Push a new [`TxFiller`] onto the stack
    pub fn push<F: TxFiller<N>>(self, filler: F) -> Fillers<T::Pushed, N>
    where
        T: TuplePush<F, N>,
        Fillers<T::Pushed, N>: From<(T, F)>,
    {
        Fillers::from((self.inner, filler))
    }

    /// Change the [`Network`] that is associated with the fillers
    pub fn network<Net: Network>(self) -> Fillers<T, Net> {
        Fillers { inner: self.inner, _pd: PhantomData }
    }

    /// Access the inner tuple.
    ///
    /// Useful for implementing custom [`Provider`]s that require access to the inner tuple.
    /// e.g. [`crate::WalletProvider`]
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutable access to the inner tuple.
    ///
    /// Useful for implementing custom [`Provider`]s that require mutable access to the inner tuple.
    /// e.g. [`crate::WalletProvider`]
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
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
    Fillers<T::Pushed, N>: From<(T, F)>,
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

// Implement TuplePush for Empty
impl<T: TxFiller<N>, N: Network> TuplePush<T, N> for Identity {
    type Pushed = (T,);
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
