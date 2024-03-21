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

/// Return type of EIP1155 gas fee estimator.
#[derive(Debug, Clone, Copy)]
pub struct Eip1559Estimation {
    /// The base fee per gas.
    pub max_fee_per_gas: U256,
    /// The max priority fee per gas.
    pub max_priority_fee_per_gas: U256,
}

fn estimate_priority_fee(rewards: &[Vec<U256>]) -> U256 {
    let mut rewards =
        rewards.iter().filter_map(|r| r.first()).filter(|r| **r > U256::ZERO).collect::<Vec<_>>();
    if rewards.is_empty() {
        return U256::ZERO;
    }

    rewards.sort_unstable();

    // Return the median.
    let n = rewards.len();

    if n % 2 == 0 {
        (*rewards[n / 2 - 1] + *rewards[n / 2]) / U256::from(2)
    } else {
        *rewards[n / 2]
    }
}

/// The default EIP-1559 fee estimator which is based on the work by [MetaMask](https://github.com/MetaMask/core/blob/main/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56)
/// (constants for "medium" priority level are used)
pub fn eip1559_default_estimator(base_fee_per_gas: U256, rewards: &[Vec<U256>]) -> (U256, U256) {
    let max_priority_fee_per_gas = estimate_priority_fee(rewards);
    let potential_max_fee = base_fee_per_gas * U256::from(EIP1559_BASE_FEE_MULTIPLIER);
    (potential_max_fee, max_priority_fee_per_gas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_estimate_priority_fee() {
        let rewards = vec![
            vec![U256::from(10_000_000_000_u64)],
            vec![U256::from(200_000_000_000_u64)],
            vec![U256::from(3_000_000_000_u64)],
        ];
        assert_eq!(super::estimate_priority_fee(&rewards), U256::from(10_000_000_000_u64));

        let rewards = vec![
            vec![U256::from(400_000_000_000_u64)],
            vec![U256::from(2_000_000_000_u64)],
            vec![U256::from(5_000_000_000_u64)],
            vec![U256::from(3_000_000_000_u64)],
        ];

        assert_eq!(super::estimate_priority_fee(&rewards), U256::from(4_000_000_000_u64));

        let rewards = vec![vec![U256::from(0)], vec![U256::from(0)], vec![U256::from(0)]];

        assert_eq!(super::estimate_priority_fee(&rewards), U256::from(0));

        assert_eq!(super::estimate_priority_fee(&[]), U256::from(0));
    }

    #[test]
    fn test_eip1559_default_estimator() {
        let base_fee_per_gas = U256::from(1_000_000_000_u64);
        let rewards = vec![
            vec![U256::from(200_000_000_000_u64)],
            vec![U256::from(200_000_000_000_u64)],
            vec![U256::from(300_000_000_000_u64)],
        ];
        assert_eq!(
            super::eip1559_default_estimator(base_fee_per_gas, &rewards),
            (U256::from(2_000_000_000_u64), U256::from(200_000_000_000_u64))
        );
    }
}
