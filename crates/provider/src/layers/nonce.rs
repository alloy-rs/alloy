use crate::{
    layers::{FillerControlFlow, TxFiller},
    Provider,
};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_transport::{Transport, TransportResult};
use dashmap::DashMap;
use std::sync::Arc;
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
pub struct NonceFillerConfig;

/// A [`TxFiller`] that fills the nonce on transactions by keeping a local
/// mapping of account addresses to their next nonce.
#[derive(Debug, Clone, Default)]
pub struct NonceFiller {
    nonces: DashMap<Address, Arc<Mutex<Option<u64>>>>,
}

impl<N: Network> TxFiller<N> for NonceFiller {
    type Fillable = u64;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        if tx.nonce().is_some() {
            return FillerControlFlow::Finished;
        }
        if tx.from().is_none() {
            return FillerControlFlow::missing("NonceManager", &["from"]);
        }
        FillerControlFlow::Ready
    }

    async fn prepare<P, T>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<T, N>,
        T: Transport + Clone,
    {
        let from = tx.from().expect("checked by 'ready()'");
        self.get_next_nonce(provider, from).await
    }

    fn fill(&self, nonce: Self::Fillable, tx: &mut N::TransactionRequest) {
        tx.set_nonce(nonce);
    }
}

impl NonceFiller {
    /// Get the next nonce for the given account.
    async fn get_next_nonce<P, T, N>(&self, provider: &P, from: Address) -> TransportResult<u64>
    where
        P: Provider<T, N>,
        N: Network,
        T: Transport + Clone,
    {
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
                let initial_nonce = provider.get_transaction_count(from, None).await?;
                *nonce = Some(initial_nonce);
                Ok(initial_nonce)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_primitives::{address, U256};
    use alloy_rpc_types::TransactionRequest;

    #[tokio::test]
    async fn no_nonce_if_sender_unset() {
        let (provider, _anvil) =
            ProviderBuilder::new().with_nonce_management().on_anvil_with_signer();

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        };

        // errors because signer layer expects nonce to be set, which it is not
        assert!(provider.send_transaction(tx.clone()).await.is_err());
    }

    #[tokio::test]
    async fn increments_nonce() {
        let (provider, anvil) =
            ProviderBuilder::new().with_nonce_management().on_anvil_with_signer();

        let from = anvil.addresses()[0];
        let tx = TransactionRequest {
            from: Some(from),
            value: Some(U256::from(100)),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
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
