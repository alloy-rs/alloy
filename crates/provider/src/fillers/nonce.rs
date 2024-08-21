use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::SendableTx,
    Provider,
};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_transport::{Transport, TransportResult};
use dashmap::DashMap;
use futures::lock::Mutex;
use std::sync::Arc;

/// A [`TxFiller`] that fills nonces on transactions.
///
/// The filler will fetch the transaction count for any new account it sees,
/// store it locally and increment the locally stored nonce as transactions are
/// sent via [`Provider::send_transaction`].
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
/// ```
/// # use alloy_network::{NetworkWallet, EthereumWallet, Ethereum};
/// # use alloy_rpc_types_eth::TransactionRequest;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # async fn test<W: NetworkWallet<Ethereum> + Clone>(url: url::Url, wallet: W) -> Result<(), Box<dyn std::error::Error>> {
/// let provider = ProviderBuilder::new()
///     .with_nonce_management()
///     .wallet(wallet)
///     .on_http(url);
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct NonceFiller {
    nonces: DashMap<Address, Arc<Mutex<u64>>>,
}

impl<N: Network> TxFiller<N> for NonceFiller {
    type Fillable = u64;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        if tx.nonce().is_some() {
            return FillerControlFlow::Finished;
        }
        if tx.from().is_none() {
            return FillerControlFlow::missing("NonceManager", vec!["from"]);
        }
        FillerControlFlow::Ready
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

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
    async fn get_next_nonce<P, T, N>(&self, provider: &P, address: Address) -> TransportResult<u64>
    where
        P: Provider<T, N>,
        N: Network,
        T: Transport + Clone,
    {
        // Use `u64::MAX` as a sentinel value to indicate that the nonce has not been fetched yet.
        const NONE: u64 = u64::MAX;

        // Locks dashmap internally for a short duration to clone the `Arc`.
        // We also don't want to hold the dashmap lock through the await point below.
        let nonce = {
            let rm = self.nonces.entry(address).or_insert_with(|| Arc::new(Mutex::new(NONE)));
            Arc::clone(rm.value())
        };

        let mut nonce = nonce.lock().await;
        let new_nonce = if *nonce == NONE {
            // Initialize the nonce if we haven't seen this account before.
            provider.get_transaction_count(address).await?
        } else {
            *nonce + 1
        };
        *nonce = new_nonce;
        Ok(new_nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProviderBuilder, WalletProvider};
    use alloy_consensus::Transaction;
    use alloy_primitives::{address, U256};
    use alloy_rpc_types_eth::TransactionRequest;

    async fn check_nonces<P, T, N>(filler: &NonceFiller, provider: &P, address: Address, start: u64)
    where
        P: Provider<T, N>,
        N: Network,
        T: Transport + Clone,
    {
        for i in start..start + 5 {
            let nonce = filler.get_next_nonce(&provider, address).await.unwrap();
            assert_eq!(nonce, i);
        }
    }

    #[tokio::test]
    async fn smoke_test() {
        let filler = NonceFiller::default();
        let provider = ProviderBuilder::new().on_anvil();
        let address = Address::ZERO;
        check_nonces(&filler, &provider, address, 0).await;

        #[cfg(feature = "anvil-api")]
        {
            use crate::ext::AnvilApi;
            filler.nonces.clear();
            provider.anvil_set_nonce(address, U256::from(69)).await.unwrap();
            check_nonces(&filler, &provider, address, 69).await;
        }
    }

    #[tokio::test]
    async fn concurrency() {
        let filler = Arc::new(NonceFiller::default());
        let provider = Arc::new(ProviderBuilder::new().on_anvil());
        let address = Address::ZERO;
        let tasks = (0..5)
            .map(|_| {
                let filler = Arc::clone(&filler);
                let provider = Arc::clone(&provider);
                tokio::spawn(async move { filler.get_next_nonce(&provider, address).await })
            })
            .collect::<Vec<_>>();

        let mut ns = Vec::new();
        for task in tasks {
            ns.push(task.await.unwrap().unwrap());
        }
        ns.sort_unstable();
        assert_eq!(ns, (0..5).collect::<Vec<_>>());

        assert_eq!(filler.nonces.len(), 1);
        assert_eq!(*filler.nonces.get(&address).unwrap().value().lock().await, 4);
    }

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
        let provider = ProviderBuilder::new().with_nonce_management().on_anvil_with_wallet();

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
        let mined_tx = provider
            .get_transaction_by_hash(tx_hash)
            .await
            .expect("failed to fetch tx")
            .expect("tx not included");
        assert_eq!(mined_tx.tx.nonce(), 0);

        let pending = provider.send_transaction(tx).await.unwrap();
        let tx_hash = pending.watch().await.unwrap();
        let mined_tx = provider
            .get_transaction_by_hash(tx_hash)
            .await
            .expect("fail to fetch tx")
            .expect("tx didn't finalize");
        assert_eq!(mined_tx.tx.nonce(), 1);
    }
}
