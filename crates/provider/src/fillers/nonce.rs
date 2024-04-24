use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::SendableTx,
    Provider,
};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_rpc_types::BlockId;
use alloy_transport::{Transport, TransportResult};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A [`TxFiller`] that fills nonces on transactions.
///
/// The filler will fetch the transaction count for any new account it sees,
/// store it locally and increment the locally stored nonce as transactions are
/// sent via [`Provider::send_transaction`].
///
/// # Note
///
/// - If the transaction request does not have a sender set, this layer will
///  not fill nonces.
/// - Using two providers with their own nonce layer can potentially fill
///  invalid nonces if transactions are sent from the same address, as the next
///  nonce to be used is cached internally in the layer.
///
/// # Example
///
/// ```
/// # use alloy_network::{NetworkSigner, EthereumSigner, Ethereum};
/// # use alloy_rpc_types::TransactionRequest;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # async fn test<S: NetworkSigner<Ethereum> + Clone>(url: url::Url, signer: S) -> Result<(), Box<dyn std::error::Error>> {
/// let provider = ProviderBuilder::new()
///     .with_nonce_management()
///     .signer(signer)
///     .on_http(url)?;
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
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

    async fn fill(
        &self,
        nonce: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            builder.set_nonce(nonce);
        }
        Ok(tx)
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
                let initial_nonce =
                    provider.get_transaction_count(from, BlockId::default()).await?;
                *nonce = Some(initial_nonce);
                Ok(initial_nonce)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProviderBuilder, WalletProvider};
    use alloy_primitives::{address, U256};
    use alloy_rpc_types::TransactionRequest;

    #[tokio::test]
    async fn no_nonce_if_sender_unset() {
        let provider = ProviderBuilder::new().with_nonce_management().on_anvil();

        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        };

        // errors because signer layer expects nonce to be set, which it is not
        assert!(provider.send_transaction(tx).await.is_err());
    }

    #[tokio::test]
    async fn increments_nonce() {
        let provider = ProviderBuilder::new().with_nonce_management().on_anvil_with_signer();

        let from = provider.default_signer_address();
        let tx = TransactionRequest {
            from: Some(from),
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
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
