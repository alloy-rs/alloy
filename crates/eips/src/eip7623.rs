//! Constants and utils for calldata cost.
//!
//! See also [EIP-7623](https://eips.ethereum.org/EIPS/eip-7623): Increase calldata cost

/// The standard cost of calldata token.
pub const STANDARD_TOKEN_COST: u64 = 4;

/// The cost of a non-zero byte in calldata.
pub const NON_ZERO_BYTE_DATA_COST: u64 = 68;

/// The multiplier for a non zero byte in calldata.
pub const NON_ZERO_BYTE_MULTIPLIER: u64 = NON_ZERO_BYTE_DATA_COST / STANDARD_TOKEN_COST;

/// The cost floor per token
pub const TOTAL_COST_FLOOR_PER_TOKEN: u64 = 10;

/// Retrieve the total number of tokens in calldata.
#[inline]
pub fn tokens_in_calldata(input: &[u8]) -> u64 {
    let zero_data_len = input.iter().filter(|v| **v == 0).count() as u64;
    let non_zero_data_len = input.len() as u64 - zero_data_len;
    zero_data_len + non_zero_data_len * NON_ZERO_BYTE_MULTIPLIER
}

/// Calculate the transaction cost floor as specified in EIP-7623.
///
/// Any transaction with a gas limit below this value is considered invalid.
#[inline]
pub const fn transaction_floor_cost(tokens_in_calldata: u64) -> u64 {
    21_000 + TOTAL_COST_FLOOR_PER_TOKEN * tokens_in_calldata
}
