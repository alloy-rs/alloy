use std::{
    fmt::{self, Formatter},
    future::IntoFuture,
    sync::Arc,
};

use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::SendableTx,
    utils::Eip1559Estimation,
    Provider,
};
use alloy_eips::eip4844::BLOB_TX_MIN_BLOB_GASPRICE;
use alloy_json_rpc::RpcError;
use alloy_network::{Network, TransactionBuilder, TransactionBuilder4844};
use alloy_rpc_types_eth::BlockNumberOrTag;
use alloy_transport::TransportResult;
use futures::FutureExt;

/// An enum over the different types of gas fillable.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasFillable {
    Legacy { gas_limit: u64, gas_price: u128 },
    Eip1559 { gas_limit: u64, estimate: Eip1559Estimation },
}

/// A [`TxFiller`] that populates gas related fields in transaction requests if
/// unset.
///
/// Gas related fields are gas_price, gas_limit, max_fee_per_gas
/// and max_priority_fee_per_gas. For EIP-4844 `max_fee_per_blob_gas`,
/// see [`BlobGasFiller`].
///
/// The layer fetches the estimations for these via the
/// [`Provider::get_gas_price`], [`Provider::estimate_gas`] and
/// [`Provider::estimate_eip1559_fees`] methods.
///
/// ## Note:
///
/// The layer will populate gas fields based on the following logic:
/// - if `gas_price` is set, it will process as a legacy tx (or EIP-2930 if `access_list` is also
///   set) and populate the `gas_limit` field if unset, and `gas_price` if unset for EIP-2930.
/// - if `access_list` is set but `gas_price` is not set, it will process as an EIP-1559 tx (which
///   can also have an access_list) and populate the `gas_limit`, `max_fee_per_gas` and
///   `max_priority_fee_per_gas` fields if unset.
/// - if `blob_sidecar` is set, it will process as an EIP-4844 tx and populate the `gas_limit`,
///   `max_fee_per_gas`, and `max_priority_fee_per_gas` fields if unset. The `max_fee_per_blob_gas`
///   is populated by [`BlobGasFiller`].
/// - Otherwise, it will process as a EIP-1559 tx and populate the `gas_limit`, `max_fee_per_gas`
///   and `max_priority_fee_per_gas` fields if unset.
/// - If the network does not support EIP-1559, it will fallback to the legacy tx and populate the
///   `gas_limit` and `gas_price` fields if unset.
///
/// # Example
///
/// ```
/// # use alloy_network::{Ethereum};
/// # use alloy_rpc_types_eth::TransactionRequest;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # use alloy_signer_local::PrivateKeySigner;
/// # async fn test(url: url::Url) -> Result<(), Box<dyn std::error::Error>> {
/// let pk: PrivateKeySigner = "0x...".parse()?;
/// let provider = ProviderBuilder::<_, _, Ethereum>::default()
///     .with_gas_estimation()
///     .wallet(pk)
///     .connect_http(url);
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct GasFiller;

impl GasFiller {
    async fn prepare_legacy<P, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<N>,
        N: Network,
    {
        let gas_price_fut = tx.gas_price().map_or_else(
            || provider.get_gas_price().right_future(),
            |gas_price| async move { Ok(gas_price) }.left_future(),
        );

        let gas_limit_fut = tx.gas_limit().map_or_else(
            || provider.estimate_gas(tx.clone()).into_future().right_future(),
            |gas_limit| async move { Ok(gas_limit) }.left_future(),
        );

        let (gas_price, gas_limit) = futures::try_join!(gas_price_fut, gas_limit_fut)?;

        Ok(GasFillable::Legacy { gas_limit, gas_price })
    }

    async fn prepare_1559<P, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<N>,
        N: Network,
    {
        let gas_limit_fut = tx.gas_limit().map_or_else(
            || provider.estimate_gas(tx.clone()).into_future().right_future(),
            |gas_limit| async move { Ok(gas_limit) }.left_future(),
        );

        let eip1559_fees_fut = if let (Some(max_fee_per_gas), Some(max_priority_fee_per_gas)) =
            (tx.max_fee_per_gas(), tx.max_priority_fee_per_gas())
        {
            async move { Ok(Eip1559Estimation { max_fee_per_gas, max_priority_fee_per_gas }) }
                .left_future()
        } else {
            provider.estimate_eip1559_fees().right_future()
        };

        let (gas_limit, estimate) = futures::try_join!(gas_limit_fut, eip1559_fees_fut)?;

        Ok(GasFillable::Eip1559 { gas_limit, estimate })
    }
}

