//! Ethereum Web3 Signer.

use alloy_consensus::{SignableTransaction, TxEnvelope};
use alloy_network::{eip2718::Decodable2718, Ethereum, Network, TransactionBuilder, TxSigner};
use alloy_primitives::{Address, Bytes, PrimitiveSignature as Signature};
use alloy_provider::Provider;

/// Web3 Signer.
#[derive(Debug)]
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
        let max_fee_or_gas_price = tx.max_fee_per_gas();
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

        if let Some(hashes) = tx.blob_versioned_hashes() {
            // Cannot set versioned hashes via TransactionBuilder.
            todo!("Set versioned hashes")
        }

        // TODO: Read and set sidecar??
        // Cannot access sidecar via Transaction trait

        if let Some(auth) = tx.authorization_list() {
            // Cannot set authorization list via TransactionBuilder.
            todo!("Set authorization list")
        }

        let raw = self.sign_transaction(request).await?;

        let envelope =
            TxEnvelope::decode_2718(&mut raw.as_ref()).map_err(alloy_signer::Error::other)?;

        Ok(*envelope.signature())
    }
}

alloy_network::impl_into_wallet!(@[P: Provider<N> + core::fmt::Debug + 'static, N: Network] Web3Signer<P, N>);
