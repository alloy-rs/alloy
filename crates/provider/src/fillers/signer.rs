use crate::{provider::SendableTx, Provider};
use alloy_json_rpc::RpcError;
use alloy_network::{Network, NetworkSigner, TransactionBuilder};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};

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
/// # async fn test<T: Transport + Clone, S: NetworkSigner<Ethereum>>(transport: T, signer: S) {
/// let provider = ProviderBuilder::new()
///     .signer(EthereumSigner::from(signer))
///     .provider(RootProvider::new(transport));
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SignerFiller<S> {
    signer: S,
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

    fn status(&self, _tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        todo!("check on if tx is buildable")
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
        panic!("This function should not be called. This is a bug. If you have not manually called SignerLayer::prepare, please file an issue.")
    }

    fn fill(&self, _fillable: Self::Fillable, _tx: &mut SendableTx<N>) {
        panic!("This function should not be called. This is a bug. If you have not manually called SignerLayer::prepare, please file an issue.")
    }

    async fn prepare_and_fill<P, T>(
        &self,
        _provider: &P,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        let builder = match tx {
            SendableTx::Builder(builder) => builder,
            _ => return Ok(tx),
        };

        let envelope = builder.build(&self.signer).await.map_err(|e| {
            RpcError::<TransportErrorKind>::make_err_resp(
                -42069,
                format!("failed to build transaction: {e}"),
            )
        })?;
        tx = SendableTx::Envelope(envelope);

        Ok(tx)
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
        let (provider, _anvil) = ProviderBuilder::new().on_anvil_with_signer();

        let tx = TransactionRequest {
            nonce: Some(0),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
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
