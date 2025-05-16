use alloy_eips::Decodable2718;
use alloy_network::{Ethereum, Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes};

use super::Provider;

/// A remote signer that leverages the underlying provider to sign transactions using
/// `"eth_signTransaction"` requests.
///
/// For more information, please see [Web3Signer](https://docs.web3signer.consensys.io/)
///
/// Note:
///
/// `"eth_signTransaction"` is not supported by regular nodes.
///
/// [`ProviderBuilder`]: crate::ProviderBuilder
#[derive(Debug, Clone)]
pub struct Web3Signer<P: Provider<N> + Clone, N: Network = Ethereum> {
    /// The provider used to make `"eth_signTransaction"` requests.
    provider: P,
    /// The address of the remote signer that will sign the transactions.
    ///
    /// This is set as the `from` field in the [`Network::TransactionRequest`]'s for the
    /// `"eth_signTransaction"` requests.
    address: Address,
    _pd: std::marker::PhantomData<N>,
}

impl<P: Provider<N> + Clone, N: Network> Web3Signer<P, N> {
    /// Instantiates a new [`Web3Signer`] with the given [`Provider`] and the signer address.
    ///
    /// The `address` is used to set the `from` field in the transaction requests.
    ///
    /// The remote signer's address _must_ be the same as the signer address provided here.
    pub fn new(provider: P, address: Address) -> Self {
        Self { provider, address, _pd: std::marker::PhantomData }
    }

    /// Returns the underlying [`Provider`] used by the [`Web3Signer`].
    pub fn provider(&self) -> P {
        self.provider.clone()
    }
    /// Signs a transaction request and return the raw signed transaction in the form of [`Bytes`].
    ///
    /// The returned [`Bytes`] can be used to broadcast the transaction to the network using
    /// [`Provider::send_raw_transaction`].
    ///
    /// Sets the `from` field to the provided `address`.
    ///
    /// If you'd like to receive a [`Network::TxEnvelope`] instead, use
    /// [`Web3Signer::sign_and_decode`].
    pub async fn sign_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> alloy_signer::Result<Bytes> {
        // Always overrides the `from` field with the web3 signer's address.
        tx.set_from(self.address);
        self.provider.sign_transaction(tx).await.map_err(alloy_signer::Error::other)
    }

    /// Signs a transaction request using [`Web3Signer::sign_transaction`] and decodes the raw bytes
    /// returning a [`Network::TxEnvelope`].
    pub async fn sign_and_decode(
        &self,
        tx: N::TransactionRequest,
    ) -> alloy_signer::Result<N::TxEnvelope> {
        let raw = self.sign_transaction(tx).await?;
        N::TxEnvelope::decode_2718(&mut raw.as_ref()).map_err(alloy_signer::Error::other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ext::test::async_ci_only, Provider, ProviderBuilder};
    use alloy_consensus::{transaction::SignerRecoverable, TxEnvelope};
    use alloy_node_bindings::{utils::run_with_tempdir, Reth};
    use alloy_primitives::{Address, U256};

    #[tokio::test]
    #[cfg(not(windows))]
    async fn eth_sign_transaction() {
        async_ci_only(|| async {
            run_with_tempdir("reth-sign-tx", |dir| async {
                let reth = Reth::new().dev().disable_discovery().data_dir(dir).spawn();
                let provider = ProviderBuilder::new().connect_http(reth.endpoint_url());

                let accounts = provider.get_accounts().await.unwrap();
                let from = accounts[0];
                let signer = Web3Signer::new(provider, from);

                let tx = signer
                    .provider()
                    .transaction_request()
                    .from(from)
                    .to(Address::ZERO)
                    .value(U256::from(100))
                    .gas_limit(21000);

                let signed_tx = signer.sign_transaction(tx).await.unwrap();

                let tx = TxEnvelope::decode_2718(&mut signed_tx.as_ref()).unwrap();

                let signer = tx.recover_signer().unwrap();

                assert_eq!(signer, from);
            })
            .await
        })
        .await;
    }
}
