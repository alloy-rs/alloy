use alloy_primitives::B256;

/// Metadata locating an included transaction within a block.
///
/// [`crate::TransactionResponse::inclusion_info`] constructs this only when the block hash, block
/// number, and transaction index are all present. The derived `Default` is an all-zero location;
/// it is not a pending sentinel.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct InclusionInfo {
    /// The hash of the block.
    pub block_hash: B256,
    /// The block number.
    pub block_number: u64,
    /// The index of the transaction in the block.
    pub transaction_index: u64,
}
