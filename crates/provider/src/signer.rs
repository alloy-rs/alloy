use crate::{PendingTransaction, PendingTransactionConfig, Provider, ProviderLayer};
use alloy_network::{eip2718::Encodable2718, Network, NetworkSigner, TransactionBuilder};
use alloy_rpc_client::{ClientRef, WeakClient};
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
/// let provider = ProviderBuilder::<_, Ethereum>::new()
///     .layer(SignerLayer::new(EthereumSigner::from(signer)))
///     .network::<Ethereum>()
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

impl<P, N, T, S> ProviderLayer<P, N, T> for SignerLayer<S>
where
    P: Provider<N, T>,
    N: Network,
    T: Transport + Clone,
    S: NetworkSigner<N> + Clone,
{
    type Provider = SignerProvider<N, T, P, S>;

    fn layer(&self, inner: P) -> Self::Provider {
        SignerProvider { inner, signer: self.signer.clone(), _phantom: PhantomData }
    }
}

/// A locally-signing provider.
///
/// Signs transactions locally using a [`NetworkSigner`]
///
/// # Note
///
/// You cannot construct this provider directly. Use [`ProviderBuilder`] with a [`SignerLayer`].
///
/// [`ProviderBuilder`]: crate::ProviderBuilder
#[derive(Debug)]
pub struct SignerProvider<N, T, P, S>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    inner: P,
    signer: S,
    _phantom: PhantomData<(N, T)>,
}

#[async_trait]
impl<N, T, P, S> Provider<N, T> for SignerProvider<N, T, P, S>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
    S: NetworkSigner<N>,
{
    fn client(&self) -> ClientRef<'_, T> {
        self.inner.client()
    }

    fn weak_client(&self) -> WeakClient<T> {
        self.inner.weak_client()
    }

    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> TransportResult<PendingTransaction> {
        self.inner.watch_pending_transaction(config).await
    }

    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionConfig> {
        let envelope = tx.build(&self.signer).await.map_err(TransportErrorKind::custom)?;
        let rlp = envelope.encoded_2718();

        self.inner.send_raw_transaction(&rlp).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Provider, ProviderBuilder, RootProvider};
    use alloy_network::{Ethereum, EthereumSigner};
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, b256, U256, U64};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types::TransactionRequest;
    use alloy_transport_http::Http;
    use reqwest::Client;

    #[tokio::test]
    async fn poc() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer::Wallet::from(anvil.keys()[0].clone());

        // can we somehow remove the need for <_, Ethereum>? we NEED to call .network<Ethereum>
        // note: we need to 1) add <_, Ethereum> 2) layer things, and then 3) call .network before
        // we can call provider
        let provider = ProviderBuilder::<_, Ethereum>::new()
            .signer(EthereumSigner::from(wallet))
            .network::<Ethereum>()
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let tx = TransactionRequest {
            nonce: Some(U64::from(0)),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(U256::from(20e9)),
            gas: Some(U256::from(21000)),
            ..Default::default()
        };

        let config = provider.send_transaction(tx).await.unwrap();
        let node_hash = *config.tx_hash();
        assert_eq!(
            node_hash,
            b256!("eb56033eab0279c6e9b685a5ec55ea0ff8d06056b62b7f36974898d4fbb57e64")
        );

        let pending = config.with_provider(&provider).register().await.unwrap();
        let local_hash = *pending.tx_hash();
        assert_eq!(local_hash, node_hash);

        let local_hash2 = pending.await.unwrap();
        assert_eq!(local_hash2, node_hash);

        let receipt =
            provider.get_transaction_receipt(local_hash2).await.unwrap().expect("no receipt");
        let receipt_hash = receipt.transaction_hash.expect("no receipt hash");
        assert_eq!(receipt_hash, node_hash);
    }
}
