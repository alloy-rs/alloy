use crate::{PendingTransactionBuilder, Provider, ProviderLayer, RootProvider};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_transport::{Transport, TransportResult};
use async_trait::async_trait;
use dashmap::DashMap;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::Mutex;

/// A layer that fills nonces on transactions.
///
/// The layer will fetch the transaction count for any new account it sees, store it locally and
/// increment the locally stored nonce as transactions are sent via [`Provider::send_transaction`].
///
/// If you use layers that redirect the behavior of [`Provider::send_transaction`] (e.g.
/// [`SignerLayer`]), you should add this layer before those.
///
/// # Note
///
/// - If the transaction request does not have a sender set, this layer will not fill nonces.
/// - Using two providers with their own nonce layer can potentially fill invalid nonces if
///   transactions are sent from the same address, as the next nonce to be used is cached internally
///   in the layer.
///
/// # Example
///
/// ```rs
/// # async fn test<T: Transport + Clone, S: NetworkSigner<Ethereum>>(transport: T, signer: S) {
/// let provider = ProviderBuilder::new()
///     .with_nonce_management()
///     .signer(EthereumSigner::from(signer)) // note the order!
///     .provider(RootProvider::new(transport));
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # }
/// ```
///
/// [`SignerLayer`]: crate::layers::SignerLayer
#[derive(Debug, Clone, Copy)]
pub struct NonceManagerLayer;

impl<P, N, T> ProviderLayer<P, N, T> for NonceManagerLayer
where
    P: Provider<N, T>,
    N: Network,
    T: Transport + Clone,
{
    type Provider = ManagedNonceProvider<N, T, P>;

    fn layer(&self, inner: P) -> Self::Provider {
        ManagedNonceProvider { inner, nonces: DashMap::default(), _phantom: PhantomData }
    }
}

/// A provider that manages account nonces.
///
/// Fills nonces for transaction requests if unset.
///
/// # Note
///
/// If the transaction requests do not have a sender set, this provider will not set nonces.
///
/// You cannot construct this provider directly. Use [`ProviderBuilder`] with a
/// [`NonceManagerLayer`].
///
/// [`ProviderBuilder`]: crate::ProviderBuilder
#[derive(Debug, Clone)]
pub struct ManagedNonceProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    inner: P,
    nonces: DashMap<Address, Arc<Mutex<Option<u64>>>>,
    _phantom: PhantomData<(N, T)>,
}

impl<N, T, P> ManagedNonceProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    async fn get_next_nonce(&self, from: Address) -> TransportResult<u64> {
        // locks dashmap internally for a short duration to clone the `Arc`
        let mutex = Arc::clone(self.nonces.entry(from).or_default().value());

        // locks the value (does not lock dashmap)
        let mut nonce = mutex.lock().await;
        match *nonce {
            Some(ref mut nonce) => {
                *nonce += 1;
                Ok(*nonce)
            }
            None => {
                // initialize the nonce if we haven't seen this account before
                let initial_nonce = self.inner.get_transaction_count(from, None).await?;
                *nonce = Some(initial_nonce.to());
                Ok(initial_nonce.to())
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<N, T, P> Provider<N, T> for ManagedNonceProvider<N, T, P>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
{
    #[inline]
    fn root(&self) -> &RootProvider<N, T> {
        self.inner.root()
    }

    async fn send_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<'_, N, T>> {
        if tx.nonce().is_none() {
            if let Some(from) = tx.from() {
                tx.set_nonce(self.get_next_nonce(from).await?);
            }
        }

        self.inner.send_transaction(tx).await
    }
}

#[cfg(feature = "http")]
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
    async fn no_nonce_if_sender_unset() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .with_nonce_management()
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(U256::from(20e9)),
            gas: Some(U256::from(21000)),
            ..Default::default()
        };

        // errors because signer layer expects nonce to be set, which it is not
        assert!(provider.send_transaction(tx.clone()).await.is_err());
    }

    #[tokio::test]
    async fn increments_nonce() {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);

        let wallet = alloy_signer_wallet::Wallet::from(anvil.keys()[0].clone());

        let provider = ProviderBuilder::new()
            .with_nonce_management()
            .signer(EthereumSigner::from(wallet))
            .provider(RootProvider::new(RpcClient::new(http, true)));

        let from = anvil.addresses()[0];
        let tx = TransactionRequest {
            from: Some(from),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(U256::from(20e9)),
            gas: Some(U256::from(21000)),
            ..Default::default()
        };

        let pending = provider.send_transaction(tx.clone()).await.unwrap();
        let tx_hash = pending.watch().await.unwrap();
        let mined_tx = provider.get_transaction_by_hash(tx_hash).await.expect("tx didn't finalize");
        assert_eq!(mined_tx.nonce, 0);

        let pending = provider.send_transaction(tx).await.unwrap();
        let tx_hash = pending.watch().await.unwrap();
        let mined_tx = provider.get_transaction_by_hash(tx_hash).await.expect("tx didn't finalize");
        assert_eq!(mined_tx.nonce, 1);
    }
}
