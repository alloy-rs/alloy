use crate::{PendingTransactionBuilder, Provider, ProviderLayer, RootProvider};
use alloy_network::{Network, TransactionBuilder};
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
    fn layer(&self, inner: P) -> Self::Provider {
        GasEstimatorProvider { inner, _phantom: PhantomData }
    }
}

/// A provider that provides gas estimation for transactions.
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
        let gas_price = self.inner.get_gas_price().await?;
        Ok(gas_price)
    }

    async fn get_gas_estimate(&self, tx: &N::TransactionRequest) -> TransportResult<U256> {
        let gas_estimate = self.inner.estimate_gas(tx, None).await?;
        Ok(gas_estimate)
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
        if tx.gas_price().is_none() {
            let gas_price = self.get_gas_price().await?;
            tx.set_gas_price(gas_price);
        }
        if tx.gas_limit().is_none() {
            let gas_estimate = self.get_gas_estimate(&tx).await?;
            tx.set_gas_limit(gas_estimate);
        }

        self.inner.send_transaction(tx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layers::ManagedNonceLayer, ProviderBuilder};
    use alloy_network::EthereumSigner;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, U128, U256};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types::TransactionRequest;
    use alloy_transport_http::Http;
    use reqwest::Client;

    #[tokio::test]
    async fn no_gas_price_or_limit() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .layer(ManagedNonceLayer)
            .layer(GasEstimatorLayer)
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let tx = TransactionRequest {
            from: Some(anvil.addresses()[0]),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let tx = tx.get_receipt().await.unwrap();

        let set_gas_price = U128::from(0x77359400);
        let set_gas_limit = U256::from(0x5208);

        assert_eq!(tx.effective_gas_price, set_gas_price);
        assert_eq!(tx.gas_used, Some(set_gas_limit));
    }
}
