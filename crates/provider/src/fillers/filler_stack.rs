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
pub struct FillerStack<T, N: Network = Ethereum> {
    fillers: TupleWrapper<T, N>,
}

impl<N: Network> Default for FillerStack<Empty, N> {
    fn default() -> Self {
        FillerStack { fillers: TupleWrapper::new(Empty) }
    }
}

/// A trait for tuples that can have types pushed to them
pub trait TuplePush<T, N: Network> {
    /// The resulting type after pushing T
    type Pushed;
}

/// Newtype wrapper for tuple conversions
#[derive(Debug, Clone)]
pub struct TupleWrapper<T, N: Network> {
    inner: T,
    _network: PhantomData<N>,
}

impl<T, N: Network> TupleWrapper<T, N> {
    fn new(inner: T) -> Self {
        Self { inner, _network: PhantomData }
    }
}

// Implement TuplePush for Empty
impl<T: TxFiller<N>, N: Network> TuplePush<T, N> for Empty {
    type Pushed = (T,);
}

// Implement methods for all FillerStack variants
impl<T, N: Network> FillerStack<T, N> {
    pub fn new(filler: T) -> Self {
        Self { fillers: TupleWrapper::new(filler) }
    }

    /// Push a new filler onto the stack
    pub fn push<F: TxFiller<N>>(self, filler: F) -> FillerStack<T::Pushed, N>
    where
        T: TuplePush<F, N>,
        TupleWrapper<T::Pushed, N>: From<(T, F)>,
    {
        FillerStack { fillers: TupleWrapper::from((self.fillers.inner, filler)) }
    }
}

impl<T, N: Network> TxFiller<N> for FillerStack<T, N>
where
    T: Clone + Debug + Send + Sync,
    TupleWrapper<T, N>: TxFiller<N>,
{
    type Fillable = <TupleWrapper<T, N> as TxFiller<N>>::Fillable;

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
        impl<$($ty: TxFiller<N>,)+ N: Network> TxFiller<N> for TupleWrapper<($($ty,)+), N> {
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
impl<T: TxFiller<N>, N: Network> From<(Empty, T)> for TupleWrapper<(T,), N> {
    fn from((_, t): (Empty, T)) -> Self {
        TupleWrapper::new((t,))
    }
}

impl<T: TxFiller<N>, N: Network> From<((T,),)> for TupleWrapper<(T,), N> {
    fn from(value: ((T,),)) -> Self {
        TupleWrapper::new(value.0)
    }
}

impl<L: TxFiller<N>, R: TxFiller<N>, N: Network> From<((L,), R)> for TupleWrapper<(L, R), N> {
    fn from((l, r): ((L,), R)) -> Self {
        TupleWrapper::new((l.0, r))
    }
}

impl<A: TxFiller<N>, B: TxFiller<N>, C: TxFiller<N>, N: Network> From<((A, B), C)>
    for TupleWrapper<(A, B, C), N>
{
    fn from((ab, c): ((A, B), C)) -> Self {
        TupleWrapper::new((ab.0, ab.1, c))
    }
}

impl<A: TxFiller<N>, B: TxFiller<N>, C: TxFiller<N>, D: TxFiller<N>, N: Network>
    From<((A, B, C), D)> for TupleWrapper<(A, B, C, D), N>
{
    fn from((abc, d): ((A, B, C), D)) -> Self {
        TupleWrapper::new((abc.0, abc.1, abc.2, d))
    }
}

impl<
        A: TxFiller<N>,
        B: TxFiller<N>,
        C: TxFiller<N>,
        D: TxFiller<N>,
        E: TxFiller<N>,
        N: Network,
    > From<((A, B, C, D), E)> for TupleWrapper<(A, B, C, D, E), N>
{
    fn from((abcd, e): ((A, B, C, D), E)) -> Self {
        TupleWrapper::new((abcd.0, abcd.1, abcd.2, abcd.3, e))
    }
}

/// Macro to implement ProviderLayer for tuples of different sizes
macro_rules! impl_provider_layer {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ P: Provider<N>, N: Network> ProviderLayer<P, N> for FillerStack<($($ty,)+), N> {
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
    use crate::fillers::{
        BlobGasFiller, ChainIdFiller, GasFiller, NonceFiller, RecommendedFillers, WalletFiller,
    };

    #[test]
    fn test_filler_stack() {
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

        // Type should be FillerStack<(GasFiller, NonceFiller, ChainIdFiller)>
        let recommend: FillerStack<
            (GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller),
            Ethereum,
        > = Ethereum::recommended_fillers();

        let _full_stack = recommend.push(WalletFiller::new(EthereumWallet::from(pk)));
    }
}
