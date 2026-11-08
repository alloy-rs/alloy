use crate::{
    fillers::{FillProvider, FillerControlFlow, TxFiller},
    provider::SendableTx,
    Provider, ProviderLayer,
};
use alloy_network::Network;
use alloy_primitives::Address;
use alloy_transport::{TransportError, TransportResult};
use futures::join;

/// A filler that can fill in a [`TransactionRequest`] with additional information by joining two
/// [`TxFiller`]s.
///
/// This filler can be used to compose any number of fillers in layers by recursively joining them.
///
/// Ready fillers run [`TxFiller::prepare`] concurrently on the same transaction request.
/// Their prepared values are then applied by [`TxFiller::fill`] from left to right.
/// Both preparations finish before either [`TxFiller::fill`] is called. If only one of them fails,
/// the other's prepared value is passed to [`TxFiller::release`].
/// [`TxFiller::fill_sync`] also runs from left to right.
///
/// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
#[derive(Clone, Copy, Debug, Default)]
pub struct JoinFill<L, R> {
    left: L,
    right: R,
}

impl<L, R> JoinFill<L, R> {
    /// Creates a new `JoinFill` with the given layers.
    pub const fn new(left: L, right: R) -> Self {
        Self { left, right }
    }

    /// Get a reference to the left filler.
    pub const fn left(&self) -> &L {
        &self.left
    }

    /// Get a reference to the right filler.
    pub const fn right(&self) -> &R {
        &self.right
    }

    /// Get a mutable reference to the left filler.
    pub const fn left_mut(&mut self) -> &mut L {
        &mut self.left
    }

    /// Get a mutable reference to the right filler.
    pub const fn right_mut(&mut self) -> &mut R {
        &mut self.right
    }

    /// Maps the left filler to a new type.
    pub fn map_left<F, T>(self, f: F) -> JoinFill<T, R>
    where
        F: FnOnce(L) -> T,
    {
        JoinFill::new(f(self.left), self.right)
    }

    /// Maps the right filler to a new type.
    pub fn map_right<F, T>(self, f: F) -> JoinFill<L, T>
    where
        F: FnOnce(R) -> T,
    {
        JoinFill::new(self.left, f(self.right))
    }
}

