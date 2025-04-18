//! Provider-related utilities.

use crate::fillers::{BlobGasFiller, ChainIdFiller, Fillers, GasFiller, NonceFiller};
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

/// An estimator function for EIP1559 fees.
pub type EstimatorFunction = fn(u128, &[Vec<u128>]) -> Eip1559Estimation;

/// A trait responsible for estimating EIP-1559 values
pub trait Eip1559EstimatorFn: Send + Unpin {
    /// Estimates the EIP-1559 values given the latest basefee and the recent rewards.
    fn estimate(&self, base_fee: u128, rewards: &[Vec<u128>]) -> Eip1559Estimation;
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
        F: Fn(u128, &[Vec<u128>]) -> Eip1559Estimation + Send + Unpin + 'static,
    {
        Self::new_estimator(f)
    }

    /// Creates a new estimate fn
    pub fn new_estimator<F: Eip1559EstimatorFn + 'static>(f: F) -> Self {
        Self::Custom(Box::new(f))
    }

    /// Estimates the EIP-1559 values given the latest basefee and the recent rewards.
    pub fn estimate(self, base_fee: u128, rewards: &[Vec<u128>]) -> Eip1559Estimation {
        match self {
            Self::Default => eip1559_default_estimator(base_fee, rewards),
            Self::Custom(val) => val.estimate(base_fee, rewards),
        }
    }
}

impl<F> Eip1559EstimatorFn for F
where
    F: Fn(u128, &[Vec<u128>]) -> Eip1559Estimation + Send + Unpin,
{
    fn estimate(&self, base_fee: u128, rewards: &[Vec<u128>]) -> Eip1559Estimation {
        (self)(base_fee, rewards)
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

fn estimate_priority_fee(rewards: &[Vec<u128>]) -> u128 {
    let mut rewards =
        rewards.iter().filter_map(|r| r.first()).filter(|r| **r > 0_u128).collect::<Vec<_>>();
    if rewards.is_empty() {
        return EIP1559_MIN_PRIORITY_FEE;
    }

    rewards.sort_unstable();

    let n = rewards.len();

    let median =
        if n % 2 == 0 { (*rewards[n / 2 - 1] + *rewards[n / 2]) / 2 } else { *rewards[n / 2] };

    std::cmp::max(median, EIP1559_MIN_PRIORITY_FEE)
}

/// The default EIP-1559 fee estimator.
///
/// Based on the work by [MetaMask](https://github.com/MetaMask/core/blob/0fd4b397e7237f104d1c81579a0c4321624d076b/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56);
/// constants for "medium" priority level are used.
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
pub type RecommendedFillers = Fillers<(GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller)>;

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

        assert_eq!(super::estimate_priority_fee(&rewards), EIP1559_MIN_PRIORITY_FEE);

        assert_eq!(super::estimate_priority_fee(&[]), EIP1559_MIN_PRIORITY_FEE);
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
