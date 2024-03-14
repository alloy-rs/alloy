use crate::Network;
use alloy_consensus::SignableTransaction;
use async_trait::async_trait;

/// A signer capable of signing any transaction for the given network.
///
/// Network crate authors should implement this trait on a type capable of signing any transaction
/// (regardless of signature type) on a given network. Signer crate authors should instead implement
/// [`TxSigner`] to signify signing capability for specific signature types.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkSigner<N: Network>: Send + Sync {
    /// Asynchronously sign an unsigned transaction.
    async fn sign_transaction(&self, tx: N::UnsignedTx) -> alloy_signer::Result<N::TxEnvelope>;
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
    /// Synchronously sign an unsigned transaction.
    fn sign_transaction_sync(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}
