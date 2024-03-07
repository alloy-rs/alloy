use crate::Network;
use alloy_consensus::SignableTransaction;
use alloy_signer::{
    k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId},
    Signature, Signer, SignerSync, Wallet,
};
use async_trait::async_trait;

// todo: move
/// A signer capable of signing any transaction for the given network.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkSigner<N: Network>: Sync {
    /// Asynchronously sign an unsigned transaction.
    async fn sign(&self, tx: N::UnsignedTx) -> alloy_signer::Result<N::TxEnvelope>;
}

// todo: move
/// An async signer capable of signing any [SignableTransaction] for the given [Signature] type.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

// todo: these are implemented here because of a potential circular dep
// we should move wallet/yubi etc. into its own crate
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<D> TxSigner<Signature> for Wallet<D>
where
    D: PrehashSigner<(ecdsa::Signature, RecoveryId)> + Send + Sync,
{
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature> {
        self.sign_hash(&tx.signature_hash()).await
    }
}

impl<D> TxSignerSync<Signature> for Wallet<D>
where
    D: PrehashSigner<(ecdsa::Signature, RecoveryId)>,
{
    fn sign_transaction_sync(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature> {
        self.sign_hash_sync(&tx.signature_hash())
    }
}
