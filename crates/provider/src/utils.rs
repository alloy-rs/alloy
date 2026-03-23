//! Provider-related utilities.

use crate::{
    fillers::{BlobGasFiller, ChainIdFiller, GasFiller, JoinFill, NonceFiller},
    Identity,
};
use alloy_json_rpc::RpcRecv;
use alloy_network::BlockResponse;
use alloy_primitives::{B256, U128, U64};
use alloy_rpc_client::WeakClient;
use alloy_transport::{TransportError, TransportResult};
use std::{fmt, fmt::Formatter};

pub use alloy_eips::eip1559::Eip1559Estimation;

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 10;
/// Multiplier for the current base fee to estimate max base fee for the next block.
pub const EIP1559_BASE_FEE_MULTIPLIER: u128 = 2;
/// The default percentile of gas premiums that are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 20.0;
/// The minimum priority fee to provide.
pub const EIP1559_MIN_PRIORITY_FEE: u128 = 1;
/// Minimum gas used ratio for a block's reward to be included in fee estimation.
/// Blocks below this threshold are considered "near-empty" and their rewards are ignored
/// to prevent outlier tips from skewing estimates on low-traffic chains.
pub const EIP1559_FEE_ESTIMATION_MIN_GAS_USED_RATIO: f64 = 0.1;
/// Maximum multiplier of base fee used to cap the estimated priority fee.
/// Prevents the priority fee from exceeding `base_fee * N` even after filtering.
pub const EIP1559_PRIORITY_FEE_BASE_FEE_CAP_MULTIPLIER: u128 = 3;

/// Context for EIP-1559 fee estimation, holding read-only references to fee history data.
#[derive(Debug, Clone, Copy)]
pub struct FeeEstimationContext<'a> {
    /// The latest base fee per gas (in wei).
    pub base_fee_per_gas: u128,
    /// Per-block priority fee reward samples from `eth_feeHistory`.
    pub rewards: &'a [Vec<u128>],
    /// Per-block gas used ratio (0.0 = empty block, 1.0 = full block).
    pub gas_used_ratio: &'a [f64],
}

/// An estimator function for EIP1559 fees.
pub type EstimatorFunction = fn(&FeeEstimationContext<'_>) -> Eip1559Estimation;

/// A trait responsible for estimating EIP-1559 values
pub trait Eip1559EstimatorFn: Send + Unpin {
    /// Estimates the EIP-1559 fees given the fee estimation context.
    fn estimate(&self, ctx: &FeeEstimationContext<'_>) -> Eip1559Estimation;
}

/// EIP-1559 estimator variants
#[derive(Default)]
pub enum Eip1559Estimator {
    /// Uses the builtin estimator
    #[default]
    Default,
    /// Uses a custom estimator
    Custom(Box<dyn Eip1559EstimatorFn>),
}

impl Eip1559Estimator {
    /// Creates a new estimator from a closure
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&FeeEstimationContext<'_>) -> Eip1559Estimation + Send + Unpin + 'static,
    {
        Self::new_estimator(f)
    }

    /// Creates a new estimate fn
    pub fn new_estimator<F: Eip1559EstimatorFn + 'static>(f: F) -> Self {
        Self::Custom(Box::new(f))
    }

    /// Estimates the EIP-1559 fees given the fee estimation context.
    pub fn estimate(self, ctx: &FeeEstimationContext<'_>) -> Eip1559Estimation {
        match self {
            Self::Default => eip1559_default_estimator(ctx),
            Self::Custom(val) => val.estimate(ctx),
        }
    }
}

impl<F> Eip1559EstimatorFn for F
where
    F: Fn(&FeeEstimationContext<'_>) -> Eip1559Estimation + Send + Unpin,
{
    fn estimate(&self, ctx: &FeeEstimationContext<'_>) -> Eip1559Estimation {
        (self)(ctx)
    }
}

impl fmt::Debug for Eip1559Estimator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Eip1559Estimator")
            .field(
                "estimator",
                &match self {
                    Self::Default => "default",
                    Self::Custom(_) => "custom",
                },
            )
            .finish()
    }
}

fn estimate_priority_fee(rewards: &[Vec<u128>], gas_used_ratio: &[f64]) -> u128 {
    let mut filtered: Vec<u128> = rewards
        .iter()
        .zip(gas_used_ratio.iter())
        .filter(|(_, &ratio)| {
            ratio.is_finite() && ratio.clamp(0.0, 1.0) >= EIP1559_FEE_ESTIMATION_MIN_GAS_USED_RATIO
        })
        .filter_map(|(r, _)| r.first().copied())
        .filter(|&r| r > 0)
        .collect();

    if filtered.is_empty() {
        return EIP1559_MIN_PRIORITY_FEE;
    }

    filtered.sort_unstable();

    let n = filtered.len();
    let median =
        if n % 2 == 0 { (filtered[n / 2 - 1] + filtered[n / 2]) / 2 } else { filtered[n / 2] };

    std::cmp::max(median, EIP1559_MIN_PRIORITY_FEE)
}

