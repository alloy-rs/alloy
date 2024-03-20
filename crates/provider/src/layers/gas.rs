use crate::{PendingTransactionBuilder, Provider, ProviderLayer, RootProvider};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::U256;
use alloy_transport::{Transport, TransportError, TransportResult};
use async_trait::async_trait;
use std::marker::PhantomData;

/// A layer that populates gas related fields in transaction requests if unset.
///
/// Gas related fields are gas_price, gas_limit, max_fee_per_gas and max_priority_fee_per_gas.
///
/// The layer fetches the estimations for these via the [`Provider::get_gas_price`],
/// [`Provider::estimate_gas`] and [`Provider::estimate_eip1559_fees`] methods.
///
/// If you use layers that redirect the behavior of [`Provider::send_transaction`] (e.g.
/// [`crate::layers::SignerLayer`]), you should add this layer before those.
///
/// Note:
///     - If none of the gas related fields are set, the layer first assumes it's a EIP-1559 tx and
///       populates the gas_limit, max_fee_per_gas and max_priority_fee_per_gas fields.
///     - If the network does not support EIP-1559, it will process as a legacy tx and populate the
///       gas_limit and gas_price fields.
///     - If the gas_price is already set by the user, it will process as a legacy tx and populate
///       the gas_limit field if unset.
///
/// # Example
///
/// ```rs
/// # async fn test<T: Transport + Clone, S: NetworkSigner<Ethereum>>(transport: T, signer: S) {
/// let provider = ProviderBuilder::new()
///     .layer(ManagedNonceLayer)
///     .layer(GasEstimatorLayer)
///     .signer(EthereumSigner::from(signer)) // note the order!
///     .provider(RootProvider::new(transport));
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # }
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

/// A provider that estimates gas for transactions.
///
/// Note: This provider requires the chain_id to be set in the transaction request if it's a
/// EIP1559.
///
/// You cannot construct this directly, use [`ProviderBuilder`] with a [`GasEstimatorLayer`].
///
/// [`ProviderBuilder`]: crate::ProviderBuilder
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
    /// Gets the gas_price to be used in legacy txs.
    async fn get_gas_price(&self) -> TransportResult<U256> {
        self.inner.get_gas_price().await
    }

    /// Gets the gas_limit to be used in txs.
    async fn get_gas_estimate(&self, tx: &N::TransactionRequest) -> TransportResult<U256> {
        self.inner.estimate_gas(tx, None).await
    }

    /// Gets the max_fee_per_gas and max_priority_fee_per_gas to be used in EIP-1559 txs.
    async fn get_eip1559_fees_estimate(&self) -> TransportResult<(U256, U256)> {
        self.inner.estimate_eip1559_fees(None).await
    }

    /// Populates the gas_limit, max_fee_per_gas and max_priority_fee_per_gas fields if unset.
    /// Requires the chain_id to be set in the transaction request to be processed as a EIP-1559 tx.
    /// If the network does not support EIP-1559, it will process it as a legacy tx.
    async fn handle_eip1559_tx<'a, 'b>(
        &'a self,
        tx: &'b mut N::TransactionRequest,
    ) -> Result<&'b mut N::TransactionRequest, TransportError> {
        let gas_estimate = self.get_gas_estimate(tx);
        let eip1559_fees = self.get_eip1559_fees_estimate();

        let (gas_estimate, eip1559_fees) = futures::join!(gas_estimate, eip1559_fees);

        gas_estimate.map(|gas_estimate| tx.set_gas_limit(gas_estimate))?;

        match eip1559_fees {
            Ok((max_fee_per_gas, max_priority_fee_per_gas)) => {
                tx.set_max_fee_per_gas(max_fee_per_gas);
                tx.set_max_priority_fee_per_gas(max_priority_fee_per_gas);
                Ok(tx)
            }
            Err(err) => {
                if err.is_transport_error()
                    && err.to_string() == *"EIP-1559 not activated".to_string()
                {
                    // If EIP-1559 is not activated, it will process as a legacy tx.
                    match self.handle_legacy_tx(tx).await {
                        Ok(tx) => Ok(tx),
                        Err(err) => Err(err),
                    }
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Populates the gas_price and only populates the gas_limit field if unset.
    /// This method always assumes that the gas_price is unset.
    async fn handle_legacy_tx<'a, 'b>(
        &'a self,
        tx: &'b mut N::TransactionRequest,
    ) -> Result<&'b mut N::TransactionRequest, TransportError> {
        let gas_price = self.get_gas_price();

        if tx.gas_limit().is_none() {
            let gas_estimate = self.get_gas_estimate(&tx);

            match futures::join!(gas_price, gas_estimate) {
                (Ok(gas_price), Ok(gas_estimate)) => {
                    tx.set_gas_price(gas_price);
                    tx.set_gas_limit(gas_estimate);
                    Ok(tx)
                }
                (Ok(_gas_price), Err(err)) => Err(err),
                (Err(err), Ok(_gas_estimate)) => Err(err),
                (Err(err1), Err(_err2)) => Err(err1),
            }
        } else {
            let gas_price = gas_price.await;
            match gas_price {
                Ok(gas_price) => {
                    tx.set_gas_price(gas_price);
                    Ok(tx)
                }
                Err(err) => Err(err),
            }
        }
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
            // Assume its a EIP1559 tx
            // Populate the following gas_limit, max_fee_per_gas and max_priority_fee_per_gas fields
            // if unset.
            self.handle_eip1559_tx(&mut tx).await?;
        } else {
            // Assume its a legacy tx
            // Populate only the gas_limit field if unset.
            self.handle_legacy_tx(&mut tx).await?;
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

        // GasEstimationLayer requires chain_id to be set to handle EIP-1559 tx
        let tx = TransactionRequest {
            from: Some(anvil.addresses()[0]),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            chain_id: Some(31337),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let tx = tx.get_receipt().await.unwrap();

        let set_gas_price = U128::from(0x3b9aca00);
        let set_gas_limit = U256::from(0x5208);

        assert_eq!(tx.effective_gas_price, set_gas_price);
        assert_eq!(tx.gas_used, Some(set_gas_limit));
    }

    #[tokio::test]
    async fn no_gas_limit() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .layer(ManagedNonceLayer)
            .layer(GasEstimatorLayer)
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest {
            from: Some(anvil.addresses()[0]),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(gas_price),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let tx = tx.get_receipt().await.unwrap();

        let set_gas_limit = U256::from(0x5208);

        assert_eq!(tx.gas_used, Some(set_gas_limit));
    }

    #[tokio::test]
    async fn non_eip1559_network() {
        let anvil = Anvil::new().arg("--hardfork").arg("frontier").spawn();
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
            // chain_id: Some(31337), Not required as this will fallback to legacy_tx
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let tx = tx.get_receipt().await.unwrap();

        let set_gas_price = U128::from(0x6fc23ac0);

        assert_eq!(tx.effective_gas_price, set_gas_price);
    }
}
