use crate::{fillers::TxFiller, Identity, Provider, ProviderLayer};
use alloy_network::{Ethereum, Network};
use std::marker::PhantomData;

use super::FillProvider;

/// A [`ProviderLayer`] that encapsulates a tuple of [`TxFiller`]s, and traverses through the tuple
/// to fill a [`TransactionRequest`].
///
/// Usage in [`ProviderBuilder`] and [`Provider`] is facilitated by the implementation of the
/// following traits:
///
/// - [`Pushable`]: used to push new fillers in the tuple.
/// - [`FillerNetwork`]: used to change the network associated with the fillers.
/// - [`TuplePush`]: enables the pushing of new types to the tuple.
/// - [`TxFiller`]: Enables traversing through the inner tuple of fillers and filling the
///   [`TransactionRequest`].
/// - [`ProviderLayer`]: Enabling the fillers type to be used as a layer in the provider stack.
///
/// Note:
///
/// This struct can accomodate a maximum of 15 fillers in the tuple. This limitation is
/// imposed by the underlying trait implementations.
///
/// [`ProviderBuilder`]: crate::builder::ProviderBuilder
/// [`ProviderLayer`]: crate::ProviderLayer
/// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
/// [`Provider`]: crate::Provider
#[derive(Clone)]
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

impl<T, N> Fillers<T, N> {
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
}

impl<T, N: Network> Fillers<T, N> {
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
    /// Useful for implementing custom [`crate::Provider`]s that require access to the inner tuple.
    /// e.g. [`crate::WalletProvider`]
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutable access to the inner tuple.
    ///
    /// Useful for implementing custom [`crate::Provider`]s that require mutable access to the inner
    /// tuple. e.g. [`crate::WalletProvider`]
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

mod private {

    /// Used to seal the Pushable trait.
    #[allow(unnameable_types)]
    pub trait Sealed {}
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
pub trait Pushable<F: TxFiller<N>, N: Network>: private::Sealed {
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

impl private::Sealed for crate::Identity {}
impl<T, N: Network> private::Sealed for Fillers<T, N> {}

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
/// Useful for changing the network of the provider being built using
/// [`ProviderBuilder::network`].
///
/// [`ProviderBuilder::network`]: crate::builder::ProviderBuilder::network
pub trait FillerNetwork<N> {
    /// The current tuple of fillers in the stack.
    ///
    /// e.g. `(GasFiller, NonceFiller, ChainIdFiller)`
    type CurrentFillers;

