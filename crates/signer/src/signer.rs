use crate::Result;
use alloy_primitives::{eip191_hash_message, Address, ChainId, Signature, B256};
use async_trait::async_trait;
use auto_impl::auto_impl;

#[cfg(feature = "eip712")]
use alloy_dyn_abi::eip712::TypedData;
#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

pub use alloy_network::Transaction;

/// A signable transaction.
pub type SignableTx = dyn Transaction<Signature = Signature>;

/// Extension trait for utilities for signable transactions.
///
/// This trait is implemented for all types that implement [`Transaction`] with [`Signature`] as the
/// signature associated type.
pub trait TransactionExt: Transaction<Signature = Signature> {
    /// Set `chain_id` if it is not already set. Checks that the provided `chain_id` matches the
    /// existing `chain_id` if it is already set.
    fn set_chain_id_checked(&mut self, chain_id: ChainId) -> Result<()> {
        match self.chain_id() {
            Some(tx_chain_id) => {
                if tx_chain_id != chain_id {
                    return Err(crate::Error::TransactionChainIdMismatch {
                        signer: chain_id,
                        tx: tx_chain_id,
                    });
                }
            }
            None => {
                self.set_chain_id(chain_id);
            }
        }
        Ok(())
    }
}

impl<T: ?Sized + Transaction<Signature = Signature>> TransactionExt for T {}

/// Asynchronous Ethereum signer.
///
/// All provided implementations rely on [`sign_hash`](Signer::sign_hash). A signer may not always
/// be able to implement this method, in which case it should return
/// [`UnsupportedOperation`](crate::Error::UnsupportedOperation), and implement all the signing
/// methods directly.
///
/// A signer should hold an optional [`ChainId`] value, which is used for [EIP-155] replay
/// protection.
///
/// If `chain_id` is Some, [EIP-155] should be applied to the input transaction in
/// [`sign_transaction`](Self::sign_transaction), and to the resulting signature in all the methods.
/// If `chain_id` is None, [EIP-155] should not be applied.
///
/// Synchronous signers should implement both this trait and [`SignerSync`].
///
/// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[auto_impl(&mut, Box)]
pub trait Signer: Send + Sync {
    /// Signs the given hash.
    async fn sign_hash(&self, hash: B256) -> Result<Signature>;

    /// Signs the hash of the provided message after prefixing it, as specified in [EIP-191].
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        self.sign_hash(eip191_hash_message(message)).await
    }

    /// Signs the transaction.
    #[inline]
    async fn sign_transaction(&self, tx: &mut SignableTx) -> Result<Signature> {
        let chain_id = self.chain_id();
        if let Some(chain_id) = chain_id {
            tx.set_chain_id_checked(chain_id)?;
        }
        let mut sig = self.sign_hash(tx.signature_hash()).await?;
        if let Some(chain_id) = chain_id.or_else(|| tx.chain_id()) {
            sig = sig.with_chain_id(chain_id);
        }
        Ok(sig)
    }

    /// Encodes and signs the typed data according to [EIP-712].
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_typed_data<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature>
    where
        Self: Sized,
    {
        self.sign_hash(payload.eip712_signing_hash(domain)).await
    }

    /// Encodes and signs the typed data according to [EIP-712] for Signers that are not dynamically
    /// sized.
    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_dynamic_typed_data(&self, payload: &TypedData) -> Result<Signature> {
        let hash = payload.eip712_signing_hash()?;
        self.sign_hash(hash).await
    }

    /// Returns the signer's Ethereum Address.
    fn address(&self) -> Address;

    /// Returns the signer's chain ID.
    fn chain_id(&self) -> Option<ChainId>;

    /// Sets the signer's chain ID.
    fn set_chain_id(&mut self, chain_id: Option<ChainId>);

    /// Sets the signer's chain ID and returns `self`.
    #[inline]
    #[must_use]
    #[auto_impl(keep_default_for(&mut, Box))]
    fn with_chain_id(mut self, chain_id: Option<ChainId>) -> Self
    where
        Self: Sized,
    {
        self.set_chain_id(chain_id);
        self
    }
}

