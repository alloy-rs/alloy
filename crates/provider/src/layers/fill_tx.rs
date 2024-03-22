use crate::{
    layers::{GasEstimatorProvider, ManagedNonceProvider},
    PendingTransactionBuilder, Provider, ProviderLayer, RootProvider,
};
use alloy_network::{Network, TransactionBuilder};
use alloy_transport::{Transport, TransportError, TransportErrorKind, TransportResult};
use async_trait::async_trait;
use futures::FutureExt;
use std::marker::PhantomData;

/// A layer that fills in missing transaction fields.
#[derive(Debug, Clone, Copy)]
pub struct FillTxLayer;

impl<P, N, T> ProviderLayer<P, N, T> for FillTxLayer
where
    P: Provider<N, T> + Clone,
    N: Network,
    T: Transport + Clone,
{
    type Provider = FillTxProvider<N, T, P>;

    fn layer(&self, inner: P) -> Self::Provider {
        let nonce_provider = ManagedNonceProvider::new(inner.clone());
        let gas_estimation_provider = GasEstimatorProvider::new(inner.clone());
        FillTxProvider { inner, nonce_provider, gas_estimation_provider, _phantom: PhantomData }
    }
}

/// A provider that fills in missing transaction fields.
#[derive(Debug, Clone)]
pub struct FillTxProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T> + Clone,
{
    inner: P,
    nonce_provider: ManagedNonceProvider<N, T, P>,
    gas_estimation_provider: GasEstimatorProvider<N, T, P>,
    _phantom: PhantomData<(N, T)>,
}

impl<N, T, P> FillTxProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T> + Clone,
{
    /// Fills in missing transaction fields.
    pub async fn fill_tx(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        let chain_id_fut = if let Some(chain_id) = tx.chain_id() {
            async move { Ok(chain_id) }.left_future()
        } else {
            async move { self.inner.get_chain_id().await.map(|ci| ci.to::<u64>()) }.right_future()
        };

        // Check if `from` is set
        if tx.nonce().is_none() && tx.from().is_none() {
            return Err(TransportError::Transport(TransportErrorKind::Custom(
                "`from` field must be set in transaction request to populate the `nonce` field"
                    .into(),
            )));
        }

        let nonce_fut = if let Some(nonce) = tx.nonce() {
            async move { Ok(nonce) }.left_future()
        } else {
            let from = tx.from().unwrap();
            async move { self.nonce_provider.get_next_nonce(from).await }.right_future()
        };

        let gas_estimation_fut = if tx.gas_price().is_none() {
            async { self.gas_estimation_provider.handle_eip1559_tx(tx).await }.left_future()
        } else {
            async { self.gas_estimation_provider.handle_legacy_tx(tx).await }.right_future()
        };

        match futures::try_join!(chain_id_fut, nonce_fut, gas_estimation_fut) {
            Ok((chain_id, nonce, _)) => {
                tx.set_chain_id(chain_id);
                tx.set_nonce(nonce);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<N, T, P> Provider<N, T> for FillTxProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T> + Clone,
{
    #[inline]
    fn root(&self) -> &RootProvider<N, T> {
        self.inner.root()
    }

    async fn send_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, N, T>> {
        self.fill_tx(&mut tx).await?;
        self.inner.send_transaction(tx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_network::EthereumSigner;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, U256};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types::TransactionRequest;
    use alloy_transport_http::Http;
    use reqwest::Client;

    #[tokio::test]
    async fn test_1559_tx_no_nonce_no_chain_id() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint_url();

        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .layer(FillTxLayer)
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let tx = TransactionRequest {
            from: Some(anvil.addresses()[0]),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let tx = provider.get_transaction_by_hash(tx.tx_hash().to_owned()).await.unwrap();

        assert_eq!(tx.max_fee_per_gas, Some(U256::from(0x77359400)));
        assert_eq!(tx.max_priority_fee_per_gas, Some(U256::from(0x0)));
        assert_eq!(tx.gas, U256::from(21000));
        assert_eq!(tx.nonce, 0);
        assert_eq!(tx.chain_id, Some(31337));
    }

    #[tokio::test]
    async fn test_legacy_tx_no_nonce_chain_id() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint_url();

        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .layer(FillTxLayer)
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest {
            from: Some(anvil.addresses()[0]),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            chain_id: Some(31337),
            gas_price: Some(gas_price),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let tx = provider.get_transaction_by_hash(tx.tx_hash().to_owned()).await.unwrap();

        assert_eq!(tx.gas_price, Some(gas_price));
        assert_eq!(tx.gas, U256::from(21000));
        assert_eq!(tx.nonce, 0);
        assert_eq!(tx.chain_id, Some(31337));
    }

    #[tokio::test]
    #[should_panic]
    async fn test_no_from() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint_url();

        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .layer(FillTxLayer)
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            ..Default::default()
        };

        let _ = provider.send_transaction(tx).await.unwrap();
    }
}
