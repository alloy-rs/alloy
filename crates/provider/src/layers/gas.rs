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

    async fn get_eip1159_fees_estimate(&self) -> TransportResult<(U256, U256)> {
        let (max_fee_per_gas, max_priority_fee_per_gas) =
            self.inner.estimate_eip1559_fees(None).await?;

        Ok((max_fee_per_gas, max_priority_fee_per_gas))
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
        let (gas_price, gas_limit, eip1559_fees) = match (
            tx.gas_price().is_none(),
            tx.gas_limit().is_none(),
            (tx.max_fee_per_gas().is_none() && tx.max_priority_fee_per_gas().is_none()),
        ) {
            (true, true, true) => {
                let gas_price = self.get_gas_price();
                let gas_estimate = self.get_gas_estimate(&tx);
                let eip1559_fees = self.get_eip1159_fees_estimate();

                let (gas_price, gas_estimate, eip1559_fees) =
                    futures::join!(gas_price, gas_estimate, eip1559_fees);

                let gas_price = gas_price.unwrap();
                let gas_estimate = gas_estimate.unwrap();
                let (max_fee_per_gas, max_priority_fee_per_gas) = eip1559_fees.unwrap();

                (
                    Some(gas_price),
                    Some(gas_estimate),
                    Some((max_fee_per_gas, max_priority_fee_per_gas)),
                )
            }
            (true, true, false) => {
                let gas_price = self.get_gas_price();
                let gas_estimate = self.get_gas_estimate(&tx);

                let (gas_price, gas_estimate) = futures::join!(gas_price, gas_estimate);

                let gas_price = gas_price.unwrap();
                let gas_estimate = gas_estimate.unwrap();

                (Some(gas_price), Some(gas_estimate), None)
            }
            (true, false, true) => {
                let gas_price = self.get_gas_price();
                let eip1559_fees = self.get_eip1159_fees_estimate();

                let (gas_price, eip1559_fees) = futures::join!(gas_price, eip1559_fees);
                let (max_fee_per_gas, max_priority_fee_per_gas) = eip1559_fees.unwrap();

                let gas_price = gas_price.unwrap();

                (Some(gas_price), None, Some((max_fee_per_gas, max_priority_fee_per_gas)))
            }
            (true, false, false) => {
                let gas_price = self.get_gas_price().await?;
                (Some(gas_price), None, None)
            }
            (false, true, true) => {
                let gas_estimate = self.get_gas_estimate(&tx);
                let eip1559_fees = self.get_eip1159_fees_estimate();

                let (gas_estimate, eip1559_fees) = futures::join!(gas_estimate, eip1559_fees);
                let (max_fee_per_gas, max_priority_fee_per_gas) = eip1559_fees.unwrap();

                let gas_estimate = gas_estimate.unwrap();

                (None, Some(gas_estimate), Some((max_fee_per_gas, max_priority_fee_per_gas)))
            }
            (false, false, true) => {
                let (max_fee_per_gas, max_priority_fee_per_gas) =
                    self.get_eip1159_fees_estimate().await?;

                (None, None, Some((max_fee_per_gas, max_priority_fee_per_gas)))
            }
            _ => (None, None, None),
        };

        if let Some(gas_price) = gas_price {
            tx.set_gas_price(gas_price);
        }

        if let Some(gas_limit) = gas_limit {
            tx.set_gas_limit(gas_limit);
        }

        if let Some((max_fee_per_gas, max_priority_fee_per_gas)) = eip1559_fees {
            tx.set_max_fee_per_gas(max_fee_per_gas);
            tx.set_max_priority_fee_per_gas(max_priority_fee_per_gas);
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