impl<L, R> JoinFill<L, R> {
    /// Get a request for the left filler, if the left filler is ready.
    async fn prepare_left<P, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Option<L::Fillable>>
    where
        P: Provider<N>,
        L: TxFiller<N>,
        N: Network,
    {
        if self.left.ready(tx) {
            self.left.prepare(provider, tx).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Get a prepare for the right filler, if the right filler is ready.
    async fn prepare_right<P, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Option<R::Fillable>>
    where
        P: Provider<N>,
        R: TxFiller<N>,
        N: Network,
    {
        if self.right.ready(tx) {
            self.right.prepare(provider, tx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<L, R, N> TxFiller<N> for JoinFill<L, R>
where
    L: TxFiller<N>,
    R: TxFiller<N>,
    N: Network,
{
    type Fillable = (Option<L::Fillable>, Option<R::Fillable>);

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        self.left.status(tx).absorb(self.right.status(tx))
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        self.left.fill_sync(tx);
        self.right.fill_sync(tx);
    }

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        // Let both sides finish, so that one side's prepared value can be released if the other
        // fails.
        match join!(self.prepare_left(provider, tx), self.prepare_right(provider, tx)) {
            (Ok(left), Ok(right)) => Ok((left, right)),
            (Ok(left), Err(err)) => {
                if let Some(left) = left {
                    self.left.release(left).await;
                }
                Err(err)
            }
            (Err(err), Ok(right)) => {
                if let Some(right) = right {
                    self.right.release(right).await;
                }
                Err(err)
            }
            (Err(err), Err(_)) => Err(err),
        }
    }

    async fn fill(
        &self,
        to_fill: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(to_fill) = to_fill.0 {
            tx = self.left.fill(to_fill, tx).await?;
        };
        if let Some(to_fill) = to_fill.1 {
            tx = self.right.fill(to_fill, tx).await?;
        };
        Ok(tx)
    }

    async fn release(&self, fillable: Self::Fillable) {
        if let Some(left) = fillable.0 {
            self.left.release(left).await;
        }
        if let Some(right) = fillable.1 {
            self.right.release(right).await;
        }
    }

    async fn prepare_call(
        &self,
        tx: &mut <N as Network>::TransactionRequest,
    ) -> TransportResult<()> {
        self.left.prepare_call(tx).await?;
        self.right.prepare_call(tx).await?;
        Ok(())
    }

    fn prepare_call_sync(
        &self,
        tx: &mut <N as Network>::TransactionRequest,
    ) -> TransportResult<()> {
        self.left.prepare_call_sync(tx)?;
        self.right.prepare_call_sync(tx)?;
        Ok(())
    }

    async fn on_send_failure(&self, from: Address, nonce: u64, error: &TransportError) {
        self.left.on_send_failure(from, nonce, error).await;
        self.right.on_send_failure(from, nonce, error).await;
    }
}

impl<L, R, P, N> ProviderLayer<P, N> for JoinFill<L, R>
where
    L: TxFiller<N>,
    R: TxFiller<N>,
    P: Provider<N>,
    N: Network,
{
    type Provider = FillProvider<Self, P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        FillProvider::new(inner, self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RootProvider;
    use alloy_network::Ethereum;
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_transport::{mock::Asserter, TransportErrorKind};
    use std::sync::{Arc, Mutex};

    /// A filler that prepares its id, or fails with it, and records the ids it releases.
    #[derive(Clone, Debug)]
    struct ReservingFiller {
        id: u64,
        fails: bool,
        released: Arc<Mutex<Vec<u64>>>,
    }

    impl TxFiller for ReservingFiller {
        type Fillable = u64;

        fn status(&self, _tx: &TransactionRequest) -> FillerControlFlow {
            FillerControlFlow::Ready
        }

        fn fill_sync(&self, _tx: &mut SendableTx<Ethereum>) {}

        async fn prepare<P: Provider>(
            &self,
            _provider: &P,
            _tx: &TransactionRequest,
        ) -> TransportResult<u64> {
            if self.fails {
                return Err(TransportErrorKind::custom_str(&format!("filler {} failed", self.id)));
            }
            Ok(self.id)
        }

        async fn fill(
            &self,
            _id: u64,
            tx: SendableTx<Ethereum>,
        ) -> TransportResult<SendableTx<Ethereum>> {
            Ok(tx)
        }

        async fn release(&self, id: u64) {
            self.released.lock().unwrap().push(id);
        }
    }

    #[tokio::test]
    async fn releases_prepared_side_if_other_fails() {
        let provider = RootProvider::<Ethereum>::new(RpcClient::mocked(Asserter::new()));
        let tx = TransactionRequest::default();
        let released = Arc::new(Mutex::new(Vec::new()));
        let filler = |id, fails| ReservingFiller { id, fails, released: released.clone() };
        let take_released = || std::mem::take(&mut *released.lock().unwrap());

        let err = JoinFill::new(filler(1, false), filler(2, true))
            .prepare(&provider, &tx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("filler 2 failed"), "{err}");
        assert_eq!(take_released(), [1]);

        let err = JoinFill::new(filler(1, true), filler(2, false))
            .prepare(&provider, &tx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("filler 1 failed"), "{err}");
        assert_eq!(take_released(), [2]);

        // Nested joins release every side that prepared.
        let err = JoinFill::new(filler(1, false), JoinFill::new(filler(2, false), filler(3, true)))
            .prepare(&provider, &tx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("filler 3 failed"), "{err}");
        assert_eq!(take_released(), [2, 1]);

        // If both sides fail, the left error is returned and nothing is released.
        let err = JoinFill::new(filler(1, true), filler(2, true))
            .prepare(&provider, &tx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("filler 1 failed"), "{err}");
        assert!(take_released().is_empty());

        let join = JoinFill::new(filler(1, false), filler(2, false));
        join.release((Some(1), Some(2))).await;
        assert_eq!(take_released(), [1, 2]);
        join.release((None, Some(2))).await;
        assert_eq!(take_released(), [2]);
    }
}
