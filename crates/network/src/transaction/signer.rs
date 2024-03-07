use crate::Network;
use alloy_consensus::SignableTransaction;
use async_trait::async_trait;

// todo: move
/// A signer capable of signing any transaction for the given network.
#[async_trait]
pub trait NetworkSigner<N: Network>: Sync {
    /// Asynchronously sign an unsigned transaction.
    async fn sign(&self, tx: N::UnsignedTx) -> alloy_signer::Result<N::TxEnvelope>;
}

// todo: move
/// An async signer capable of signing any [SignableTransaction] for the given [Signature] type.
#[async_trait]
pub trait TxSigner<Signature> {
    /// Asynchronously sign an unsigned transaction.
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}

// todo: move
/// A sync signer capable of signing any [SignableTransaction] for the given [Signature] type.
pub trait TxSignerSync<Signature> {
    /// Synchronously sign an unsigned transaction.
    fn sign_transaction_sync(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}
