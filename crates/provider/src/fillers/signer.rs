use crate::{provider::SendableTx, Provider};
use alloy_json_rpc::RpcError;
use alloy_network::{Network, NetworkSigner, TransactionBuilder};
use alloy_transport::{Transport, TransportResult};

use super::{FillerControlFlow, TxFiller};

/// A layer that signs transactions locally.
///
/// The layer uses a [`NetworkSigner`] to sign transactions sent using
/// [`Provider::send_transaction`] locally before passing them to the node with
/// [`Provider::send_raw_transaction`].
///
/// # Example
///
/// ```
/// # use alloy_network::{NetworkSigner, EthereumSigner, Ethereum};
/// # use alloy_rpc_types::TransactionRequest;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # async fn test<S: NetworkSigner<Ethereum> + Clone>(url: url::Url, signer: S) -> Result<(), Box<dyn std::error::Error>> {
/// let provider = ProviderBuilder::new()
///     .signer(signer)
///     .on_http(url)?;
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct SignerFiller<S> {
    signer: S,
}

impl<S> AsRef<S> for SignerFiller<S> {
    fn as_ref(&self) -> &S {
        &self.signer
    }
}

impl<S> AsMut<S> for SignerFiller<S> {
    fn as_mut(&mut self) -> &mut S {
        &mut self.signer
    }
}

impl<S> SignerFiller<S> {
    /// Creates a new signing layer with the given signer.
    pub const fn new(signer: S) -> Self {
        Self { signer }
    }
}

impl<S, N> TxFiller<N> for SignerFiller<S>
where
    N: Network,
    S: NetworkSigner<N> + Clone,
{
    type Fillable = ();

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        if tx.from().is_none() {
            return FillerControlFlow::Ready;
        }

        if tx.can_build() {
            FillerControlFlow::Ready
        } else {
            // Blocked by #431
            // https://github.com/alloy-rs/alloy/pull/431
            FillerControlFlow::Missing(vec![("Signer", &["TODO"])])
        }
    }

    async fn prepare<P, T>(
        &self,
        _provider: &P,
        _tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        Ok(())
    }

    async fn fill(
        &self,
        _fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        let mut builder = match tx {
            SendableTx::Builder(builder) => builder,
            _ => return Ok(tx),
        };

        if builder.from().is_none() {
            builder.set_from(self.signer.default_signer_address());
            if !builder.can_build() {
                return Ok(SendableTx::Builder(builder));
            }
        }

        let envelope = builder.build(&self.signer).await.map_err(RpcError::local_usage)?;

        Ok(SendableTx::Envelope(envelope))
    }
}

#[cfg(feature = "reqwest")]
#[cfg(test)]
mod tests {
    use crate::{Provider, ProviderBuilder};
    use alloy_primitives::{address, b256, U256};
    use alloy_rpc_types::TransactionRequest;

    #[tokio::test]
    async fn poc() {
        let provider = ProviderBuilder::new().on_anvil_with_signer();

        let tx = TransactionRequest {
            nonce: Some(0),
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        };

        let builder = provider.send_transaction(tx).await.unwrap();
        let node_hash = *builder.tx_hash();
        assert_eq!(
            node_hash,
            b256!("eb56033eab0279c6e9b685a5ec55ea0ff8d06056b62b7f36974898d4fbb57e64")
        );

        let pending = builder.register().await.unwrap();
        let local_hash = *pending.tx_hash();
        assert_eq!(local_hash, node_hash);

        let local_hash2 = pending.await.unwrap();
        assert_eq!(local_hash2, node_hash);

        let receipt =
            provider.get_transaction_receipt(local_hash2).await.unwrap().expect("no receipt");
        let receipt_hash = receipt.transaction_hash;
        assert_eq!(receipt_hash, node_hash);
    }
}
