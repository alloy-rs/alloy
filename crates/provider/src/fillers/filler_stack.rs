use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::SendableTx,
    Provider,
};
use alloy_network::Network;
use alloy_transport::TransportResult;
use futures::try_join;

/// Empty filler stack state
#[derive(Debug, Clone)]
pub struct Empty;

/// A stack of transaction fillers
#[derive(Debug, Clone)]
pub struct FillerStack<T> {
    fillers: T,
}

/// A trait for tuples that can have types pushed to them
pub trait TuplePush<T> {
    /// The resulting type after pushing T
    type Pushed;
}

/// Helper trait for tuple conversions
trait TupleFrom<T> {
    type Output;
    fn tuple_from(t: T) -> Self::Output;
}

/// Newtype wrapper for tuple conversions
#[derive(Debug, Clone)]
struct TupleWrapper<T>(T);

// Implement TuplePush for Empty
impl<T: TxFiller> TuplePush<T> for Empty {
    type Pushed = (T,);
}

// Implement base FillerStack methods
impl FillerStack<Empty> {
    /// Create a new empty filler stack
    pub fn new() -> Self {
        Self { fillers: Empty }
    }
}

// Implement methods for all FillerStack variants
impl<T> FillerStack<T> {
    /// Push a new filler onto the stack
    pub fn push<F: TxFiller>(self, filler: F) -> FillerStack<T::Pushed>
    where
        T: TuplePush<F>,
        TupleWrapper<T::Pushed>: From<(T, F)>,
    {
        FillerStack { fillers: TupleWrapper::from((self.fillers, filler)).0 }
    }
}

// Macro to implement TuplePush for tuples of different sizes
macro_rules! impl_tuple {
    ($($idx:tt => $ty:ident),+) => {
        impl<T: TxFiller, $($ty: TxFiller,)+> TuplePush<T> for ($($ty,)+) {
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
        impl<$($ty: TxFiller<N>,)+ N: Network> TxFiller<N> for FillerStack<($($ty,)+)> {
            type Fillable = ($(Option<$ty::Fillable>,)+);

            fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
                let mut flow = FillerControlFlow::Finished;
                $(
                    flow = flow.absorb($ty::status(&self.fillers.$idx, tx));
                )+
                flow
            }

            fn fill_sync(&self, tx: &mut SendableTx<N>) {
                $($ty::fill_sync(&self.fillers.$idx, tx);)+
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
                            if $ty::ready(&self.fillers.$idx, tx) {
                                $ty::prepare(&self.fillers.$idx, provider, tx).await.map(Some)
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
                        tx = $ty::fill(&self.fillers.$idx, to_fill, tx).await?;
                    }
                )+
                Ok(tx)
            }

            async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
                $($ty::prepare_call(&self.fillers.$idx, tx).await?;)+
                Ok(())
            }

            fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
                $($ty::prepare_call_sync(&self.fillers.$idx, tx)?;)+
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
impl<T: TxFiller> From<(Empty, T)> for TupleWrapper<(T,)> {
    fn from((_, t): (Empty, T)) -> Self {
        TupleWrapper((t,))
    }
}

impl<T: TxFiller> From<((T,),)> for TupleWrapper<(T,)> {
    fn from(value: ((T,),)) -> Self {
        TupleWrapper(value.0)
    }
}

impl<L: TxFiller, R: TxFiller> From<((L,), R)> for TupleWrapper<(L, R)> {
    fn from((l, r): ((L,), R)) -> Self {
        TupleWrapper((l.0, r))
    }
}

impl<A: TxFiller, B: TxFiller, C: TxFiller> From<((A, B), C)> for TupleWrapper<(A, B, C)> {
    fn from((ab, c): ((A, B), C)) -> Self {
        TupleWrapper((ab.0, ab.1, c))
    }
}

impl<A: TxFiller, B: TxFiller, C: TxFiller, D: TxFiller> From<((A, B, C), D)>
    for TupleWrapper<(A, B, C, D)>
{
    fn from((abc, d): ((A, B, C), D)) -> Self {
        TupleWrapper((abc.0, abc.1, abc.2, d))
    }
}

#[cfg(test)]
mod tests {
    use alloy_network::EthereumWallet;
    use alloy_signer_local::PrivateKeySigner;

    use super::*;
    use crate::fillers::{ChainIdFiller, GasFiller, NonceFiller, SimpleNonceManager, WalletFiller};

    #[test]
    fn test_filler_stack() {
        let pk: PrivateKeySigner =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();
        let stack = FillerStack::new()
            .push(GasFiller)
            .push(NonceFiller::new(SimpleNonceManager::default()))
            .push(ChainIdFiller::default())
            .push(WalletFiller::new(EthereumWallet::new(pk)));

        // Type should be FillerStack<(GasFiller, NonceFiller, ChainIdFiller)>
        let _: FillerStack<(GasFiller, NonceFiller, ChainIdFiller, WalletFiller<EthereumWallet>)> =
            stack;
    }
}
