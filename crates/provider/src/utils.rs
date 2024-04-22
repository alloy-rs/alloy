//! Provider-related utilities.

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 10;
/// Multiplier for the current base fee to estimate max base fee for the next block.
pub const EIP1559_BASE_FEE_MULTIPLIER: u128 = 2;
/// The default percentile of gas premiums that are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 20.0;

/// An estimator function for EIP1559 fees.
pub type EstimatorFunction = fn(u128, &[Vec<u128>]) -> Eip1559Estimation;

/// Return type of EIP1155 gas fee estimator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eip1559Estimation {
    /// The base fee per gas.
    pub max_fee_per_gas: u128,
    /// The max priority fee per gas.
    pub max_priority_fee_per_gas: u128,
}

fn estimate_priority_fee(rewards: &[Vec<u128>]) -> u128 {
    let mut rewards =
        rewards.iter().filter_map(|r| r.first()).filter(|r| **r > 0_u128).collect::<Vec<_>>();
    if rewards.is_empty() {
        return 0_u128;
    }

    rewards.sort_unstable();

    // Return the median.
    let n = rewards.len();

    if n % 2 == 0 {
        (*rewards[n / 2 - 1] + *rewards[n / 2]) / 2
    } else {
        *rewards[n / 2]
    }
}

/// The default EIP-1559 fee estimator which is based on the work by [MetaMask](https://github.com/MetaMask/core/blob/main/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56)
/// (constants for "medium" priority level are used)
pub fn eip1559_default_estimator(
    base_fee_per_gas: u128,
    rewards: &[Vec<u128>],
) -> Eip1559Estimation {
    let max_priority_fee_per_gas = estimate_priority_fee(rewards);
    let potential_max_fee = base_fee_per_gas * EIP1559_BASE_FEE_MULTIPLIER;

    Eip1559Estimation {
        max_fee_per_gas: potential_max_fee + max_priority_fee_per_gas,
        max_priority_fee_per_gas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_estimate_priority_fee() {
        let rewards =
            vec![vec![10_000_000_000_u128], vec![200_000_000_000_u128], vec![3_000_000_000_u128]];
        assert_eq!(super::estimate_priority_fee(&rewards), 10_000_000_000_u128);

        let rewards = vec![
            vec![400_000_000_000_u128],
            vec![2_000_000_000_u128],
            vec![5_000_000_000_u128],
            vec![3_000_000_000_u128],
        ];

        assert_eq!(super::estimate_priority_fee(&rewards), 4_000_000_000_u128);

        let rewards = vec![vec![0_u128], vec![0_u128], vec![0_u128]];

        assert_eq!(super::estimate_priority_fee(&rewards), 0_u128);

        assert_eq!(super::estimate_priority_fee(&[]), 0_u128);
    }

    #[test]
    fn test_eip1559_default_estimator() {
        let base_fee_per_gas = 1_000_000_000_u128;
        let rewards = vec![
            vec![200_000_000_000_u128],
            vec![200_000_000_000_u128],
            vec![300_000_000_000_u128],
        ];
        assert_eq!(
            super::eip1559_default_estimator(base_fee_per_gas, &rewards),
            Eip1559Estimation {
                max_fee_per_gas: 202_000_000_000_u128,
                max_priority_fee_per_gas: 200_000_000_000_u128
            }
        );

        let base_fee_per_gas = 0u128;
        let rewards = vec![
            vec![200_000_000_000_u128],
            vec![200_000_000_000_u128],
            vec![300_000_000_000_u128],
        ];

        assert_eq!(
            super::eip1559_default_estimator(base_fee_per_gas, &rewards),
            Eip1559Estimation {
                max_fee_per_gas: 200_000_000_000_u128,
                max_priority_fee_per_gas: 200_000_000_000_u128
            }
        );
    }
}
