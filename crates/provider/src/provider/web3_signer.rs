use crate::Provider;
use alloy_consensus::{SignableTransaction, TxEnvelope};
use alloy_network::{
    eip2718::Decodable2718, AnyNetwork, Ethereum, EthereumWallet, IntoWallet, Network,
    TransactionBuilder, TransactionBuilder4844, TransactionBuilder7702, TxSigner,
};
use alloy_primitives::{Address, Bytes, PrimitiveSignature as Signature};

/// A remote signer that leverages the underlying provider to sign transactions using
/// `"eth_signTransaction"` requests.
///
/// For more information, please see [Web3Signer](https://docs.web3signer.consensys.io/).
///
/// Note:
///
/// `"eth_signTransaction"` is not supported by regular nodes.
#[derive(Debug, Clone)]
pub struct Web3Signer<P: Provider<N>, N: Network = Ethereum> {
    /// The provider used to make `"eth_signTransaction"` requests.
    provider: P,
    /// The address of the remote signer that will sign the transactions.
    ///
    /// This is set as the `from` field in the [`Network::TransactionRequest`]'s for the
    /// `"eth_signTransaction"` requests.
    address: Address,
    _pd: std::marker::PhantomData<N>,
}

impl<P: Provider<N>, N: Network> Web3Signer<P, N> {
    /// Instantiates a new [`Web3Signer`] with the given provider and the signer address.
    ///
    /// The `address` is used to set the `from` field in the transaction requests.
    ///
    /// The remote signer's address _must_ be the same as the signer address provided here.
    pub fn new(provider: P, address: Address) -> Self {
        Self { provider, address, _pd: std::marker::PhantomData }
    }

    /// Signs a transaction request and return the raw signed transaction in the form of [`Bytes`].
    ///
    /// The returned [`Bytes`] can be used to broadcast the transaction to the network using
    /// [`Provider::send_raw_transaction`].
    ///
    /// Sets the `from` field to the provided `address`.
    pub async fn sign_transaction(
        &self,
        mut tx: N::TransactionRequest,
    ) -> alloy_signer::Result<Bytes> {
        tx.set_from(self.address);
        self.provider.sign_transaction(tx).await.map_err(alloy_signer::Error::other)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<P, N> TxSigner<Signature> for Web3Signer<P, N>
where
    P: Provider<N>,
    N: Network,
    N::TransactionRequest: TransactionBuilder7702 + TransactionBuilder4844,
{
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature> {
        let mut request = N::TransactionRequest::default();

        // Basics
        request.set_kind(tx.kind());
        request.set_nonce(tx.nonce());
        request.set_input(tx.input().clone());
        request.set_value(tx.value());

        if let Some(chain_id) = tx.chain_id() {
            request.set_chain_id(chain_id);
        }

        // Gas related fields
        request.set_gas_limit(tx.gas_limit());
        let max_fee_or_gas_price = tx.max_fee_per_gas(); // Returns `gasPrice` if not dynamic fee.
        if tx.is_dynamic_fee() {
            request.set_max_fee_per_gas(max_fee_or_gas_price);
            if let Some(max_priority_fee) = tx.max_priority_fee_per_gas() {
                request.set_max_priority_fee_per_gas(max_priority_fee);
            }
        } else {
            request.set_gas_price(max_fee_or_gas_price);
        }

        if let Some(access_list) = tx.access_list() {
            request.set_access_list(access_list.clone());
        }

        if let Some(sidecar) = tx.blob_sidecar() {
            request.set_blob_sidecar(sidecar.clone());
        }

        if let Some(auth) = tx.authorization_list() {
            request.set_authorization_list(auth.to_vec());
        }

        let raw = self.sign_transaction(request).await?;

        let envelope =
            TxEnvelope::decode_2718(&mut raw.as_ref()).map_err(alloy_signer::Error::other)?;

        Ok(*envelope.signature())
    }
}

impl<P: Provider + core::fmt::Debug + 'static> IntoWallet for Web3Signer<P> {
    type NetworkWallet = EthereumWallet;

    fn into_wallet(self) -> Self::NetworkWallet {
        EthereumWallet::from(self)
    }
}

impl<P: Provider<AnyNetwork> + core::fmt::Debug + 'static> IntoWallet<AnyNetwork>
    for Web3Signer<P, AnyNetwork>
{
    type NetworkWallet = EthereumWallet;

    fn into_wallet(self) -> Self::NetworkWallet {
        EthereumWallet::from(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ext::test::async_ci_only, Provider, ProviderBuilder};
    use alloy_consensus::TxEnvelope;
    use alloy_network::eip2718::Decodable2718;
    use alloy_node_bindings::{utils::run_with_tempdir, Reth};
    use alloy_primitives::{Address, U256};

    #[tokio::test]
    #[cfg(not(windows))]
    async fn eth_sign_transaction() {
        async_ci_only(|| async {
            run_with_tempdir("reth-sign-tx", |dir| async {
                let reth = Reth::new().dev().disable_discovery().data_dir(dir).spawn();
                let provider = ProviderBuilder::new().on_http(reth.endpoint_url());

                let accounts = provider.get_accounts().await.unwrap();
                let from = accounts[0];
                let signer = Web3Signer::new(provider.clone(), from);

                let tx = provider
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