impl<N: Network> TxFiller<N> for GasFiller {
    type Fillable = GasFillable;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        // legacy and eip2930 tx
        if tx.gas_price().is_some() && tx.gas_limit().is_some() {
            return FillerControlFlow::Finished;
        }

        // eip1559
        if tx.max_fee_per_gas().is_some()
            && tx.max_priority_fee_per_gas().is_some()
            && tx.gas_limit().is_some()
        {
            return FillerControlFlow::Finished;
        }

        FillerControlFlow::Ready
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        if tx.gas_price().is_some() {
            self.prepare_legacy(provider, tx).await
        } else {
            match self.prepare_1559(provider, tx).await {
                // fallback to legacy
                Ok(estimate) => Ok(estimate),
                Err(RpcError::UnsupportedFeature(_)) => self.prepare_legacy(provider, tx).await,
                Err(e) => Err(e),
            }
        }
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            match fillable {
                GasFillable::Legacy { gas_limit, gas_price } => {
                    builder.set_gas_limit(gas_limit);
                    builder.set_gas_price(gas_price);
                }
                GasFillable::Eip1559 { gas_limit, estimate } => {
                    builder.set_gas_limit(gas_limit);
                    builder.set_max_fee_per_gas(estimate.max_fee_per_gas);
                    builder.set_max_priority_fee_per_gas(estimate.max_priority_fee_per_gas);
                }
            }
        };
        Ok(tx)
    }
}

/// An estimator function for blob gas fees.
pub type BlobGasEstimatorFunction = fn(u128, &[f64]) -> u128;

/// A trait responsible for estimating blob gas values
pub trait BlobGasEstimatorFn: Send + Sync + Unpin {
    /// Estimates the blob gas fee given the base fee per blob gas
    /// and the blob gas usage ratio.
    fn estimate(&self, base_fee_per_blob_gas: u128, blob_gas_used_ratio: &[f64]) -> u128;
}

/// Blob Gas estimator variants
#[derive(Default, Clone)]
pub enum BlobGasEstimator {
    /// Uses the builtin estimator
    #[default]
    Default,
    /// Uses a custom estimator
    Custom(Arc<dyn BlobGasEstimatorFn>),
}

impl BlobGasEstimator {
    /// Creates a new estimator from a closure
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(u128, &[f64]) -> u128 + Send + Sync + Unpin + 'static,
    {
        Self::new_estimator(f)
    }

    /// Creates a new estimate fn
    pub fn new_estimator<F: BlobGasEstimatorFn + 'static>(f: F) -> Self {
        Self::Custom(Arc::new(f))
    }

    /// Create a custom estimator
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(u128, &[f64]) -> u128 + Send + Sync + Unpin + 'static,
    {
        Self::Custom(Arc::new(f))
    }

    /// Create a scaled estimator
    pub fn scaled(scale: u128) -> Self {
        Self::custom(move |base_fee, _| base_fee.saturating_mul(scale))
    }

    /// Estimates the blob gas fee given the base fee per blob gas
    /// and the blob gas usage ratio.
    pub fn estimate(&self, base_fee_per_blob_gas: u128, blob_gas_used_ratio: &[f64]) -> u128 {
        match self {
            Self::Default => base_fee_per_blob_gas,
            Self::Custom(val) => val.estimate(base_fee_per_blob_gas, blob_gas_used_ratio),
        }
    }
}

impl<F> BlobGasEstimatorFn for F
where
    F: Fn(u128, &[f64]) -> u128 + Send + Sync + Unpin,
{
    fn estimate(&self, base_fee_per_blob_gas: u128, blob_gas_used_ratio: &[f64]) -> u128 {
        (self)(base_fee_per_blob_gas, blob_gas_used_ratio)
    }
}

