//! Contains constants and utility functions for [EIP-7742](https://eips.ethereum.org/EIPS/eip-7742)

use crate::eip4844;

/// Calculates the `excess_blob_gas` from the parent header's `blob_gas_used`, `excess_blob_gas` and
/// `target_blobs_per_block`.
///
/// Similar to [`eip4844::calc_excess_blob_gas`], but uses the `target_blobs_per_block` to calculate
/// the target gas usage.
#[inline]
pub const fn calc_excess_blob_gas(
    parent_excess_blob_gas: u64,
    parent_blob_gas_used: u64,
    parent_target_blobs_per_block: u64,
) -> u64 {
    (parent_excess_blob_gas + parent_blob_gas_used)
        .saturating_sub(parent_target_blobs_per_block * eip4844::DATA_GAS_PER_BLOB)
}
