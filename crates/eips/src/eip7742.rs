//! Contains constants and utility functions for [EIP-7742](https://eips.ethereum.org/EIPS/eip-7742)

use crate::eip4844::{fake_exponential, BLOB_TX_MIN_BLOB_GASPRICE, DATA_GAS_PER_BLOB};

/// Controls the update rate of the blob base fee based on `target_blobs_per_block`.
pub const BLOB_BASE_FEE_UPDATE_FRACTION_PER_TARGET_BLOB: u64 = 1112825;

/// Calculates the `excess_blob_gas` from the parent header's `blob_gas_used`, `excess_blob_gas` and
/// `target_blobs_per_block`.
///
/// Similar to [crate::eip4844::calc_excess_blob_gas], but derives the target blob gas from
/// `parent_target_blobs_per_block`.
#[inline]
pub const fn calc_excess_blob_gas(
    parent_excess_blob_gas: u64,
    parent_blob_gas_used: u64,
    parent_target_blobs_per_block: u64,
) -> u64 {
    (parent_excess_blob_gas + parent_blob_gas_used)
        .saturating_sub(DATA_GAS_PER_BLOB * parent_target_blobs_per_block)
}

/// Calculates the blob gas price from the header's excess blob gas field.
///
/// Similar to [crate::eip4844::calc_blob_gasprice], but adjusts the update rate based on
/// `target_blobs_per_block`.
#[inline]
pub fn calc_blob_gasprice(excess_blob_gas: u64, target_blobs_per_block: u64) -> u128 {
    let update_fraction = BLOB_BASE_FEE_UPDATE_FRACTION_PER_TARGET_BLOB * target_blobs_per_block;
    fake_exponential(BLOB_TX_MIN_BLOB_GASPRICE, excess_blob_gas as u128, update_fraction as u128)
}