/// The default EIP-1559 fee estimator.
///
/// Based on the work by [MetaMask](https://github.com/MetaMask/core/blob/0fd4b397e7237f104d1c81579a0c4321624d076b/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56);
/// constants for "medium" priority level are used.
///
/// Improvements over the original:
/// - Blocks with gas used ratio below [`EIP1559_FEE_ESTIMATION_MIN_GAS_USED_RATIO`] are excluded
///   from reward sampling to avoid outlier tips on low-traffic chains.
/// - Priority fee is capped at `base_fee * [`EIP1559_PRIORITY_FEE_BASE_FEE_CAP_MULTIPLIER`]` as a
///   safety bound.
pub fn eip1559_default_estimator(ctx: &FeeEstimationContext<'_>) -> Eip1559Estimation {
    let priority_fee = estimate_priority_fee(ctx.rewards, ctx.gas_used_ratio);

    // Cap priority fee relative to base fee to prevent extreme overestimates.
    let capped = std::cmp::min(
        priority_fee,
        ctx.base_fee_per_gas.saturating_mul(EIP1559_PRIORITY_FEE_BASE_FEE_CAP_MULTIPLIER),
    );
    let max_priority_fee_per_gas = std::cmp::max(capped, EIP1559_MIN_PRIORITY_FEE);

    let potential_max_fee = ctx.base_fee_per_gas * EIP1559_BASE_FEE_MULTIPLIER;

    Eip1559Estimation {
        max_fee_per_gas: potential_max_fee + max_priority_fee_per_gas,
        max_priority_fee_per_gas,
    }
}

/// Convert `U128` to `u128`.
pub(crate) fn convert_u128(r: U128) -> u128 {
    r.to::<u128>()
}

pub(crate) fn convert_u64(r: U64) -> u64 {
    r.to::<u64>()
}

pub(crate) fn convert_to_hashes<BlockResp: alloy_network::BlockResponse>(
    r: Option<BlockResp>,
) -> Option<BlockResp> {
    r.map(|mut block| {
        if block.transactions().is_empty() {
            block.transactions_mut().convert_to_hashes();
        }

        block
    })
}

/// Fetches full blocks for a list of block hashes
pub(crate) async fn hashes_to_blocks<BlockResp: BlockResponse + RpcRecv>(
    hashes: Vec<B256>,
    client: WeakClient,
    full: bool,
) -> TransportResult<Vec<Option<BlockResp>>> {
    let client = client.upgrade().ok_or(TransportError::local_usage_str("client dropped"))?;
    let blocks = futures::future::try_join_all(hashes.into_iter().map(|hash| {
        client
            .request::<_, Option<BlockResp>>("eth_getBlockByHash", (hash, full))
            .map_resp(|resp| if !full { convert_to_hashes(resp) } else { resp })
    }))
    .await?;
    Ok(blocks)
}