    /// Change the network associated with the [`Fillers`] stack.
    fn network<Net: Network>(self) -> Fillers<Self::CurrentFillers, Net>;
}

impl<T, N: Network> FillerNetwork<N> for Fillers<T, N>
where
    Self: TxFiller<N>,
{
    type CurrentFillers = T;

    fn network<Net: Network>(self) -> Fillers<T, Net> {
        Fillers::new(self.inner)
    }
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

// Blanket impl for ProviderLayer
impl<T, P: Provider<N>, N: Network> ProviderLayer<P, N> for Fillers<T, N>
where
    Self: TxFiller<N>,
{
    type Provider = FillProvider<Self, P, N>;
    fn layer(&self, inner: P) -> Self::Provider {
        FillProvider::new(inner, self.clone())
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use alloy_network::{AnyNetwork, Ethereum, EthereumWallet, Network, TransactionBuilder};
    use alloy_primitives::Bytes;
    use alloy_signer_local::PrivateKeySigner;
    use alloy_transport::TransportResult;

    use crate::{
        fillers::{
            BlobGasFiller, CachedNonceManager, ChainIdFiller, FillProvider, Fillers, GasFiller,
            NonceFiller, RecommendedFiller, RecommendedFillers, TxFiller, WalletFiller,
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
        let _provider = ProviderBuilder::new().connect_anvil();

        // With wallet
        let _provider: AnvilWalletFiller = ProviderBuilder::new().wallet(pk).connect_anvil();

        // With anvil wallet
        let _provider: AnvilWalletFiller = ProviderBuilder::new().connect_anvil_with_wallet();

        // With anvil wallet and config
        let _provider: AnvilWalletFiller = ProviderBuilder::new()
            .connect_anvil_with_wallet_and_config(|a| a.block_time(1))
            .unwrap();
    }

    #[tokio::test]
    async fn custom_filler() {
        #[derive(Debug, Clone)]
        struct InputFiller;

        impl<N: Network> TxFiller<N> for InputFiller {
            type Fillable = Bytes;

            fn status(
                &self,
                tx: &<N as Network>::TransactionRequest,
            ) -> crate::fillers::FillerControlFlow {
                if tx.input().is_some() {
                    crate::fillers::FillerControlFlow::Finished
                } else {
                    crate::fillers::FillerControlFlow::Ready
                }
            }

            fn fill_sync(&self, tx: &mut crate::SendableTx<N>) {
                if let Some(builder) = tx.as_mut_builder() {
                    if builder.input().is_none() {
                        builder.set_input(Bytes::from_str("0xdeadbeef").unwrap());
                    }
                }
            }

            async fn prepare<P: crate::Provider<N>>(
                &self,
                _provider: &P,
                _tx: &<N as Network>::TransactionRequest,
            ) -> TransportResult<Self::Fillable> {
                Ok(Bytes::from_str("0xdeadbeef").unwrap())
            }

            async fn fill(
                &self,
                _fillable: Self::Fillable,
                mut tx: crate::SendableTx<N>,
            ) -> TransportResult<crate::SendableTx<N>> {
                self.fill_sync(&mut tx);
                Ok(tx)
            }
        }

        // With recommended fillers
        #[allow(clippy::type_complexity)]
        let _p: FillProvider<
            Fillers<(GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller, InputFiller)>,
            _,
        > = ProviderBuilder::new().filler(InputFiller).connect_anvil();

        // Without recommended fillers
        let _p: FillProvider<Fillers<(InputFiller,)>, _> = ProviderBuilder::new()
            .disable_recommended_fillers()
            .filler(InputFiller)
            .connect_anvil();

        // With wallet
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();
        let _p: FillProvider<Fillers<(InputFiller, WalletFiller<EthereumWallet>)>, _> =
            ProviderBuilder::new()
                .disable_recommended_fillers()
                .filler(InputFiller)
                .wallet(pk)
                .connect_anvil();
    }

    #[test]
    fn change_network() {
        #[allow(clippy::type_complexity)]
        let _p: FillProvider<
            Fillers<(GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller), AnyNetwork>,
            AnvilProvider<RootProvider<AnyNetwork>, AnyNetwork>,
            AnyNetwork,
        > = ProviderBuilder::new().network::<AnyNetwork>().connect_anvil();
    }

    #[test]
    fn upto_15_fillers() {
        #[allow(clippy::type_complexity)]
        let _: FillProvider<
            Fillers<(
                GasFiller,
                BlobGasFiller,
                NonceFiller,
                ChainIdFiller,
                GasFiller,
                BlobGasFiller,
                NonceFiller,
                ChainIdFiller,
                GasFiller,
                BlobGasFiller,
                NonceFiller,
                ChainIdFiller,
                GasFiller,
                BlobGasFiller,
                WalletFiller<EthereumWallet>,
            )>,
            _,
        > = ProviderBuilder::new() // Adds 4.
            .filler(GasFiller)
            .filler(BlobGasFiller)
            .filler(NonceFiller::new(CachedNonceManager::default()))
            .filler(ChainIdFiller::default())
            .filler(GasFiller)
            .filler(BlobGasFiller)
            .filler(NonceFiller::new(CachedNonceManager::default()))
            .filler(ChainIdFiller::default())
            .filler(GasFiller)
            .filler(BlobGasFiller)
            .connect_anvil_with_wallet(); // Adds the 15th i.e wallet filler.
    }
}
