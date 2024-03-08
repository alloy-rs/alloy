use crate::Network;
use alloy_consensus::SignableTransaction;
use alloy_signer::{
    k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId},
    Signature, Signer, SignerSync, Wallet,
};
use async_trait::async_trait;

/// A signer capable of signing any transaction for the given network.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkSigner<N: Network>: Send + Sync {
    /// Asynchronously sign an unsigned transaction.
    async fn sign(&self, tx: N::UnsignedTx) -> alloy_signer::Result<N::TxEnvelope>;
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
        let chain_id = self.chain_id_sync();
        if let Some(chain_id) = chain_id {
            match tx.chain_id() {
                Some(tx_chain_id) => {
                    if tx_chain_id != chain_id {
                        return Err(alloy_signer::Error::TransactionChainIdMismatch {
                            signer: chain_id,
                            tx: tx_chain_id,
                        });
                    }
                }
                None => {
                    tx.set_chain_id(chain_id);
                }
            }
        }

        let mut sig = self.sign_hash(&tx.signature_hash()).await?;
        if let Some(chain_id) = chain_id.or_else(|| tx.chain_id()) {
            sig = sig.with_chain_id(chain_id);
        }
        Ok(sig)
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
        let chain_id = self.chain_id_sync();
        if let Some(chain_id) = chain_id {
            match tx.chain_id() {
                Some(tx_chain_id) => {
                    if tx_chain_id != chain_id {
                        return Err(alloy_signer::Error::TransactionChainIdMismatch {
                            signer: chain_id,
                            tx: tx_chain_id,
                        });
                    }
                }
                None => {
                    tx.set_chain_id(chain_id);
                }
            }
        }

        let mut sig = self.sign_hash_sync(&tx.signature_hash())?;
        if let Some(chain_id) = chain_id.or_else(|| tx.chain_id()) {
            sig = sig.with_chain_id(chain_id);
        }
        Ok(sig)
    }
}
