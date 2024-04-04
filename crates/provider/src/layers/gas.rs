use crate::{
    utils::Eip1559Estimation, PendingTransactionBuilder, Provider, ProviderLayer, RootProvider,
};
use alloy_json_rpc::RpcError;
use alloy_network::{Block, Header, Network, TransactionBuilder};
use alloy_primitives::U256;
use alloy_rpc_types::BlockNumberOrTag;
use alloy_transport::{Transport, TransportError, TransportResult};
use async_trait::async_trait;
use futures::FutureExt;
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
///     .with_nonce_management()
///     .with_gas_estimation()
///     .signer(EthereumSigner::from(signer)) // note the order!
///     .provider(RootProvider::new(transport));
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # }
#[derive(Debug, Clone, Copy, Default)]
pub struct GasEstimatorLayer;

impl<P, T, N> ProviderLayer<P, T, N> for GasEstimatorLayer
where
    P: Provider<T, N>,
    N: Network,
    T: Transport + Clone,
{
    type Provider = GasEstimatorProvider<T, P, N>;
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
pub struct GasEstimatorProvider<T, P, N>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    inner: P,
    _phantom: PhantomData<(N, T)>,
}

impl<T, P, N> GasEstimatorProvider<T, P, N>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
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
    async fn get_eip1559_fees_estimate(&self) -> TransportResult<Eip1559Estimation> {
        self.inner.estimate_eip1559_fees(None).await
    }

    /// Populates the gas_limit, max_fee_per_gas and max_priority_fee_per_gas fields if unset.
    /// Requires the chain_id to be set in the transaction request to be processed as a EIP-1559 tx.
    /// If the network does not support EIP-1559, it will process it as a legacy tx.
    async fn handle_eip1559_tx(
        &self,
        tx: &mut N::TransactionRequest,
    ) -> Result<(), TransportError> {
        let gas_estimate_fut = if let Some(gas_limit) = tx.gas_limit() {
            async move { Ok(gas_limit) }.left_future()
        } else {
            async { self.get_gas_estimate(tx).await }.right_future()
        };

        let eip1559_fees_fut = if let (Some(max_fee_per_gas), Some(max_priority_fee_per_gas)) =
            (tx.max_fee_per_gas(), tx.max_priority_fee_per_gas())
        {
            async move { Ok(Eip1559Estimation { max_fee_per_gas, max_priority_fee_per_gas }) }
                .left_future()
        } else {
            async { self.get_eip1559_fees_estimate().await }.right_future()
        };

        match futures::try_join!(gas_estimate_fut, eip1559_fees_fut) {
            Ok((gas_limit, eip1559_fees)) => {
                tx.set_gas_limit(gas_limit);
                tx.set_max_fee_per_gas(eip1559_fees.max_fee_per_gas);
                tx.set_max_priority_fee_per_gas(eip1559_fees.max_priority_fee_per_gas);
                Ok(())
            }
            Err(RpcError::UnsupportedFeature("eip1559")) => self.handle_legacy_tx(tx).await,
            Err(e) => Err(e),
        }
    }

    /// Populates the gas_price and only populates the gas_limit field if unset.
    /// This method always assumes that the gas_price is unset.
    async fn handle_legacy_tx(&self, tx: &mut N::TransactionRequest) -> Result<(), TransportError> {
        let gas_price_fut = self.get_gas_price();
        let gas_limit_fut = if let Some(gas_limit) = tx.gas_limit() {
            async move { Ok(gas_limit) }.left_future()
        } else {
            async { self.get_gas_estimate(tx).await }.right_future()
        };

        futures::try_join!(gas_price_fut, gas_limit_fut).map(|(gas_price, gas_limit)| {
            tx.set_gas_price(gas_price);
            tx.set_gas_limit(gas_limit);
            tx
        })?;

        Ok(())
    }

    /// There are a few ways to obtain the blob base fee for an EIP-4844 transaction:
    ///
    /// * `eth_blobBaseFee`: Returns the fee for the next block directly.
    /// * `eth_feeHistory`: Returns the same info as for the EIP-1559 fees.
    /// * retrieving it from the "pending" block directly.
    ///
    /// At the time of this writing support for EIP-4844 fees is lacking, hence we're defaulting to
    /// requesting the fee from the "pending" block.
    async fn handle_eip4844_tx(
        &self,
        tx: &mut N::TransactionRequest,
    ) -> Result<(), TransportError> {
        // TODO this can be optimized together with 1559 dynamic fees once blob fee support on
        // eth_feeHistory is more widely supported
        if tx.get_blob_sidecar().is_some() && tx.max_fee_per_blob_gas().is_none() {
            let next_blob_fee = self
                .inner
                .get_block_by_number(BlockNumberOrTag::Latest, false)
                .await?
                .ok_or(RpcError::NullResp)?
                .header()
                .next_block_blob_fee()
                .ok_or(RpcError::UnsupportedFeature("eip4844"))?;
            tx.set_max_fee_per_blob_gas(U256::from(next_blob_fee));
        }

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T, P, N> Provider<T, N> for GasEstimatorProvider<T, P, N>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    fn root(&self) -> &RootProvider<T, N> {
        self.inner.root()
    }

    async fn send_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        if tx.gas_price().is_none() {
            // Assume its a EIP1559 tx
            // Populate the following gas_limit, max_fee_per_gas and max_priority_fee_per_gas fields
            // if unset.
            self.handle_eip1559_tx(&mut tx).await?;
            // TODO: this can be done more elegantly once we can set EIP-1559 and EIP-4844 fields
            // with a single eth_feeHistory request
            self.handle_eip4844_tx(&mut tx).await?;
        } else {
            // Assume its a legacy tx
            // Populate only the gas_limit field if unset.
            self.handle_legacy_tx(&mut tx).await?;
        }
        self.inner.send_transaction(tx).await
    }
}

#[cfg(feature = "reqwest")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_network::EthereumSigner;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::address;
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
            .with_nonce_management()
            .with_gas_estimation()
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

        assert_eq!(tx.effective_gas_price, 0x3b9aca00);
        assert_eq!(tx.gas_used, Some(0x5208));
    }

    #[tokio::test]
    async fn no_gas_limit() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .with_nonce_management()
            .with_gas_estimation()
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

        assert_eq!(tx.gas_used, Some(0x5208));
    }

    #[tokio::test]
    async fn non_eip1559_network() {
        let anvil = Anvil::new().arg("--hardfork").arg("frontier").spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .with_nonce_management()
            .with_gas_estimation()
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

        assert_eq!(tx.effective_gas_price, 0x6fc23ac0);
    }
}
