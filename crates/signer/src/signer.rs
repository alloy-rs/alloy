use crate::{Error, Result, Signature, UnsupportedSignerOperation};
use alloy_primitives::{eip191_hash_message, Address, B256};
use async_trait::async_trait;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

/// Ethereum signer.
///
/// All provided implementations rely on [`sign_hash`](Signer::sign_hash). If the signer is not able
/// to implement this method, then all other methods must be implemented directly, or they will
/// return [`UnsupportedOperation`](Error::UnsupportedOperation).
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: Send + Sync {
    /// Signs the hash.
    #[inline]
    fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        let _ = hash;
        Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
    }

    /// Signs the hash.
    ///
    /// Asynchronous version of [`sign_hash`](Signer::sign_hash). The default implementation
    /// delegates to the synchronous version.
    #[inline]
    async fn sign_hash_async(&self, hash: &B256) -> Result<Signature> {
        self.sign_hash(hash)
    }

    /// Signs the hash of the provided message after prefixing it, as specified in [EIP-191].
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        self.sign_hash(&eip191_hash_message(message))
    }

    /// Signs the hash of the provided message after prefixing it, as specified in [EIP-191].
    ///
    /// Asynchronous version of [`sign_message`](Signer::sign_message). The default implementation
    /// delegates to the synchronous version.
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    async fn sign_message_async(&self, message: &[u8]) -> Result<Signature> {
        self.sign_message(message)
    }

    /// Signs the transaction.
    #[cfg(TODO)]
    #[inline]
    fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature> {
        self.sign_hash(&message.sighash())
    }

    /// Signs the transaction.
    ///
    /// Asynchronous version of [`sign_transaction`](Signer::sign_transaction). The default
    /// implementation delegates to the synchronous version.
    #[cfg(TODO)]
    #[inline]
    async fn sign_transaction_async(&self, message: &TypedTransaction) -> Result<Signature> {
        self.sign_transaction(message)
    }

    /// Encodes and signs the typed data according to [EIP-712].
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(feature = "eip712")]
    #[inline]
    fn sign_typed_data<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature>
    where
        Self: Sized,
    {
        self.sign_hash(&payload.eip712_signing_hash(domain))
    }

    /// Encodes and signs the typed data according to [EIP-712].
    ///
    /// Asynchronous version of [`sign_typed_data`](Signer::sign_typed_data). The default
    /// implementation delegates to the synchronous version.
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(feature = "eip712")]
    #[inline]
    async fn sign_typed_data_async<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature>
    where
        Self: Sized,
    {
        self.sign_typed_data(payload, domain)
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    struct _ObjectSafe(Box<dyn Signer>);

    #[tokio::test]
    async fn unimplemented() {
        #[cfg(feature = "eip712")]
        alloy_sol_types::sol! {
            #[derive(Default)]
            struct Eip712Data {
                uint64 a;
            }
        }

        async fn test_unimplemented_signer<S: Signer>(s: &S) {
            test_unsized_unimplemented_signer(s).await;

            #[cfg(feature = "eip712")]
            {
                assert!(s
                    .sign_typed_data(&Eip712Data::default(), &Eip712Domain::default())
                    .is_err());
                assert!(s
                    .sign_typed_data_async(&Eip712Data::default(), &Eip712Domain::default())
                    .await
                    .is_err());
            }
        }

        async fn test_unsized_unimplemented_signer<S: Signer + ?Sized>(s: &S) {
            assert_matches!(
                s.sign_hash(&B256::ZERO),
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );
            assert_matches!(
                s.sign_hash_async(&B256::ZERO).await,
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            assert_matches!(
                s.sign_message(&[]),
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );
            assert_matches!(
                s.sign_message_async(&[]).await,
                Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
            );

            #[cfg(TODO)]
            assert!(s.sign_transaction(&TypedTransaction::default()).is_err());
            #[cfg(TODO)]
            assert!(s.sign_transaction_async(&TypedTransaction::default()).await.is_err());
        }

        struct UnimplementedSigner;

        impl Signer for UnimplementedSigner {
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

        test_unimplemented_signer(&UnimplementedSigner).await;
        test_unsized_unimplemented_signer(&UnimplementedSigner as &dyn Signer).await;
    }
}
