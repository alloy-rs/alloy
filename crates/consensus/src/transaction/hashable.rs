use alloy_primitives::TxHash;

/// Generic trait to get a transaction hash from any signature type
pub trait TxHashable<S> {
    /// Calculate the transaction hash for the given signature and type.
    fn tx_hash_with_type(&self, signature: &S, ty: u8) -> TxHash;
}
