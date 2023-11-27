use crate::{Error, Result, Signature, UnsupportedSignerOperation};
use alloy_primitives::{eip191_hash_message, Address, B256};
use async_trait::async_trait;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

/// Ethereum signer.
///
/// All provided implementations rely on [`sign_hash`] (or [`sign_hash_async`], which delegates to
/// [`sign_hash`]). If the signer is not able to implement this method, then all other methods will
/// have to be implemented directly, or they will return
/// [`UnsupportedOperation`](Error::UnsupportedOperation).
///
/// [`sign_hash`]: Signer::sign_hash
/// [`sign_hash_async`]: Signer::sign_hash_async
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: Send + Sync {
    /// Signs the hash.
    ///
    /// The default implementation returns [`UnsupportedOperation`](Error::UnsupportedOperation).
    #[inline]
    fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        let _ = hash;
        Err(Error::UnsupportedOperation(UnsupportedSignerOperation::SignHash))
    }

    /// Signs the hash.
    ///
    /// Asynchronous version of [`sign_hash`](Signer::sign_hash). The default implementation
    /// delegates to the synchronous version; see its documentation for more details.
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
    /// Asynchronous version of [`sign_message`](Signer::sign_message). The default
    /// implementation is the same as the synchronous version; see its documentation for more
    /// details.
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    #[inline]
    async fn sign_message_async(&self, message: &[u8]) -> Result<Signature> {
        self.sign_hash_async(&eip191_hash_message(message)).await
    }

    /// Signs the transaction.
    ///
    /// The default implementation signs the [transaction's signature hash][sighash], and optionally
    /// applies [EIP-155] to the signature if a chain ID is present.
    ///
    /// [sighash]: TypedTransaction::sighash
    /// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
    #[cfg(TODO)]
    #[inline]
    fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature> {
        self.sign_hash(&tx.sighash()).map(|mut sig| {
            if let Some(chain_id) = tx.chain_id() {
                sig.apply_eip155(chain_id);
            }
            sig
        })
    }

    /// Signs the transaction.
    ///
    /// Asynchronous version of [`sign_transaction`](Signer::sign_transaction). The default
    /// implementation is the same as the synchronous version; see its documentation for more
    /// details.
    #[cfg(TODO)]
    #[inline]
    async fn sign_transaction_async(&self, message: &TypedTransaction) -> Result<Signature> {
        self.sign_hash_async(&tx.sighash())
            .map(|mut sig| {
                if let Some(chain_id) = tx.chain_id() {
                    sig.apply_eip155(chain_id);
                }
                sig
            })
            .await
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
    /// implementation is the same as the synchronous version; see its documentation for more
    /// details.
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
        self.sign_hash_async(&payload.eip712_signing_hash(domain)).await
    }

    /// Returns the signer's Ethereum Address.
    fn address(&self) -> Address;
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
        }

        test_unimplemented_signer(&UnimplementedSigner).await;
        test_unsized_unimplemented_signer(&UnimplementedSigner as &dyn Signer).await;
    }
}
