use crate::{
    layers::{FillProvider, FillerControlFlow, TxFiller},
    utils::Eip1559Estimation,
    Provider, ProviderLayer,
};
use alloy_json_rpc::RpcError;
use alloy_network::{Network, TransactionBuilder};
use alloy_rpc_types::BlockNumberOrTag;
use alloy_transport::{Transport, TransportResult};
use futures::FutureExt;

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
pub struct GasFillerConfig;

impl<P, T, N> ProviderLayer<P, T, N> for GasFillerConfig
where
    P: Provider<T, N>,
    T: alloy_transport::Transport + Clone,
    N: Network,
{
    type Provider = FillProvider<GasFiller, P, T, N>;
    fn layer(&self, inner: P) -> Self::Provider {
        FillProvider::new(inner, GasFiller)
    }
}

/// An enum over the different types of gas fillable.
#[allow(unreachable_pub)]
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasFillable {
    Legacy { gas_limit: u128, gas_price: u128 },
    Eip1559 { gas_limit: u128, estimate: Eip1559Estimation },
    Eip4844 { gas_limit: u128, estimate: Eip1559Estimation, max_fee_per_blob_gas: u128 },
}

/// A [`TxFiller`] that populates gas related fields in transaction requests if
/// unset.
#[derive(Debug, Clone, Copy, Default)]
pub struct GasFiller;

impl GasFiller {
    async fn prepare_legacy<P, T, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
        N: Network,
    {
        let gas_price_fut = if let Some(gas_price) = tx.gas_price() {
            async move { Ok(gas_price) }.left_future()
        } else {
            async { provider.get_gas_price().await }.right_future()
        };

        let gas_limit_fut = if let Some(gas_limit) = tx.gas_limit() {
            async move { Ok(gas_limit) }.left_future()
        } else {
            async { provider.estimate_gas(tx, None).await }.right_future()
        };

        let (gas_price, gas_limit) = futures::try_join!(gas_price_fut, gas_limit_fut)?;

        Ok(GasFillable::Legacy { gas_limit, gas_price })
    }

    async fn prepare_1559<P, T, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
        N: Network,
    {
        let gas_limit_fut = if let Some(gas_limit) = tx.gas_limit() {
            async move { Ok(gas_limit) }.left_future()
        } else {
            async { provider.estimate_gas(tx, None).await }.right_future()
        };

        let eip1559_fees_fut = if let (Some(max_fee_per_gas), Some(max_priority_fee_per_gas)) =
            (tx.max_fee_per_gas(), tx.max_priority_fee_per_gas())
        {
            async move { Ok(Eip1559Estimation { max_fee_per_gas, max_priority_fee_per_gas }) }
                .left_future()
        } else {
            async { provider.estimate_eip1559_fees(None).await }.right_future()
        };

        let (gas_limit, estimate) = futures::try_join!(gas_limit_fut, eip1559_fees_fut)?;

        Ok(GasFillable::Eip1559 { gas_limit, estimate })
    }

    async fn prepare_4844<P, T, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
        N: Network,
    {
        let gas_limit_fut = if let Some(gas_limit) = tx.gas_limit() {
            async move { Ok(gas_limit) }.left_future()
        } else {
            async { provider.estimate_gas(tx, None).await }.right_future()
        };

        let eip1559_fees_fut = if let (Some(max_fee_per_gas), Some(max_priority_fee_per_gas)) =
            (tx.max_fee_per_gas(), tx.max_priority_fee_per_gas())
        {
            async move { Ok(Eip1559Estimation { max_fee_per_gas, max_priority_fee_per_gas }) }
                .left_future()
        } else {
            async { provider.estimate_eip1559_fees(None).await }.right_future()
        };

        let max_fee_per_blob_gas_fut = if let Some(max_fee_per_blob_gas) = tx.max_fee_per_blob_gas()
        {
            async move { Ok(max_fee_per_blob_gas) }.left_future()
        } else {
            async {
                provider
                    .get_block_by_number(BlockNumberOrTag::Latest, false)
                    .await?
                    .ok_or(RpcError::NullResp)?
                    .header
                    .next_block_blob_fee()
                    .ok_or(RpcError::UnsupportedFeature("eip4844"))
            }
            .right_future()
        };

        let (gas_limit, estimate, max_fee_per_blob_gas) =
            futures::try_join!(gas_limit_fut, eip1559_fees_fut, max_fee_per_blob_gas_fut)?;

        Ok(GasFillable::Eip4844 { gas_limit, estimate, max_fee_per_blob_gas })
    }
}

impl<N: Network> TxFiller<N> for GasFiller {
    type Fillable = GasFillable;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        // legacy and eip2930 tx
        if tx.gas_price().is_some() && tx.gas_limit().is_some() {
            return FillerControlFlow::Finished;
        }
        // 4844
        if tx.max_fee_per_blob_gas().is_some()
            && tx.max_fee_per_gas().is_some()
            && tx.max_priority_fee_per_gas().is_some()
        {
            return FillerControlFlow::Finished;
        }
        // eip1559
        if tx.blob_sidecar().is_none()
            && tx.max_fee_per_gas().is_some()
            && tx.max_priority_fee_per_gas().is_some()
        {
            return FillerControlFlow::Finished;
        }
        FillerControlFlow::Ready
    }

    async fn prepare<P, T>(
        &self,
        provider: &P,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        if tx.gas_price().is_some() || tx.access_list().is_some() {
            self.prepare_legacy(provider, tx).await
        } else if tx.blob_sidecar().is_some() {
            self.prepare_4844(provider, tx).await
        } else {
            self.prepare_1559(provider, tx).await
        }
    }

    fn fill(&self, fillable: Self::Fillable, tx: &mut <N as Network>::TransactionRequest) {
        match fillable {
            GasFillable::Legacy { gas_limit, gas_price } => {
                tx.set_gas_limit(gas_limit);
                tx.set_gas_price(gas_price);
            }
            GasFillable::Eip1559 { gas_limit, estimate } => {
                tx.set_gas_limit(gas_limit);
                tx.set_max_fee_per_gas(estimate.max_fee_per_gas);
                tx.set_max_priority_fee_per_gas(estimate.max_priority_fee_per_gas);
            }
            GasFillable::Eip4844 { gas_limit, estimate, max_fee_per_blob_gas } => {
                tx.set_gas_limit(gas_limit);
                tx.set_max_fee_per_gas(estimate.max_fee_per_gas);
                tx.set_max_priority_fee_per_gas(estimate.max_priority_fee_per_gas);
                tx.set_max_fee_per_blob_gas(max_fee_per_blob_gas);
            }
        }
    }
}

#[cfg(feature = "reqwest")]
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
    async fn no_gas_price_or_limit() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .with_nonce_management()
            .with_gas_estimation()
            .signer(EthereumSigner::from(wallet))
            .on_http(url);

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
            .on_http(url);

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
            .on_http(url);

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
