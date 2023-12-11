use crate::{Result, Signature};
use alloy_primitives::{eip191_hash_message, Address, B256};
use async_trait::async_trait;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

/// Asynchronous Ethereum signer.
///
/// All provided implementations rely on [`sign_hash`](Signer::sign_hash). A signer may not always
/// be able to implement this method, in which case it should return
/// [`UnsupportedOperation`](crate::Error::UnsupportedOperation), and implement all the signing
/// methods directly.
///
/// Synchronous signers should implement both this trait and [`SignerSync`].
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: Send + Sync {
    /// Signs the given hash.
    async fn sign_hash(&self, hash: &B256) -> Result<Signature>;

    /// Signs the hash of the provided message after prefixing it, as specified in [EIP-191].
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        self.sign_hash(&eip191_hash_message(message)).await
    }

    /// Signs the transaction.
    #[cfg(TODO)] // TODO: TypedTransaction
    #[inline]
    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature> {
        self.sign_hash(&message.sighash()).await
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
        self.sign_hash(&payload.eip712_signing_hash(domain)).await
    }

    /// Returns the signer's Ethereum Address.
    fn address(&self) -> Address;

    /// Returns the signer's chain ID.
    fn chain_id(&self) -> u64;

    /// Sets the signer's chain ID.
    fn set_chain_id(&mut self, chain_id: u64);

    /// Sets the signer's chain ID and returns `self`.
    #[inline]
    #[must_use]
    fn with_chain_id(mut self, chain_id: u64) -> Self
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
/// Synchronous signers should also implement [`Signer`], as they are always able to by delegating
/// the asynchronous methods to the synchronous ones.
pub trait SignerSync {
    /// Signs the given hash.
    fn sign_hash_sync(&self, hash: &B256) -> Result<Signature>;

    /// Signs the hash of the provided message after prefixing it, as specified in [EIP-191].
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    fn sign_message_sync(&self, message: &[u8]) -> Result<Signature> {
        self.sign_hash_sync(&eip191_hash_message(message))
    }

    /// Signs the transaction.
    #[cfg(TODO)] // TODO: TypedTransaction
    #[inline]
    fn sign_transaction_sync(&self, message: &TypedTransaction) -> Result<Signature> {
        self.sign_hash_sync(&message.sighash())
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
        self.sign_hash_sync(&payload.eip712_signing_hash(domain))
    }
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
                s.sign_hash(&B256::ZERO).await,
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert_matches!(
                s.sign_message(&[]).await,
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            #[cfg(TODO)] // TODO: TypedTransaction
            assert!(s.sign_transaction(&Default::default()).await.is_err());
        }

        fn test_unsized_unimplemented_signer_sync<S: SignerSync + ?Sized>(s: &S) {
            assert_matches!(
                s.sign_hash_sync(&B256::ZERO),
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert_matches!(
                s.sign_message_sync(&[]),
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            #[cfg(TODO)] // TODO: TypedTransaction
            assert!(s.sign_transaction_sync(&Default::default()).is_err());
        }

        struct UnimplementedSigner;

        #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
        #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
        impl Signer for UnimplementedSigner {
            async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            }

            fn address(&self) -> Address {
                unimplemented!()
            }

            fn chain_id(&self) -> u64 {
                unimplemented!()
            }

            fn set_chain_id(&mut self, _chain_id: u64) {
                unimplemented!()
            }
        }

        impl SignerSync for UnimplementedSigner {
            fn sign_hash_sync(&self, _hash: &B256) -> Result<Signature> {
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            }
        }

        test_unimplemented_signer(&UnimplementedSigner).await;
        test_unsized_unimplemented_signer(&UnimplementedSigner as &dyn Signer).await;
        test_unsized_unimplemented_signer_sync(&UnimplementedSigner as &dyn SignerSync);
    }
}
