//! Commonly used types that contain metadata of a block.

use alloy_primitives::{Address, B256, U256};

/// Essential info extracted from a header.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct HeaderInfo {
    /// The number of ancestor blocks of this block (block height).
    pub number: u64,
    /// Beneficiary (Coinbase or miner) is a address that have signed the block.
    ///
    /// This is the receiver address of all the gas spent in the block.
    pub beneficiary: Address,
    /// The timestamp of the block in seconds since the UNIX epoch
    pub timestamp: u64,
    /// The gas limit of the block
    pub gas_limit: u64,
    /// The base fee per gas, added in the London upgrade with [EIP-1559]
    ///
    /// [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559
    pub base_fee_per_gas: Option<u64>,
    /// A running total of blob gas consumed in excess of the target, prior to the block. Blocks
    /// with above-target blob gas consumption increase this value, blocks with below-target blob
    /// gas consumption decrease it (bounded at 0). This was added in EIP-4844.
    pub excess_blob_gas: Option<u64>,
    /// The total amount of blob gas consumed by the transactions within the block, added in
    /// EIP-4844.
    pub blob_gas_used: Option<u64>,
    /// The difficulty of the block
    ///
    /// Unused after the Paris (AKA the merge) upgrade and replaced by `prevrandao` and expected to
    /// be 0.
    pub difficulty: U256,
    /// The output of the randomness beacon provided by the beacon chain
    ///
    /// Replaces `difficulty` after the Paris (AKA the merge) upgrade with [EIP-4399].
    ///
    /// Note: `prevrandao` can be found in a block in place of `mix_hash`.
    ///
    /// [EIP-4399]: https://eips.ethereum.org/EIPS/eip-4399
    pub mix_hash: Option<B256>,
}
