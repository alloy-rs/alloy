use crate::Signature;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::error::Error;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

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

    /// Encodes and signs the typed data according to [EIP-712].
    ///
    /// [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
    #[cfg(feature = "eip712")]
    async fn sign_typed_data<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature, Self::Error>
    where
        Self: Sized;

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
struct _ObjectSafe(dyn Signer<Error = ()>);