impl fmt::Debug for BlobGasEstimator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlobGasEstimator")
            .field(
                "estimator",
                &match self {
                    Self::Default => "default",
                    Self::Custom(_) => "custom",
                },
            )
            .finish()
    }
}

/// Filler for the `max_fee_per_blob_gas` field in EIP-4844 transactions.
#[derive(Clone, Debug, Default)]
pub struct BlobGasFiller {
    /// The blob gas estimator to use.
    pub estimator: BlobGasEstimator,
}

impl<N: Network> TxFiller<N> for BlobGasFiller
where
    N::TransactionRequest: TransactionBuilder4844,
{
    type Fillable = u128;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        // Nothing to fill if non-eip4844 tx or `max_fee_per_blob_gas` is already set to a valid
        // value.
        if tx.blob_sidecar().is_none()
            || tx.max_fee_per_blob_gas().is_some_and(|gas| gas >= BLOB_TX_MIN_BLOB_GASPRICE)
        {
            return FillerControlFlow::Finished;
        }

        FillerControlFlow::Ready
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        if let Some(max_fee_per_blob_gas) = tx.max_fee_per_blob_gas() {
            if max_fee_per_blob_gas >= BLOB_TX_MIN_BLOB_GASPRICE {
                return Ok(max_fee_per_blob_gas);
            }
        }

        // Fetch the latest fee_history
        let fee_history = provider.get_fee_history(2, BlockNumberOrTag::Latest, &[]).await?;

        let base_fee_per_blob_gas =
            fee_history.base_fee_per_blob_gas.last().ok_or(RpcError::NullResp).copied()?;

        let blob_gas_used_ratio = fee_history.blob_gas_used_ratio;

        Ok(self.estimator.estimate(base_fee_per_blob_gas, &blob_gas_used_ratio))
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            builder.set_max_fee_per_blob_gas(fillable);
        }
        Ok(tx)
    }
}

#[cfg(feature = "reqwest")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_consensus::{SidecarBuilder, SimpleCoder, Transaction};
    use alloy_eips::eip4844::DATA_GAS_PER_BLOB;
    use alloy_primitives::{address, U256};
    use alloy_rpc_types_eth::TransactionRequest;

    #[tokio::test]
    async fn no_gas_price_or_limit() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        // GasEstimationLayer requires chain_id to be set to handle EIP-1559 tx
        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            chain_id: Some(31337),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        assert_eq!(receipt.effective_gas_price, 1_000_000_001);
        assert_eq!(receipt.gas_used, 21000);
    }

    #[tokio::test]
    async fn no_gas_limit() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(gas_price),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        assert_eq!(receipt.gas_used, 21000);
    }

    #[tokio::test]
    async fn no_max_fee_per_blob_gas() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let sidecar: SidecarBuilder<SimpleCoder> = SidecarBuilder::from_slice(b"Hello World");
        let sidecar = sidecar.build_4844().unwrap();

        let tx = TransactionRequest {
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            sidecar: Some(sidecar.into()),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        let tx = provider.get_transaction_by_hash(receipt.transaction_hash).await.unwrap().unwrap();

        assert!(tx.max_fee_per_blob_gas().unwrap() >= BLOB_TX_MIN_BLOB_GASPRICE);
        assert_eq!(receipt.gas_used, 21000);
        assert_eq!(
            receipt.blob_gas_used.expect("Expected to be EIP-4844 transaction"),
            DATA_GAS_PER_BLOB
        );
    }

    #[tokio::test]
    async fn zero_max_fee_per_blob_gas() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let sidecar: SidecarBuilder<SimpleCoder> = SidecarBuilder::from_slice(b"Hello World");
        let sidecar = sidecar.build_4844().unwrap();

        let tx = TransactionRequest {
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            max_fee_per_blob_gas: Some(0),
            sidecar: Some(sidecar.into()),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        let tx = provider.get_transaction_by_hash(receipt.transaction_hash).await.unwrap().unwrap();

        assert!(tx.max_fee_per_blob_gas().unwrap() >= BLOB_TX_MIN_BLOB_GASPRICE);
        assert_eq!(receipt.gas_used, 21000);
        assert_eq!(
            receipt.blob_gas_used.expect("Expected to be EIP-4844 transaction"),
            DATA_GAS_PER_BLOB
        );
    }
}