/// Helper type representing the joined recommended fillers i.e [`GasFiller`],
/// [`BlobGasFiller`], [`NonceFiller`], and [`ChainIdFiller`].
pub type JoinedRecommendedFillers = JoinFill<
    Identity,
    JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_estimate_priority_fee() {
        // Normal blocks (ratio > 0.1): median of [3B, 10B, 200B] = 10B
        let rewards =
            vec![vec![10_000_000_000_u128], vec![200_000_000_000_u128], vec![3_000_000_000_u128]];
        let ratios = vec![0.5, 0.8, 0.3];
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), 10_000_000_000_u128);

        // Even count: median of [2B, 3B, 5B, 400B] = (3B+5B)/2 = 4B
        let rewards = vec![
            vec![400_000_000_000_u128],
            vec![2_000_000_000_u128],
            vec![5_000_000_000_u128],
            vec![3_000_000_000_u128],
        ];
        let ratios = vec![0.9, 0.5, 0.6, 0.4];
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), 4_000_000_000_u128);

        // All zero rewards -> min priority fee
        let rewards = vec![vec![0_u128], vec![0_u128], vec![0_u128]];
        let ratios = vec![0.5, 0.5, 0.5];
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), EIP1559_MIN_PRIORITY_FEE);

        // Empty rewards -> min priority fee
        assert_eq!(super::estimate_priority_fee(&[], &[]), EIP1559_MIN_PRIORITY_FEE);
    }

    #[test]
    fn test_eip1559_default_estimator() {
        let base_fee_per_gas = 1_000_000_000_u128;
        let rewards = vec![
            vec![200_000_000_000_u128],
            vec![200_000_000_000_u128],
            vec![300_000_000_000_u128],
        ];
        let ratios = vec![0.5, 0.6, 0.7];
        let ctx = super::FeeEstimationContext {
            base_fee_per_gas,
            rewards: &rewards,
            gas_used_ratio: &ratios,
        };
        // Median = 200B, but capped at 3 * 1B = 3B
        assert_eq!(
            super::eip1559_default_estimator(&ctx),
            Eip1559Estimation {
                max_fee_per_gas: 2_000_000_000_u128 + 3_000_000_000_u128,
                max_priority_fee_per_gas: 3_000_000_000_u128,
            }
        );

        let base_fee_per_gas = 0u128;
        let rewards = vec![
            vec![200_000_000_000_u128],
            vec![200_000_000_000_u128],
            vec![300_000_000_000_u128],
        ];
        let ratios = vec![0.5, 0.6, 0.7];
        let ctx = super::FeeEstimationContext {
            base_fee_per_gas,
            rewards: &rewards,
            gas_used_ratio: &ratios,
        };
        // base_fee=0, cap=0, so priority_fee falls back to MIN_PRIORITY_FEE
        assert_eq!(
            super::eip1559_default_estimator(&ctx),
            Eip1559Estimation {
                max_fee_per_gas: EIP1559_MIN_PRIORITY_FEE,
                max_priority_fee_per_gas: EIP1559_MIN_PRIORITY_FEE,
            }
        );
    }

    #[test]
    fn test_estimate_priority_fee_all_low_ratio() {
        // All blocks below 10% gas usage — rewards ignored entirely
        let rewards = vec![vec![50_000_000_000_u128], vec![80_000_000_000_u128]];
        let ratios = vec![0.05, 0.02];
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), EIP1559_MIN_PRIORITY_FEE);
    }

    #[test]
    fn test_estimate_priority_fee_mixed_blocks() {
        // 3 empty blocks with high tips + 2 busy blocks with reasonable tips
        let rewards = vec![
            vec![500_000_000_000_u128], // empty block, high tip — should be filtered
            vec![2_000_000_000_u128],   // busy block
            vec![800_000_000_000_u128], // empty block, high tip — should be filtered
            vec![3_000_000_000_u128],   // busy block
            vec![999_000_000_000_u128], // empty block, high tip — should be filtered
        ];
        let ratios = vec![0.01, 0.5, 0.03, 0.7, 0.02];
        // Only busy blocks [2B, 3B] -> median = (2B + 3B) / 2 = 2.5B
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), 2_500_000_000_u128);
    }

    #[test]
    fn test_eip1559_default_estimator_cap() {
        let base_fee_per_gas = 1_000_000_000_u128; // 1 gwei
        let rewards = vec![vec![100_000_000_000_u128]]; // 100 gwei — way above 3x base
        let ratios = vec![0.9]; // busy block, not filtered
        let ctx = super::FeeEstimationContext {
            base_fee_per_gas,
            rewards: &rewards,
            gas_used_ratio: &ratios,
        };
        let est = super::eip1559_default_estimator(&ctx);
        // priority_fee capped at 3 * 1 gwei = 3 gwei
        assert_eq!(est.max_priority_fee_per_gas, 3_000_000_000_u128);
        assert_eq!(est.max_fee_per_gas, 2_000_000_000_u128 + 3_000_000_000_u128);
    }

    #[test]
    fn test_estimate_priority_fee_length_mismatch() {
        // rewards has 4 entries, gas_used_ratio has 2 — zip takes shortest (2)
        let rewards = vec![
            vec![1_000_000_000_u128],
            vec![2_000_000_000_u128],
            vec![3_000_000_000_u128],
            vec![4_000_000_000_u128],
        ];
        let ratios = vec![0.5, 0.8]; // only 2 ratios
                                     // zip pairs: (1B, 0.5), (2B, 0.8) — both above threshold
                                     // median of [1B, 2B] = (1B + 2B) / 2 = 1.5B
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), 1_500_000_000_u128);
    }

    #[test]
    fn test_estimate_priority_fee_invalid_ratios() {
        let rewards =
            vec![vec![5_000_000_000_u128], vec![6_000_000_000_u128], vec![7_000_000_000_u128]];
        let ratios = vec![f64::NAN, f64::INFINITY, 0.5];
        // NaN and Infinity are filtered out; only (7B, 0.5) survives
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), 7_000_000_000_u128);
    }

    #[test]
    fn test_estimate_priority_fee_boundary_ratio() {
        // Ratio exactly at threshold (0.1) should be included
        let rewards = vec![vec![10_000_000_000_u128], vec![20_000_000_000_u128]];
        let ratios = vec![0.1, 0.09];
        // Only first block (ratio = 0.1) passes, second (0.09) filtered out
        assert_eq!(super::estimate_priority_fee(&rewards, &ratios), 10_000_000_000_u128);
    }
}
