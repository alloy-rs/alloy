use alloy_eips::Typed2718;
use alloy_primitives::TxHash;

/// Generic trait to get a transaction hash from any signature type
pub trait TxHashable<S>: Typed2718 {
    /// Calculate the transaction hash for the given signature and type.
    fn tx_hash_with_type(&self, signature: &S, ty: u8) -> TxHash;

    /// Calculate the transaction hash for the given signature.
    fn tx_hash(&self, signature: &S) -> TxHash {
        self.tx_hash_with_type(signature, self.ty())
    }
}
