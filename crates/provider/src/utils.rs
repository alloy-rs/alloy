//! Provider-related utilities.

use alloy_primitives::U256;

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 10;
/// Multiplier for the current base fee to estimate max base fee for the next block.
pub const EIP1559_BASE_FEE_MULTIPLIER: f64 = 2.0;
/// The default percentile of gas premiums that are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 20.0;

/// An estimator function for EIP1559 fees.
pub type EstimatorFunction = fn(U256, &[Vec<U256>]) -> (U256, U256);

fn estimate_priority_fee(rewards: &[Vec<U256>]) -> U256 {
    let mut rewards =
        rewards.iter().filter_map(|r| r.first().filter(|r| **r > U256::ZERO)).collect::<Vec<_>>();
    if rewards.is_empty() {
        return U256::ZERO;
    }

    rewards.sort();
    // Return the median.
    *rewards[rewards.len() / 2]
}

/// The default EIP-1559 fee estimator which is based on the work by [MetaMask](https://github.com/MetaMask/core/blob/main/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56)
/// (constants for "medium" priority level are used)
pub fn eip1559_default_estimator(base_fee_per_gas: U256, rewards: &[Vec<U256>]) -> (U256, U256) {
    let max_priority_fee_per_gas = estimate_priority_fee(rewards);
    let potential_max_fee = base_fee_per_gas * U256::from(EIP1559_BASE_FEE_MULTIPLIER);
    (potential_max_fee, max_priority_fee_per_gas)
}
