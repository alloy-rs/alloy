use serde::{Deserialize, Serialize};

/// Internal struct to calculate reward percentiles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TxGasAndReward {
    /// Gas used by the transaction
    pub gas_used: u64,
    /// The effective gas tip by the transaction
    pub reward: u128,
}

impl PartialOrd for TxGasAndReward {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TxGasAndReward {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // compare only the reward
        // see:
        // <https://github.com/ethereum/go-ethereum/blob/ee8e83fa5f6cb261dad2ed0a7bbcde4930c41e6c/eth/gasprice/feehistory.go#L85>
        self.reward.cmp(&other.reward)
    }
}

/// Response type for `eth_feeHistory`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistory {
    /// An array of block base fees per gas.
    /// This includes the next block after the newest of the returned range,
    /// because this value can be derived from the newest block. Zeroes are
    /// returned for pre-EIP-1559 blocks.
    ///
    /// # Note
    ///
    /// Empty list is skipped only for compatibility with Erigon and Geth.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        with = "alloy_serde::num::u128_vec_via_ruint"
    )]
    pub base_fee_per_gas: Vec<u128>,
    /// An array of block gas used ratios. These are calculated as the ratio
    /// of `gasUsed` and `gasLimit`.
    ///
    /// # Note
    ///
    /// The `Option` is only for compatibility with Erigon and Geth.
    // #[serde(default, with = "alloy_serde::num::f64_hex_vec")]
    pub gas_used_ratio: Vec<f64>,
    /// An array of block base fees per blob gas. This includes the next block after the newest
    /// of  the returned range, because this value can be derived from the newest block. Zeroes
    /// are returned for pre-EIP-4844 blocks.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        // with = "alloy_serde::num::u128_vec_via_ruint"
    )]
    pub base_fee_per_blob_gas: Vec<u128>,
    /// An array of block blob gas used ratios. These are calculated as the ratio of gasUsed and
    /// gasLimit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blob_gas_used_ratio: Vec<f64>,
    /// Lowest number block of the returned range.
    #[serde(default, with = "alloy_serde::num::u64_via_ruint")]
    pub oldest_block: u64,
    /// An (optional) array of effective priority fee per gas data points from a single
    /// block. All zeroes are returned if the block is empty.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_vec_vec_opt_via_ruint"
    )]
    pub reward: Option<Vec<Vec<u128>>>,
}

impl FeeHistory {
    /// Returns the base fee of the latest block in the `eth_feeHistory` request.
    pub fn latest_block_base_fee(&self) -> Option<u128> {
        // the base fee of requested block is the second last element in the
        // list
        self.base_fee_per_gas.iter().rev().nth(1).copied()
    }

    /// Returns the base fee of the next block.
    pub fn next_block_base_fee(&self) -> Option<u128> {
        self.base_fee_per_gas.last().copied()
    }

    /// Returns the blob base fee of the next block.
    ///
    /// If the next block is pre- EIP-4844, this will return `None`.
    pub fn next_block_blob_base_fee(&self) -> Option<u128> {
        self.base_fee_per_blob_gas
            .last()
            .filter(|fee| {
                // skip zero value that is returned for pre-EIP-4844 blocks
                **fee != 0
            })
            .copied()
    }

    /// Returns the blob fee of the latest block in the `eth_feeHistory` request.
    pub fn latest_block_blob_base_fee(&self) -> Option<u128> {
        // the blob fee requested block is the second last element in the list
        self.base_fee_per_blob_gas
            .iter()
            .rev()
            .nth(1)
            .filter(|fee| {
                // skip zero value that is returned for pre-EIP-4844 blocks
                **fee != 0
            })
            .copied()
    }
}
