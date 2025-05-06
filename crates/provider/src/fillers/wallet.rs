use std::fmt::Debug;

use crate::{provider::SendableTx, Provider};
use alloy_json_rpc::RpcError;
use alloy_network::{Network, NetworkWallet, TransactionBuilder};
use alloy_transport::TransportResult;

use super::{FillerControlFlow, TxFiller};

/// A layer that signs transactions locally.
///
/// The layer uses a [`NetworkWallet`] to sign transactions sent using
/// [`Provider::send_transaction`] locally before passing them to the node with
/// [`Provider::send_raw_transaction`].
///
/// # Example
///
/// ```
/// # use alloy_network::{IntoWallet, EthereumWallet, Ethereum};
/// # use alloy_rpc_types_eth::TransactionRequest;
/// # use alloy_signer_local::PrivateKeySigner;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # async fn test(url: url::Url) -> Result<(), Box<dyn std::error::Error>> {
/// let pk: PrivateKeySigner = "0x...".parse()?;
/// let provider = ProviderBuilder::new().wallet(pk).connect_http(url);
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct WalletFiller<W> {
    wallet: W,
}

impl<W> AsRef<W> for WalletFiller<W> {
    fn as_ref(&self) -> &W {
        &self.wallet
    }
}

impl<W> AsMut<W> for WalletFiller<W> {
    fn as_mut(&mut self) -> &mut W {
        &mut self.wallet
    }
}

impl<W> WalletFiller<W> {
    /// Creates a new wallet layer with the given wallet.
    pub const fn new(wallet: W) -> Self {
        Self { wallet }
    }
}

impl<W, N> TxFiller<N> for WalletFiller<W>
where
    N: Network,
    W: NetworkWallet<N> + Clone,
{
    type Fillable = ();

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        if tx.from().is_none() {
            return FillerControlFlow::Ready;
        }

        match tx.complete_preferred() {
            Ok(_) => FillerControlFlow::Ready,
            Err(e) => FillerControlFlow::Missing(vec![("Wallet", e)]),
        }
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        if let Some(builder) = tx.as_mut_builder() {
            if builder.from().is_none() {
                builder.set_from(self.wallet.default_signer_address());
            }
        }
    }

    async fn prepare<P>(
        &self,
        _provider: &P,
        _tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        Ok(())
    }

    async fn fill(
        &self,
        _fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        let builder = match tx {
            SendableTx::Builder(builder) => builder,
            _ => return Ok(tx),
        };

        let envelope = builder.build(&self.wallet).await.map_err(RpcError::local_usage)?;

        Ok(SendableTx::Envelope(envelope))
    }

    async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        self.prepare_call_sync(tx)?;
        Ok(())
    }

    fn prepare_call_sync(
        &self,
        tx: &mut <N as Network>::TransactionRequest,
    ) -> TransportResult<()> {
        if tx.from().is_none() {
            tx.set_from(self.wallet.default_signer_address());
        }
        Ok(())
    }
}

#[cfg(feature = "reqwest")]
#[cfg(test)]
mod tests {
    use crate::{Provider, ProviderBuilder, WalletProvider};
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, b256, U256};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_signer_local::PrivateKeySigner;

    #[tokio::test]
    async fn poc() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

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
            b256!("4b56f1a6bdceb76d1b843e978c70ab88e38aa19f1a67be851b10ce4eec65b7d4")
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

    #[tokio::test]
    async fn ingest_pk_signer() {
        let pk: PrivateKeySigner =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".parse().unwrap();

        let anvil = Anvil::new().spawn();

        let provider = ProviderBuilder::new().wallet(pk.clone()).connect_http(anvil.endpoint_url());

        let tx = TransactionRequest {
            nonce: Some(0),
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        };

        let receipt = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();

        // Can access wallet via provider
        let wallet = provider.wallet();

        let default_address = wallet.default_signer().address();

        assert_eq!(pk.address(), default_address);
        assert_eq!(receipt.from, default_address);
    }
}
