use crate::{Network, TransactionBuilder};
use alloy_consensus::SignableTransaction;
use alloy_primitives::Address;
use async_trait::async_trait;
use futures_utils_wasm::impl_future;

/// A signer capable of signing any transaction for the given network.
///
/// Network crate authors should implement this trait on a type capable of
/// signing any transaction (regardless of signature type) on a given network.
/// Signer crate authors should instead implement [`TxSigner`] to signify
/// signing capability for specific signature types.
///
/// Network signers are expected to contain one or more signing credentials,
/// keyed by signing address. The default signer address should be used when
/// no specific signer address is specified.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkSigner<N: Network>: std::fmt::Debug + Send + Sync {
    /// Get the default signer address. This address should be used
    /// in [`NetworkSigner::sign_transaction_from`] when no specific signer is
    /// specified.
    fn default_signer_address(&self) -> Address;

    /// Return true if the signer contains a credential for the given address.
    fn has_signer_for(&self, address: &Address) -> bool;

    /// Return an iterator of all signer addresses.
    fn signer_addresses(&self) -> impl Iterator<Item = Address>;

    /// Asynchronously sign an unsigned transaction, with a specified
    /// credential.
    async fn sign_transaction_from(
        &self,
        sender: Address,
        tx: N::UnsignedTx,
    ) -> alloy_signer::Result<N::TxEnvelope>;

    /// Asynchronously sign an unsigned transaction.
    fn sign_transaction(
        &self,
        tx: N::UnsignedTx,
    ) -> impl_future!(<Output = alloy_signer::Result<N::TxEnvelope>>) {
        self.sign_transaction_from(self.default_signer_address(), tx)
    }

    /// Asynchronously sign a transaction request, using the sender specified
    /// in the `from` field.
    async fn sign_request(
        &self,
        request: N::TransactionRequest,
    ) -> alloy_signer::Result<N::TxEnvelope> {
        let sender = request.from().unwrap_or_else(|| self.default_signer_address());
        let tx = request.build_unsigned().map_err(|(_, e)| alloy_signer::Error::other(e))?;
        self.sign_transaction_from(sender, tx).await
    }
}

/// Asynchronous transaction signer, capable of signing any [`SignableTransaction`] for the given
/// `Signature` type.
///
/// A signer should hold an optional [`ChainId`] value, which is used for [EIP-155] replay
/// protection.
///
/// If `chain_id` is Some, [EIP-155] should be applied to the input transaction in
/// [`sign_transaction`](Self::sign_transaction), and to the resulting signature in all the methods.
/// If `chain_id` is None, [EIP-155] should not be applied.
///
/// Synchronous signers should implement both this trait and [`TxSignerSync`].
///
/// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
/// [`ChainId`]: alloy_primitives::ChainId
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TxSigner<Signature> {
    /// Get the address of the signer.
    fn address(&self) -> Address;

    /// Asynchronously sign an unsigned transaction.
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}

/// Synchronous transaction signer,  capable of signing any [`SignableTransaction`] for the given
/// `Signature` type.
///
/// A signer should hold an optional [`ChainId`] value, which is used for [EIP-155] replay
/// protection.
///
/// If `chain_id` is Some, [EIP-155] should be applied to the input transaction in
/// [`sign_transaction_sync`](Self::sign_transaction_sync), and to the resulting signature in all
/// the methods. If `chain_id` is None, [EIP-155] should not be applied.
///
/// Synchronous signers should also implement [`TxSigner`], as they are always able to by delegating
/// the asynchronous methods to the synchronous ones.
///
/// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
/// [`ChainId`]: alloy_primitives::ChainId
pub trait TxSignerSync<Signature> {
    /// Get the address of the signer.
    fn address(&self) -> Address;

    /// Synchronously sign an unsigned transaction.
    fn sign_transaction_sync(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}
