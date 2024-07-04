//! Provider-related utilities.

use alloy_primitives::{U128, U64};

/// The default percentile of gas premiums that are fetched for fee estimation.
pub(crate) const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 20.0;

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub(crate) const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 10;

/// Convert `U128` to `u128`.
pub(crate) fn convert_u128(r: U128) -> u128 {
    r.to::<u128>()
}

/// Convert `U64` to `u64`.
pub(crate) fn convert_u64(r: U64) -> u64 {
    r.to::<u64>()
}
