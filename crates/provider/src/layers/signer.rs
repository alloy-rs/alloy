use crate::{layers::TxFiller, PendingTransactionBuilder, Provider, ProviderLayer, RootProvider};
use alloy_network::{eip2718::Encodable2718, Ethereum, Network, NetworkSigner, TransactionBuilder};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use async_trait::async_trait;
use std::marker::PhantomData;

/// A layer that signs transactions locally.
///
/// The layer uses a [`NetworkSigner`] to sign transactions sent using
/// [`Provider::send_transaction`] locally before passing them to the node with
/// [`Provider::send_raw_transaction`].
///
/// If you have other layers that depend on [`Provider::send_transaction`] being invoked, add those
/// first.
///
/// # Example
///
/// ```rs
/// # async fn test<T: Transport + Clone, S: NetworkSigner<Ethereum>>(transport: T, signer: S) {
/// let provider = ProviderBuilder::new()
///     .signer(EthereumSigner::from(signer))
///     .provider(RootProvider::new(transport));
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # }
/// ```
#[derive(Debug)]
pub struct SignerLayer<S> {
    signer: S,
}

impl<S> SignerLayer<S> {
    /// Creates a new signing layer with the given signer.
    pub const fn new(signer: S) -> Self {
        Self { signer }
    }
}

impl<P, T, S, N> ProviderLayer<P, T, N> for SignerLayer<S>
where
    P: Provider<T, N>,
    T: Transport + Clone,
    S: NetworkSigner<N> + Clone,
    N: Network,
{
    type Provider = SignerProvider<T, P, S, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        SignerProvider { inner, signer: self.signer.clone(), _phantom: PhantomData }
    }
}

/// A locally-signing provider.
///
/// Signs transactions locally using a [`NetworkSigner`].
///
/// # Note
///
/// You cannot construct this provider directly. Use [`ProviderBuilder`] with a [`SignerLayer`].
///
/// [`ProviderBuilder`]: crate::ProviderBuilder
#[derive(Debug)]
pub struct SignerProvider<T, P, S, N = Ethereum>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    inner: P,
    signer: S,
    _phantom: PhantomData<(T, N)>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T, P, S, N> Provider<T, N> for SignerProvider<T, P, S, N>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    S: NetworkSigner<N>,
    N: Network,
{
    #[inline]
    fn root(&self) -> &RootProvider<T, N> {
        self.inner.root()
    }

    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, T, N>> {
        let envelope = tx.build(&self.signer).await.map_err(TransportErrorKind::custom)?;
        let rlp = envelope.encoded_2718();

        self.inner.send_raw_transaction(&rlp).await
    }
}

#[cfg(feature = "reqwest")]
#[cfg(test)]
mod tests {
    use crate::{Provider, ProviderBuilder, RootProvider};
    use alloy_network::EthereumSigner;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, b256, U256};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types::TransactionRequest;
    use alloy_transport_http::Http;
    use reqwest::Client;

    #[tokio::test]
    async fn poc() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

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
