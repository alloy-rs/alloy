use crate::fillers::{FillerControlFlow, TxFiller};
use alloy_eips::Decodable2718;
use alloy_network::{Ethereum, Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes};
use alloy_rpc_client::WeakClient;
use alloy_transport::{TransportErrorKind, TransportResult};

use super::{Provider, SendableTx};

/// A remote signer that leverages the underlying provider to sign transactions using
/// `"eth_signTransaction"` requests.
///
/// For more information, please see [Web3Signer](https://docs.web3signer.consensys.io/).
///
/// [`Web3Signer`] also implements [`TxFiller`] to allow it to be used as a filler in the
/// [`ProviderBuilder`].
///
/// Note:
///
/// `"eth_signTransaction"` is not supported by regular nodes.
///
/// [`ProviderBuilder`]: crate::ProviderBuilder
#[derive(Debug, Clone)]
pub struct Web3Signer<N: Network = Ethereum> {
    /// The [`WeakClient`] used to make `"eth_signTransaction"` requests.
    client: WeakClient,
    /// The address of the remote signer that will sign the transactions.
    ///
    /// This is set as the `from` field in the [`Network::TransactionRequest`]'s for the
    /// `"eth_signTransaction"` requests.
    address: Address,
    _pd: std::marker::PhantomData<N>,
}

impl<N: Network> Web3Signer<N> {
    /// Instantiates a new [`Web3Signer`] with the given [`WeakClient`] and the signer address.
    ///
    /// A weak client can be obtained via the [`Provider::weak_client`] method.
    ///
    /// The `address` is used to set the `from` field in the transaction requests.
    ///
    /// The remote signer's address _must_ be the same as the signer address provided here.
    ///
    /// [`Provider::weak_client`]: crate::Provider::weak_client
    pub fn new<P: Provider<N>>(provider: &P, address: Address) -> Self {
        Self { client: provider.weak_client(), address, _pd: std::marker::PhantomData }
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
        // Always overrides the `from` field with the web3 signer's address.
        tx.set_from(self.address);

        let client = self
            .client
            .upgrade()
            .ok_or_else(|| alloy_signer::Error::other("client dropped in web3signer"))?;

        client.request("eth_signTransaction", (tx,)).await.map_err(alloy_signer::Error::other)
    }
}

impl<N: Network> TxFiller<N> for Web3Signer<N> {
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
            // Always overrides the `from` field with the web3 signer's address.
            builder.set_from(self.address);
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

        let raw = self.sign_transaction(builder).await.map_err(TransportErrorKind::custom)?;

        let envelope =
            N::TxEnvelope::decode_2718(&mut raw.as_ref()).map_err(TransportErrorKind::custom)?;

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
        // Always overrides the `from` field with the web3 signer's address.
        tx.set_from(self.address);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ext::test::async_ci_only, Provider, ProviderBuilder};
    use alloy_consensus::TxEnvelope;
    use alloy_node_bindings::{utils::run_with_tempdir, Reth};
    use alloy_primitives::{Address, U256};

    #[tokio::test]
    #[cfg(not(windows))]
    async fn eth_sign_transaction() {
        // async_ci_only(|| async {
        run_with_tempdir("reth-sign-tx", |dir| async {
            let reth = Reth::new().dev().disable_discovery().data_dir(dir).spawn();
            let provider = ProviderBuilder::new().on_http(reth.endpoint_url());

            let accounts = provider.get_accounts().await.unwrap();
            let from = accounts[0];
            let signer = Web3Signer::new(&provider, from);

            let fees = provider.estimate_eip1559_fees().await.unwrap();
            let tx = provider
                .transaction_request()
                .from(from)
                .to(Address::ZERO)
                .value(U256::from(100))
                .gas_limit(21000)
                .max_fee_per_gas(fees.max_fee_per_gas)
                .max_priority_fee_per_gas(fees.max_priority_fee_per_gas)
                .nonce(0);

            let signed_tx = signer.sign_transaction(tx).await.unwrap();

            let tx = TxEnvelope::decode_2718(&mut signed_tx.as_ref()).unwrap();

            let signer = tx.recover_signer().unwrap();

            assert_eq!(signer, from);
        })
        .await
        // })
        // .await;
    }
}
