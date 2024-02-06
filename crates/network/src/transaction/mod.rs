mod builder;
use alloy_primitives::{ChainId, U256};
pub use builder::{Builder, BuilderError};

mod common;
pub use common::TxKind;

mod signed;
pub use signed::Signed;

mod signer;
pub use signer::{NetworkSigner, Signable, TxSigner, TxSignerSync};

/// Represents a minimal EVM transaction.
pub trait Transaction: std::any::Any + Send + Sync + 'static {
    /// Get `data`.
    fn input(&self) -> &[u8];

    /// Get `to`.
    fn to(&self) -> TxKind;

    /// Get `value`.
    fn value(&self) -> U256;

    /// Get `chain_id`.
    fn chain_id(&self) -> Option<ChainId>;

    /// Get `nonce`.
    fn nonce(&self) -> u64;

    /// Get `gas_limit`.
    fn gas_limit(&self) -> u64;

    /// Get `gas_price`.
    fn gas_price(&self) -> Option<U256>;
}