/// Synchronous Ethereum signer.
///
/// All provided implementations rely on [`sign_hash_sync`](SignerSync::sign_hash_sync). A signer
/// may not always be able to implement this method, in which case it should return
/// [`UnsupportedOperation`](crate::Error::UnsupportedOperation), and implement all the signing
/// methods directly.
///
/// A signer should hold an optional [`ChainId`] value, which is used for [EIP-155] replay
/// protection.
///
/// If `chain_id` is Some, [EIP-155] should be applied to the input transaction in
/// [`sign_transaction_sync`](Self::sign_transaction_sync), and to the resulting signature in all
/// the methods. If `chain_id` is None, [EIP-155] should not be applied.
///
/// Synchronous signers should also implement [`Signer`], as they are always able to by delegating
/// the asynchronous methods to the synchronous ones.
///
/// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait SignerSync {
    /// Signs the given hash.
    fn sign_hash_sync(&self, hash: B256) -> Result<Signature>;

    /// Signs the hash of the provided message after prefixing it, as specified in [EIP-191].
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    fn sign_message_sync(&self, message: &[u8]) -> Result<Signature> {
        self.sign_hash_sync(eip191_hash_message(message))
    }

    /// Signs the transaction.
    #[inline]
    fn sign_transaction_sync(&self, tx: &mut SignableTx) -> Result<Signature> {
        let chain_id = self.chain_id_sync();
        if let Some(chain_id) = chain_id {
            tx.set_chain_id_checked(chain_id)?;
        }
        let mut sig = self.sign_hash_sync(tx.signature_hash())?;
        if let Some(chain_id) = chain_id.or_else(|| tx.chain_id()) {
            sig = sig.with_chain_id(chain_id);
        }
        Ok(sig)
    }

    /// Encodes and signs the typed data according to [EIP-712].
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(feature = "eip712")]
    #[inline]
    fn sign_typed_data_sync<T: SolStruct>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature>
    where
        Self: Sized,
    {
        self.sign_hash_sync(payload.eip712_signing_hash(domain))
    }

    /// Encodes and signs the typed data according to [EIP-712] for Signers that are not dynamically
    /// sized.
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(feature = "eip712")]
    #[inline]
    fn sign_dynamic_typed_data_sync(&self, payload: &TypedData) -> Result<Signature> {
        let hash = payload.eip712_signing_hash()?;
        self.sign_hash_sync(hash)
    }

    /// Returns the signer's chain ID.
    fn chain_id_sync(&self) -> Option<ChainId>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, UnsupportedSignerOperation};
    use assert_matches::assert_matches;

    struct _ObjectSafe(Box<dyn Signer>, Box<dyn SignerSync>);

    #[tokio::test]
    async fn unimplemented() {
        #[cfg(feature = "eip712")]
        alloy_sol_types::sol! {
            #[derive(Default)]
            struct Eip712Data {
                uint64 a;
            }
        }

        async fn test_unimplemented_signer<S: Signer + SignerSync>(s: &S) {
            test_unsized_unimplemented_signer(s).await;
            test_unsized_unimplemented_signer_sync(s);

            #[cfg(feature = "eip712")]
            assert!(s
                .sign_typed_data_sync(&Eip712Data::default(), &Eip712Domain::default())
                .is_err());
            #[cfg(feature = "eip712")]
            assert!(s
                .sign_typed_data(&Eip712Data::default(), &Eip712Domain::default())
                .await
                .is_err());
        }

        async fn test_unsized_unimplemented_signer<S: Signer + ?Sized>(s: &S) {
            assert_matches!(
                s.sign_hash(B256::ZERO).await,
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert_matches!(
                s.sign_message(&[]).await,
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert!(s.sign_transaction(&mut alloy_consensus::TxLegacy::default()).await.is_err());
        }

        fn test_unsized_unimplemented_signer_sync<S: SignerSync + ?Sized>(s: &S) {
            assert_matches!(
                s.sign_hash_sync(B256::ZERO),
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert_matches!(
                s.sign_message_sync(&[]),
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert!(s.sign_transaction_sync(&mut alloy_consensus::TxLegacy::default()).is_err());
        }

        struct UnimplementedSigner;

        #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
        #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
        impl Signer for UnimplementedSigner {
            async fn sign_hash(&self, _hash: B256) -> Result<Signature> {
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            }

            fn address(&self) -> Address {
                Address::ZERO
            }

            fn chain_id(&self) -> Option<ChainId> {
                None
            }

            fn set_chain_id(&mut self, _chain_id: Option<ChainId>) {}
        }

        impl SignerSync for UnimplementedSigner {
            fn sign_hash_sync(&self, _hash: B256) -> Result<Signature> {
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            }

            fn chain_id_sync(&self) -> Option<ChainId> {
                None
            }
        }

        test_unimplemented_signer(&UnimplementedSigner).await;
        test_unsized_unimplemented_signer(&UnimplementedSigner as &dyn Signer).await;
        test_unsized_unimplemented_signer_sync(&UnimplementedSigner as &dyn SignerSync);
    }
}
