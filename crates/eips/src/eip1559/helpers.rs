use crate::eip1559::{BaseFeeParams, DEFAULT_ELASTICITY_MULTIPLIER, MIN_PROTOCOL_PRIORITY_FEE};

/// Calculate the base fee for the next block based on the EIP-1559 specification.
///
/// This function calculates the base fee for the next block according to the rules defined in the
/// EIP-1559. EIP-1559 introduces a new transaction pricing mechanism that includes a
/// fixed-per-block network fee that is burned and dynamically adjusts block sizes to handles
/// transient congestion.
///
/// For each block, the base fee per gas is determined by the gas used in the parent block and the
/// target gas (the block gas limit divided by the elasticity multiplier). The algorithm increases
/// the base fee when blocks are congested and decreases it when they are under the target gas
/// usage. The base fee per gas is always burned.
///
/// Parameters:
/// - `gas_used`: The gas used in the current block.
/// - `gas_limit`: The gas limit of the current block.
/// - `base_fee`: The current base fee per gas.
/// - `base_fee_params`: Base fee parameters such as elasticity multiplier and max change
///   denominator.
///
/// Returns:
/// The calculated base fee for the next block as a `u64`.
///
/// For more information, refer to the [EIP-1559 spec](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1559.md).
pub fn calc_next_block_base_fee(
    gas_used: u128,
    gas_limit: u128,
    base_fee: u128,
    base_fee_params: BaseFeeParams,
) -> u128 {
    // Calculate the target gas by dividing the gas limit by the elasticity multiplier.
    let gas_target = gas_limit / base_fee_params.elasticity_multiplier;

    match gas_used.cmp(&gas_target) {
        // If the gas used in the current block is equal to the gas target, the base fee remains the
        // same (no increase).
        core::cmp::Ordering::Equal => base_fee,
        // If the gas used in the current block is greater than the gas target, calculate a new
        // increased base fee.
        core::cmp::Ordering::Greater => {
            // Calculate the increase in base fee based on the formula defined by EIP-1559.
            base_fee
                + (core::cmp::max(
                    // Ensure a minimum increase of 1.
                    1,
                    base_fee * (gas_used - gas_target)
                        / (gas_target * base_fee_params.max_change_denominator),
                ))
        }
        // If the gas used in the current block is less than the gas target, calculate a new
        // decreased base fee.
        core::cmp::Ordering::Less => {
            // Calculate the decrease in base fee based on the formula defined by EIP-1559.
            base_fee.saturating_sub(
                base_fee * (gas_target - gas_used)
                    / (gas_target * base_fee_params.max_change_denominator),
            )
        }
    }
}

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
        return MIN_PROTOCOL_PRIORITY_FEE as u128;
    }

    rewards.sort_unstable();

    let n = rewards.len();

    let median =
        if n % 2 == 0 { (*rewards[n / 2 - 1] + *rewards[n / 2]) / 2 } else { *rewards[n / 2] };

    std::cmp::max(median, MIN_PROTOCOL_PRIORITY_FEE as u128)
}

