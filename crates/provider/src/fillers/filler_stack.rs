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

// Macro to implement for tuples of different sizes
macro_rules! impl_tuple {
    ($($idx:tt => $ty:ident),+) => {
        // Implement pushing a new type onto the tuple
        impl<T: TxFiller, $($ty: TxFiller,)+> TuplePush<T> for ($($ty,)+) {
            type Pushed = ($($ty,)+ T,);
        }
    };
}

// Implement for tuples up to 3 elements (can be extended if needed)
impl_tuple!(0 => T1);
impl_tuple!(0 => T1, 1 => T2);
impl_tuple!(0 => T1, 1 => T2, 2 => T3);

// Implement TxFiller for Empty
impl<N: Network> TxFiller<N> for FillerStack<Empty> {
    type Fillable = ();

    fn status(&self, _tx: &N::TransactionRequest) -> FillerControlFlow {
        FillerControlFlow::Finished
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        _provider: &P,
        _tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        Ok(())
    }

    async fn fill(
        &self,
        _fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        Ok(tx)
    }

    async fn prepare_call(&self, _tx: &mut N::TransactionRequest) -> TransportResult<()> {
        Ok(())
    }

    fn prepare_call_sync(&self, _tx: &mut N::TransactionRequest) -> TransportResult<()> {
        Ok(())
    }
}

// Implement TxFiller for single filler
impl<F: TxFiller<N>, N: Network> TxFiller<N> for FillerStack<(F,)> {
    type Fillable = F::Fillable;

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        F::status(&self.fillers.0, tx)
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        F::fill_sync(&self.fillers.0, tx)
    }

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        F::prepare(&self.fillers.0, provider, tx).await
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        F::fill(&self.fillers.0, fillable, tx).await
    }

    async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        F::prepare_call(&self.fillers.0, tx).await
    }

    fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        F::prepare_call_sync(&self.fillers.0, tx)
    }
}

// Implement TxFiller for tuple of two fillers
impl<L: TxFiller<N>, R: TxFiller<N>, N: Network> TxFiller<N> for FillerStack<(L, R)> {
    type Fillable = (Option<L::Fillable>, Option<R::Fillable>);

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        L::status(&self.fillers.0, tx).absorb(R::status(&self.fillers.1, tx))
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        L::fill_sync(&self.fillers.0, tx);
        R::fill_sync(&self.fillers.1, tx);
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
            async {
                if L::ready(&self.fillers.0, tx) {
                    L::prepare(&self.fillers.0, provider, tx).await.map(Some)
                } else {
                    Ok(None)
                }
            },
            async {
                if R::ready(&self.fillers.1, tx) {
                    R::prepare(&self.fillers.1, provider, tx).await.map(Some)
                } else {
                    Ok(None)
                }
            }
        )
    }

    async fn fill(
        &self,
        to_fill: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(to_fill) = to_fill.0 {
            tx = L::fill(&self.fillers.0, to_fill, tx).await?;
        }
        if let Some(to_fill) = to_fill.1 {
            tx = R::fill(&self.fillers.1, to_fill, tx).await?;
        }
        Ok(tx)
    }

    async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        L::prepare_call(&self.fillers.0, tx).await?;
        R::prepare_call(&self.fillers.1, tx).await?;
        Ok(())
    }

    fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        L::prepare_call_sync(&self.fillers.0, tx)?;
        R::prepare_call_sync(&self.fillers.1, tx)?;
        Ok(())
    }
}

// Implement TxFiller for tuple of three fillers
impl<A: TxFiller<N>, B: TxFiller<N>, C: TxFiller<N>, N: Network> TxFiller<N>
    for FillerStack<(A, B, C)>
{
    type Fillable = (Option<A::Fillable>, Option<B::Fillable>, Option<C::Fillable>);

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        A::status(&self.fillers.0, tx)
            .absorb(B::status(&self.fillers.1, tx))
            .absorb(C::status(&self.fillers.2, tx))
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        A::fill_sync(&self.fillers.0, tx);
        B::fill_sync(&self.fillers.1, tx);
        C::fill_sync(&self.fillers.2, tx);
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
            async {
                if A::ready(&self.fillers.0, tx) {
                    A::prepare(&self.fillers.0, provider, tx).await.map(Some)
                } else {
                    Ok(None)
                }
            },
            async {
                if B::ready(&self.fillers.1, tx) {
                    B::prepare(&self.fillers.1, provider, tx).await.map(Some)
                } else {
                    Ok(None)
                }
            },
            async {
                if C::ready(&self.fillers.2, tx) {
                    C::prepare(&self.fillers.2, provider, tx).await.map(Some)
                } else {
                    Ok(None)
                }
            }
        )
    }

    async fn fill(
        &self,
        to_fill: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(to_fill) = to_fill.0 {
            tx = A::fill(&self.fillers.0, to_fill, tx).await?;
        }
        if let Some(to_fill) = to_fill.1 {
            tx = B::fill(&self.fillers.1, to_fill, tx).await?;
        }
        if let Some(to_fill) = to_fill.2 {
            tx = C::fill(&self.fillers.2, to_fill, tx).await?;
        }
        Ok(tx)
    }

    async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        A::prepare_call(&self.fillers.0, tx).await?;
        B::prepare_call(&self.fillers.1, tx).await?;
        C::prepare_call(&self.fillers.2, tx).await?;
        Ok(())
    }

    fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        A::prepare_call_sync(&self.fillers.0, tx)?;
        B::prepare_call_sync(&self.fillers.1, tx)?;
        C::prepare_call_sync(&self.fillers.2, tx)?;
        Ok(())
    }
}

// Implement From for tuple types
impl<T: TxFiller> From<(Empty, T)> for TupleWrapper<(T,)> {
    fn from((_, t): (Empty, T)) -> Self {
        TupleWrapper((t,))
    }
}

impl<L: TxFiller, R: TxFiller> From<((L,), R)> for TupleWrapper<(L, R)> {
    fn from(((l,), r): ((L,), R)) -> Self {
        TupleWrapper((l, r))
    }
}

impl<A: TxFiller, B: TxFiller, C: TxFiller> From<((A, B), C)> for TupleWrapper<(A, B, C)> {
    fn from(((a, b), c): ((A, B), C)) -> Self {
        TupleWrapper((a, b, c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fillers::{ChainIdFiller, GasFiller, NonceFiller};

    #[test]
    fn test_filler_stack() {
        let stack = FillerStack::new()
            .push(GasFiller)
            .push(NonceFiller::default())
            .push(ChainIdFiller::default());

        // Type should be FillerStack<(GasFiller, NonceFiller, ChainIdFiller)>
        let _: FillerStack<(GasFiller, NonceFiller, ChainIdFiller)> = stack;
    }
}
