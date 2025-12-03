use alloy_primitives::B256;

/// Additional fields in the context of a block that contains an included transaction.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct InclusionInfo {
    /// The hash of the block.
    pub block_hash: B256,
    /// The block number.
    pub block_number: u64,
    /// The index of the transaction in the block.
    pub transaction_index: u64,
}
