use crate::{PendingTransactionBuilder, Provider, ProviderLayer, RootProvider};
use alloy_network::Network;
use alloy_primitives::U256;
use alloy_transport::{Transport, TransportResult};
use async_trait::async_trait;
use std::marker::PhantomData;

/// A layer that provides gas estimation for transactions.
/// Populates the gas price and gas limit for transactions.
#[derive(Debug, Clone, Copy)]
pub struct GasEstimatorLayer;

impl<P, N, T> ProviderLayer<P, N, T> for GasEstimatorLayer
where
    P: Provider<N, T>,
    N: Network,
    T: Transport + Clone,
{
    type Provider = GasEstimatorProvider<N, T, P>;
    fn layer(&self, _inner: P) -> Self::Provider {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct GasEstimatorProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    inner: P,
    _phantom: PhantomData<(N, T)>,
}

impl<N, T, P> GasEstimatorProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    async fn get_gas_price(&self) -> TransportResult<U256> {
        todo!()
    }

    async fn get_gas_estimate(&self, _tx: N::TransactionRequest) -> TransportResult<U256> {
        todo!()
    }

    async fn get_1559_estimate_fees(
        &self,
        _tx: N::TransactionRequest,
    ) -> TransportResult<(U256, U256)> {
        todo!()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<N, T, P> Provider<N, T> for GasEstimatorProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    fn root(&self) -> &RootProvider<N, T> {
        self.inner.root()
    }

    async fn send_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, N, T>> {
        // TODO: Get estimates and set the gas for the tx.
        self.inner.send_transaction(tx).await
    }
}
