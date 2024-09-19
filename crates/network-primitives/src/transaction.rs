//! Commonly used additional types that are not part of the JSON RPC spec but are often required
//! when working with RPC types, such as [Transaction]

use alloy_primitives::{BlockHash, TxHash};

/// Additional fields in the context of a block that contains this transaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[doc(alias = "TxInfo")]
pub struct TransactionInfo {
    /// Hash of the transaction.
    pub hash: Option<TxHash>,
    /// Index of the transaction in the block
    pub index: Option<u64>,
    /// Hash of the block.
    pub block_hash: Option<BlockHash>,
    /// Number of the block.
    pub block_number: Option<u64>,
    /// Base fee of the block.
    pub base_fee: Option<u128>,
}

impl TransactionInfo {
    /// Returns a new [`TransactionInfo`] with the provided base fee.
    pub const fn with_base_fee(mut self, base_fee: u128) -> Self {
        self.base_fee = Some(base_fee);
        self
    }
}
