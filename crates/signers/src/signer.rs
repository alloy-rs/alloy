use crate::Signature;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::error::Error;

/// Trait for signing transactions and messages.
///
/// Implement this trait to support different signing modes, e.g. Ledger, hosted etc.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: std::fmt::Debug + Send + Sync {
    /// The error type returned by the signer.
    type Error: Error + Send + Sync;

    /// Signs the hash of the provided message after prefixing it.
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, Self::Error>;

    /// Signs the transaction.
    #[cfg(TODO)]
    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, Self::Error>;

    /// Encodes and signs the typed data according [EIP-712].
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(TODO)]
    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, Self::Error>;

    /// Returns the signer's Ethereum Address.
    fn address(&self) -> Address;

    /// Returns the signer's chain ID.
    fn chain_id(&self) -> u64;

    /// Sets the signer's chain ID.
    #[must_use]
    fn with_chain_id<T: Into<u64>>(self, chain_id: T) -> Self
    where
        Self: Sized;
}

#[cfg(test)]
struct _ObjectSafe(dyn Signer<Error = ()>);