/// The default EIP-1559 fee estimator which is based on the work by [MetaMask](https://github.com/MetaMask/core/blob/main/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56)
/// (constants for "medium" priority level are used)
pub fn eip1559_default_estimator(
    base_fee_per_gas: u128,
    rewards: &[Vec<u128>],
) -> Eip1559Estimation {
    let max_priority_fee_per_gas = estimate_priority_fee(rewards);
    let potential_max_fee = base_fee_per_gas * DEFAULT_ELASTICITY_MULTIPLIER as u128;

    Eip1559Estimation {
        max_fee_per_gas: potential_max_fee + max_priority_fee_per_gas,
        max_priority_fee_per_gas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eip1559::constants::{MIN_PROTOCOL_BASE_FEE, MIN_PROTOCOL_BASE_FEE_U256};

    #[test]
    fn min_protocol_sanity() {
        assert_eq!(MIN_PROTOCOL_BASE_FEE_U256.to::<u64>(), MIN_PROTOCOL_BASE_FEE);
    }

    #[test]
    fn calculate_base_fee_success() {
        let base_fee = [
            1000000000, 1000000000, 1000000000, 1072671875, 1059263476, 1049238967, 1049238967, 0,
            1, 2,
        ];
        let gas_used = [
            10000000, 10000000, 10000000, 9000000, 10001000, 0, 10000000, 10000000, 10000000,
            10000000,
        ];
        let gas_limit = [
            10000000, 12000000, 14000000, 10000000, 14000000, 2000000, 18000000, 18000000,
            18000000, 18000000,
        ];
        let next_base_fee = [
            1125000000, 1083333333, 1053571428, 1179939062, 1116028649, 918084097, 1063811730, 1,
            2, 3,
        ];

        for i in 0..base_fee.len() {
            assert_eq!(
                next_base_fee[i],
                calc_next_block_base_fee(
                    gas_used[i],
                    gas_limit[i],
                    base_fee[i],
                    BaseFeeParams::ethereum(),
                )
            );
        }
    }

    #[test]
    fn calculate_optimism_sepolia_base_fee_success() {
        let base_fee = [
            1000000000, 1000000000, 1000000000, 1072671875, 1059263476, 1049238967, 1049238967, 0,
            1, 2,
        ];
        let gas_used = [
            10000000, 10000000, 10000000, 9000000, 10001000, 0, 10000000, 10000000, 10000000,
            10000000,
        ];
        let gas_limit = [
            10000000, 12000000, 14000000, 10000000, 14000000, 2000000, 18000000, 18000000,
            18000000, 18000000,
        ];
        let next_base_fee = [
            1100000048, 1080000000, 1065714297, 1167067046, 1128881311, 1028254188, 1098203452, 1,
            2, 3,
        ];

        for i in 0..base_fee.len() {
            assert_eq!(
                next_base_fee[i],
                calc_next_block_base_fee(
                    gas_used[i] as u128,
                    gas_limit[i] as u128,
                    base_fee[i] as u128,
                    BaseFeeParams::optimism_sepolia(),
                ) as u64
            );
        }
    }

    #[test]
    fn calculate_optimism_base_fee_success() {
        let base_fee = [
            1000000000, 1000000000, 1000000000, 1072671875, 1059263476, 1049238967, 1049238967, 0,
            1, 2,
        ];
        let gas_used = [
            10000000, 10000000, 10000000, 9000000, 10001000, 0, 10000000, 10000000, 10000000,
            10000000,
        ];
        let gas_limit = [
            10000000, 12000000, 14000000, 10000000, 14000000, 2000000, 18000000, 18000000,
            18000000, 18000000,
        ];
        let next_base_fee = [
            1100000048, 1080000000, 1065714297, 1167067046, 1128881311, 1028254188, 1098203452, 1,
            2, 3,
        ];

        for i in 0..base_fee.len() {
            assert_eq!(
                next_base_fee[i],
                calc_next_block_base_fee(
                    gas_used[i] as u128,
                    gas_limit[i] as u128,
                    base_fee[i] as u128,
                    BaseFeeParams::optimism(),
                ) as u64
            );
        }
    }

    #[test]
    fn calculate_optimism_canyon_base_fee_success() {
        let base_fee = [
            1000000000, 1000000000, 1000000000, 1072671875, 1059263476, 1049238967, 1049238967, 0,
            1, 2,
        ];
        let gas_used = [
            10000000, 10000000, 10000000, 9000000, 10001000, 0, 10000000, 10000000, 10000000,
            10000000,
        ];
        let gas_limit = [
            10000000, 12000000, 14000000, 10000000, 14000000, 2000000, 18000000, 18000000,
            18000000, 18000000,
        ];
        let next_base_fee = [
            1020000009, 1016000000, 1013142859, 1091550909, 1073187043, 1045042012, 1059031864, 1,
            2, 3,
        ];

        for i in 0..base_fee.len() {
            assert_eq!(
                next_base_fee[i],
                calc_next_block_base_fee(
                    gas_used[i] as u128,
                    gas_limit[i] as u128,
                    base_fee[i] as u128,
                    BaseFeeParams::optimism_canyon(),
                ) as u64
            );
        }
    }

    #[test]
    fn calculate_base_sepolia_base_fee_success() {
        let base_fee = [
            1000000000, 1000000000, 1000000000, 1072671875, 1059263476, 1049238967, 1049238967, 0,
            1, 2,
        ];
        let gas_used = [
            10000000, 10000000, 10000000, 9000000, 10001000, 0, 10000000, 10000000, 10000000,
            10000000,
        ];
        let gas_limit = [
            10000000, 12000000, 14000000, 10000000, 14000000, 2000000, 18000000, 18000000,
            18000000, 18000000,
        ];
        let next_base_fee = [
            1180000000, 1146666666, 1122857142, 1244299375, 1189416692, 1028254188, 1144836295, 1,
            2, 3,
        ];

        for i in 0..base_fee.len() {
            assert_eq!(
                next_base_fee[i],
                calc_next_block_base_fee(
                    gas_used[i] as u128,
                    gas_limit[i] as u128,
                    base_fee[i] as u128,
                    BaseFeeParams::base_sepolia(),
                ) as u64
            );
        }
    }

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

        assert_eq!(super::estimate_priority_fee(&rewards), MIN_PROTOCOL_PRIORITY_FEE as u128);

        assert_eq!(super::estimate_priority_fee(&[]), MIN_PROTOCOL_PRIORITY_FEE as u128);
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
