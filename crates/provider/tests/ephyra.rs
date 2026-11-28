//! The default estimator uses the median nonzero first reward from each block.

use alloy_primitives::U256;
use alloy_provider::utils::{
    eip1559_default_estimator, EIP1559_BASE_FEE_MULTIPLIER, EIP1559_MIN_PRIORITY_FEE,
};

#[derive(Clone, Debug, ephyra::Arbitrary)]
enum Fee {
    Zero,
    Small(u64),
    FullWidth([u8; 16]),
}

impl Fee {
    fn wei(self) -> u128 {
        match self {
            Self::Zero => 0,
            Self::Small(value) => u128::from(value),
            Self::FullWidth(bytes) => u128::from_be_bytes(bytes),
        }
    }
}

#[ephyra::test(tests = 256)]
fn default_fees_match_reward_history(base_fee: Fee, rows: Vec<Vec<Fee>>) {
    let base_fee = base_fee.wei();
    let rewards: Vec<Vec<_>> =
        rows.into_iter().map(|row| row.into_iter().map(Fee::wei).collect()).collect();
    let mut samples: Vec<_> =
        rewards.iter().filter_map(|row| row.first().copied()).filter(|&fee| fee > 0).collect();
    samples.sort_unstable();
    let tip = if samples.is_empty() {
        EIP1559_MIN_PRIORITY_FEE
    } else {
        let lower = samples[(samples.len() - 1) / 2];
        let upper = samples[samples.len() / 2];
        ((U256::from(lower) + U256::from(upper)) / U256::from(2)).to::<u128>()
    };
    let cap = U256::from(base_fee) * U256::from(EIP1559_BASE_FEE_MULTIPLIER) + U256::from(tip);
    let estimate = eip1559_default_estimator(base_fee, &rewards);
    assert_eq!(estimate.max_priority_fee_per_gas, tip);
    assert_eq!(estimate.max_fee_per_gas, cap.saturating_to::<u128>());
}

#[test]
fn median_rounds_down_even_when_the_sum_overflows() {
    let estimate = eip1559_default_estimator(0, &[vec![u128::MAX - 1], vec![u128::MAX]]);
    assert_eq!(estimate.max_priority_fee_per_gas, u128::MAX - 1);
    assert_eq!(estimate.max_fee_per_gas, u128::MAX - 1);
}

#[test]
fn unrepresentable_estimated_cap_saturates() {
    for base_fee in [u128::MAX / 2, u128::MAX / 2 + 1] {
        let estimate = eip1559_default_estimator(base_fee, &[vec![2]]);
        assert_eq!(estimate.max_fee_per_gas, u128::MAX);
        assert_eq!(estimate.max_priority_fee_per_gas, 2);
    }
}
