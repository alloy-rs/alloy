use alloy_primitives::{Bloom, Log};

/// Receipt is the result of a transaction execution.
pub trait Receipt {
    /// Returns true if the transaction was successful.
    fn success(&self) -> bool;

    /// Returns the bloom filter for the logs in the receipt. This operation
    /// may be expensive.
    fn bloom(&self) -> Bloom;

    /// Returns the bloom filter for the logs in the receipt, if it is cheap to
    /// compute.
    fn bloom_cheap(&self) -> Option<Bloom> {
        None
    }

    /// Returns the cumulative gas used in the block after this transaction was executed.
    fn cumulative_gas_used(&self) -> u64;

    /// Returns the logs emitted by this transaction.
    fn logs(&self) -> &[Log];
}
