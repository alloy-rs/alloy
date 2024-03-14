//! Provider-related utilities.

use alloy_primitives::{I256, U256};

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 5;
/// Multiplier for the current base fee to estimate max base fee for the next block.
pub const EIP1559_BASE_FEE_MULTIPLIER: f64 = 2.0;
/// The default percentile of gas premiums that are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 20.0;
/// The threshold max change/difference (in %) at which we will ignore the fee history values
/// under it.
pub const EIP1559_FEE_ESTIMATION_THRESHOLD_MAX_CHANGE: i64 = 200;

/// An estimator function for EIP1559 fees.
pub type EstimatorFunction = fn(U256, Vec<Vec<U256>>) -> (U256, U256);

fn estimate_priority_fee(rewards: Vec<Vec<U256>>) -> U256 {
    let mut rewards: Vec<U256> = rewards.iter().map(|r| r[0]).filter(|r| *r > U256::ZERO).collect();
    if rewards.is_empty() {
        return U256::ZERO;
    }
    if rewards.len() == 1 {
        return rewards[0];
    }
    // Sort the rewards as we will eventually take the median.
    rewards.sort();

    // A copy of the same vector is created for convenience to calculate percentage change
    // between subsequent fee values.
    let mut rewards_copy = rewards.clone();
    rewards_copy.rotate_left(1);

    let mut percentage_change: Vec<I256> = rewards
        .iter()
        .zip(rewards_copy.iter())
        .map(|(a, b)| {
            let a = I256::try_from(*a).expect("priority fee overflow");
            let b = I256::try_from(*b).expect("priority fee overflow");
            ((b - a) * I256::try_from(100).expect("Unexpected overflow")) / a
        })
        .collect();
    percentage_change.pop();

    // Fetch the max of the percentage change, and that element's index.
    let max_change = percentage_change.iter().max().unwrap();
    let max_change_index = percentage_change.iter().position(|&c| c == *max_change).unwrap();

    // If we encountered a big change in fees at a certain position, then consider only
    // the values >= it.
    let values = if *max_change
        >= I256::from_raw(U256::from(EIP1559_FEE_ESTIMATION_THRESHOLD_MAX_CHANGE))
        && (max_change_index >= (rewards.len() / 2))
    {
        rewards[max_change_index..].to_vec()
    } else {
        rewards
    };

    // Return the median.
    values[values.len() / 2]
}

/// The default EIP-1559 fee estimator which is based on the work by [MyCrypto](https://github.com/MyCryptoHQ/MyCrypto/blob/master/src/services/ApiService/Gas/eip1559.ts)
pub fn eip1559_default_estimator(base_fee_per_gas: U256, rewards: Vec<Vec<U256>>) -> (U256, U256) {
    let max_priority_fee_per_gas = estimate_priority_fee(rewards);
    let potential_max_fee = base_fee_per_gas * U256::from(EIP1559_BASE_FEE_MULTIPLIER);
    let max_fee_per_gas = if max_priority_fee_per_gas > potential_max_fee {
        max_priority_fee_per_gas + potential_max_fee
    } else {
        potential_max_fee
    };
    (max_fee_per_gas, max_priority_fee_per_gas)
}
