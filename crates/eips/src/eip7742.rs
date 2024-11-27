//! Contains constants and utility functions for [EIP-7742](https://eips.ethereum.org/EIPS/eip-7742)

use crate::eip4844::{self, fake_exponential, BLOB_TX_MIN_BLOB_GASPRICE, DATA_GAS_PER_BLOB};

/// Controls the update rate of the blob base fee based on `target_blobs_per_block`.
pub const BLOB_BASE_FEE_UPDATE_FRACTION_PER_TARGET_BLOB: u128 = 1112825;

/// Controls the update rate of the blob base fee based on `target_blobs_per_block`.
pub const EXCESS_BLOB_GAS_NORMALIZATION_TARGET: u64 = 128;

/// Same as [`eip4844::BLOB_GASPRICE_UPDATE_FRACTION`], but normalized for the target of 128
/// blobs.
pub const BLOB_BASE_FEE_UPDATE_FRACTION_NORMALIZED: u128 =
    BLOB_BASE_FEE_UPDATE_FRACTION_PER_TARGET_BLOB * EXCESS_BLOB_GAS_NORMALIZATION_TARGET as u128;

/// Calculates the `excess_blob_gas` for the header of the block enabling EIP-7742.
///
/// Normalizes the parent's excess blob gas as per EIP-7742.
#[inline]
pub const fn calc_excess_blob_gas_at_transition(
    parent_excess_blob_gas: u64,
    parent_blob_gas_used: u64,
    parent_target_blobs_per_block: u64,
) -> u64 {
    let normalized_parent_excess_blob_gas = parent_excess_blob_gas
        * EXCESS_BLOB_GAS_NORMALIZATION_TARGET
        / eip4844::TARGET_BLOBS_PER_BLOCK;

    calc_excess_blob_gas(
        normalized_parent_excess_blob_gas,
        parent_blob_gas_used,
        parent_target_blobs_per_block,
    )
}

/// Calculates the `excess_blob_gas` from the parent header's `blob_gas_used`, `excess_blob_gas` and
/// `target_blobs_per_block`.
///
/// Note: this function assumes that the parent block's excess blob gas is normalized as per
/// EIP-7742.
#[inline]
pub const fn calc_excess_blob_gas(
    parent_excess_blob_gas: u64,
    parent_blob_gas_used: u64,
    parent_target_blobs_per_block: u64,
) -> u64 {
    let normalized_blob_gas_used =
        parent_blob_gas_used * EXCESS_BLOB_GAS_NORMALIZATION_TARGET / parent_target_blobs_per_block;
    let normalized_target_blob_gas = DATA_GAS_PER_BLOB * EXCESS_BLOB_GAS_NORMALIZATION_TARGET;

    (parent_excess_blob_gas + normalized_blob_gas_used).saturating_sub(normalized_target_blob_gas)
}

/// Calculates the blob gas price from the header's excess blob gas field.
///
/// Similar to [crate::eip4844::calc_blob_gasprice], but adjusts the update rate based on
/// `target_blobs_per_block`.
#[inline]
pub fn calc_blob_gasprice(excess_blob_gas: u64) -> u128 {
    fake_exponential(
        BLOB_TX_MIN_BLOB_GASPRICE,
        excess_blob_gas as u128,
        BLOB_BASE_FEE_UPDATE_FRACTION_NORMALIZED,
    )
}
