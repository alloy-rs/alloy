use crate::{
    fillers::{FillProvider, FillerControlFlow, TxFiller},
    provider::Provider,
    ProviderLayer, SendableTx,
};
use alloy_network::{Ethereum, Network};
use alloy_transport::TransportResult;
use futures::try_join;
use std::{fmt::Debug, marker::PhantomData};

/// Empty filler stack state
#[derive(Debug, Clone)]
pub struct Empty;

/// A stack of transaction fillers
#[derive(Debug, Clone)]
pub struct Fillers<T, N = Ethereum> {
    pub fillers: FillerTuple<T, N>,
}

impl<N: Network> Default for Fillers<Empty, N> {
    fn default() -> Self {
        Fillers { fillers: FillerTuple::new(Empty) }
    }
}

// Implement methods for all Fillers variants
impl<T, N: Network> Fillers<T, N> {
    pub fn new(filler: T) -> Self {
        Self { fillers: FillerTuple::new(filler) }
    }

    /// Push a new filler onto the stack
    pub fn push<F: TxFiller<N>>(self, filler: F) -> Fillers<T::Pushed, N>
    where
        T: TuplePush<F, N>,
        FillerTuple<T::Pushed, N>: From<(T, F)>,
    {
        Fillers { fillers: FillerTuple::from((self.fillers.inner, filler)) }
    }
}

pub trait Pushable<F: TxFiller<N>, N: Network> {
    type Pushed;

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

/// A trait for tuples that can have types pushed to them
pub trait TuplePush<T, N = Ethereum> {
    /// The resulting type after pushing T
    type Pushed;
}

/// Newtype wrapper for tuple conversions
#[derive(Debug, Clone)]
pub struct FillerTuple<T, N = Ethereum> {
    pub inner: T,
    _network: PhantomData<N>,
}

impl<T, N: Network> FillerTuple<T, N> {
    fn new(inner: T) -> Self {
        Self { inner, _network: PhantomData }
    }
}

// Implement TuplePush for Empty
impl<T: TxFiller<N>, N: Network> TuplePush<T, N> for Empty {
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

// Implement From for tuple types
impl<T: TxFiller<N>, N: Network> From<(Empty, T)> for FillerTuple<(T,), N> {
    fn from((_, t): (Empty, T)) -> Self {
        FillerTuple::new((t,))
    }
}

impl<T: TxFiller<N>, N: Network> From<((T,),)> for FillerTuple<(T,), N> {
    fn from(value: ((T,),)) -> Self {
        FillerTuple::new(value.0)
    }
}

impl<L: TxFiller<N>, R: TxFiller<N>, N: Network> From<((L,), R)> for FillerTuple<(L, R), N> {
    fn from((l, r): ((L,), R)) -> Self {
        FillerTuple::new((l.0, r))
    }
}

impl<A: TxFiller<N>, B: TxFiller<N>, C: TxFiller<N>, N: Network> From<((A, B), C)>
    for FillerTuple<(A, B, C), N>
{
    fn from((ab, c): ((A, B), C)) -> Self {
        FillerTuple::new((ab.0, ab.1, c))
    }
}

impl<A: TxFiller<N>, B: TxFiller<N>, C: TxFiller<N>, D: TxFiller<N>, N: Network>
    From<((A, B, C), D)> for FillerTuple<(A, B, C, D), N>
{
    fn from((abc, d): ((A, B, C), D)) -> Self {
        FillerTuple::new((abc.0, abc.1, abc.2, d))
    }
}

impl<
        A: TxFiller<N>,
        B: TxFiller<N>,
        C: TxFiller<N>,
        D: TxFiller<N>,
        E: TxFiller<N>,
        N: Network,
    > From<((A, B, C, D), E)> for FillerTuple<(A, B, C, D, E), N>
{
    fn from((abcd, e): ((A, B, C, D), E)) -> Self {
        FillerTuple::new((abcd.0, abcd.1, abcd.2, abcd.3, e))
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
            BlobGasFiller, ChainIdFiller, GasFiller, NonceFiller, RecommendedFillers, WalletFiller,
        },
        layers::AnvilProvider,
        ProviderBuilder, RootProvider,
    };

    #[test]
    fn test_filler_stack() {
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

        // Type should be Fillers<(GasFiller, NonceFiller, ChainIdFiller)>
        let recommend: Fillers<(GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller), Ethereum> =
            Ethereum::recommended_fillers();

        let _full_stack = recommend.push(WalletFiller::new(EthereumWallet::from(pk)));
    }

    #[test]
    fn test_provider_builder() {
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

        type AnvilWalletFiller = FillProvider<
            Fillers<
                (
                    GasFiller,
                    BlobGasFiller,
                    NonceFiller,
                    ChainIdFiller,
                    WalletFiller<EthereumWallet>,
                ),
                Ethereum,
            >,
            AnvilProvider<RootProvider>,
        >;

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
