//! Fee scaling agrees with exact integer arithmetic over the full fee range.

use alloy_eips::eip1559::Eip1559Estimation;
use alloy_primitives::U256;

#[ephyra::test(tests = 256)]
fn fee_scaling_matches_wide_arithmetic(cap: [u8; 16], tip: [u8; 16], pct: u64) {
    let cap = u128::from_be_bytes(cap);
    let tip = u128::from_be_bytes(tip).min(cap);
    let estimate = Eip1559Estimation { max_fee_per_gas: cap, max_priority_fee_per_gas: tip };
    // Exercise common percentages as well as the full u64 range.
    for pct in [pct % 101, pct] {
        let multiplier = U256::from(100) + U256::from(pct);
        let expected =
            |fee| (U256::from(fee) * multiplier / U256::from(100)).saturating_to::<u128>();
        let scaled = estimate.scaled_by_pct(pct);
        assert_eq!(scaled.max_fee_per_gas, expected(cap));
        assert_eq!(scaled.max_priority_fee_per_gas, expected(tip));
    }
}

#[test]
fn scaling_is_exact_when_only_the_intermediate_product_overflows() {
    let cap = u128::MAX / 2;
    let estimate = Eip1559Estimation { max_fee_per_gas: cap, max_priority_fee_per_gas: 3 };
    assert_eq!(
        estimate.scaled_by_pct(50),
        Eip1559Estimation { max_fee_per_gas: cap + cap / 2, max_priority_fee_per_gas: 4 }
    );
}
