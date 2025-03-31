use crate::{
    fillers::{FillProvider, FillerControlFlow, TxFiller},
    provider::Provider,
    Identity, ProviderLayer, SendableTx,
};
use alloy_network::{Ethereum, Network};
use alloy_transport::TransportResult;
use futures::try_join;
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

/// Macro to implement FillerNetwork for tuples of different sizes
macro_rules! impl_filler_network {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty,)+ N: Network> FillerNetwork<N> for Fillers<($($ty,)+), N> {
            type CurrentFillers = ($($ty,)+);

            fn network<Net: Network>(self) -> Fillers<($($ty,)+), Net> {
                self.network::<Net>()
            }
        }
    };
}

// Generate implementations for tuples from 1 to 8 fillers
impl_filler_network!(0 => T1);
impl_filler_network!(0 => T1, 1 => T2);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

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
    fn new(inner: T) -> Self {
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

// Macro to implement TuplePush for tuples of different sizes
macro_rules! impl_tuple {
    ($($idx:tt => $ty:ident),+) => {
        impl<T: TxFiller<N>, $($ty: TxFiller<N>,)+ N: Network> TuplePush<T, N> for ($($ty,)+) {
            type Pushed = ($($ty,)+ T,);
        }
    };
}

// Implement TuplePush for tuples up to 8 elements
impl_tuple!(0 => T1);
impl_tuple!(0 => T1, 1 => T2);
impl_tuple!(0 => T1, 1 => T2, 2 => T3);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

/// Macro to implement TxFiller for tuples of different sizes
macro_rules! impl_tx_filler {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ N: Network> TxFiller<N> for FillerTuple<($($ty,)+), N> {
            type Fillable = ($(Option<$ty::Fillable>,)+);

            fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
                let mut flow = FillerControlFlow::Finished;
                $(
                    flow = flow.absorb($ty::status(&self.inner.$idx, tx));
                )+
                flow
            }

            fn fill_sync(&self, tx: &mut SendableTx<N>) {
                $($ty::fill_sync(&self.inner.$idx, tx);)+
            }

            async fn prepare<P>(
                &self,
                provider: &P,
                tx: &N::TransactionRequest,
            ) -> TransportResult<Self::Fillable>
            where
                P: Provider<N>,
            {
                try_join!(
                    $(
                        async {
                            if $ty::ready(&self.inner.$idx, tx) {
                                $ty::prepare(&self.inner.$idx, provider, tx).await.map(Some)
                            } else {
                                Ok(None)
                            }
                        },
                    )+
                )
            }

            async fn fill(
                &self,
                to_fill: Self::Fillable,
                mut tx: SendableTx<N>,
            ) -> TransportResult<SendableTx<N>> {
                $(
                    if let Some(to_fill) = to_fill.$idx {
                        tx = $ty::fill(&self.inner.$idx, to_fill, tx).await?;
                    }
                )+
                Ok(tx)
            }

            async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
                $($ty::prepare_call(&self.inner.$idx, tx).await?;)+
                Ok(())
            }

            fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
                $($ty::prepare_call_sync(&self.inner.$idx, tx)?;)+
                Ok(())
            }
        }
    };
}

// Generate implementations for tuples from 1 to 8 fillers
impl_tx_filler!(0 => T1);
impl_tx_filler!(0 => T1, 1 => T2);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

/// Macro to implement From for FillerTuple of different sizes
macro_rules! impl_filler_tuple_from {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ T: TxFiller<N>, N: Network> From<(($($ty,)+), T)> for FillerTuple<($($ty,)+ T,), N> {
            fn from((tuple, t): (($($ty,)+), T)) -> Self {
                FillerTuple::new(($(tuple.$idx,)+ t))
            }
        }
    };
}

// Generate implementations for tuples from 1 to 8 fillers
impl_filler_tuple_from!(0 => T1);
impl_filler_tuple_from!(0 => T1, 1 => T2);
impl_filler_tuple_from!(0 => T1, 1 => T2, 2 => T3);
impl_filler_tuple_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_filler_tuple_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_filler_tuple_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_filler_tuple_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_filler_tuple_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

// Special case for Identity or default filler
impl<T: TxFiller<N>, N: Network> From<(Identity, T)> for FillerTuple<(T,), N> {
    fn from((_, t): (Identity, T)) -> Self {
        Self::new((t,))
    }
}

/// Macro to implement ProviderLayer for tuples of different sizes
macro_rules! impl_provider_layer {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ P: Provider<N>, N: Network> ProviderLayer<P, N> for Fillers<($($ty,)+), N> {
            type Provider = FillProvider<Self, P, N>;
            fn layer(&self, inner: P) -> Self::Provider {
                FillProvider::new(inner, self.clone())
            }
        }
    };
}

// Generate implementations for tuples from 1 to 8 fillers
impl_provider_layer!(0 => T1);
impl_provider_layer!(0 => T1, 1 => T2);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

#[cfg(test)]
mod tests {
    use alloy_network::{Ethereum, EthereumWallet};
    use alloy_signer_local::PrivateKeySigner;

    use super::*;
    use crate::{
        fillers::{
            BlobGasFiller, ChainIdFiller, GasFiller, NonceFiller, RecommendedFiller,
            RecommendedFillers, WalletFiller,
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
