use alloy_primitives::U256;
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
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_fee_per_gas: Vec<U256>,
    /// An array of block gas used ratios. These are calculated as the ratio
    /// of `gasUsed` and `gasLimit`.
    ///
    /// # Note
    ///
    /// The `Option` is only for compatability with Erigon and Geth.
    pub gas_used_ratio: Vec<f64>,
    /// An array of block base fees per blob gas. This includes the next block after the newest
    /// of  the returned range, because this value can be derived from the newest block. Zeroes
    /// are returned for pre-EIP-4844 blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_fee_per_blob_gas: Vec<U256>,
    /// An array of block blob gas used ratios. These are calculated as the ratio of gasUsed and
    /// gasLimit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blob_gas_used_ratio: Vec<f64>,
    /// Lowest number block of the returned range.
    pub oldest_block: U256,
    /// An (optional) array of effective priority fee per gas data points from a single
    /// block. All zeroes are returned if the block is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward: Option<Vec<Vec<U256>>>,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_tx_gas_and_reward() {
        let tx = TxGasAndReward { gas_used: 50000, reward: 200 };
        assert_eq!(tx.gas_used, 50000);
        assert_eq!(tx.reward, 200);
    }

    #[test]
    fn compare_tx_gas_and_reward() {
        let tx1 = TxGasAndReward { gas_used: 50000, reward: 300 };
        let tx2 = TxGasAndReward { gas_used: 60000, reward: 200 };
        assert!(tx1 > tx2);
    }

    #[test]
    fn equality_tx_gas_and_reward() {
        let tx1 = TxGasAndReward { gas_used: 50000, reward: 200 };
        let tx2 = tx1.clone();
        assert_eq!(tx1, tx2);
    }

    #[test]
    fn create_default_fee_history() {
        let fee_history = FeeHistory::default();
        assert_eq!(fee_history.base_fee_per_gas.len(), 0);
        assert_eq!(fee_history.gas_used_ratio.len(), 0);
        assert_eq!(fee_history.base_fee_per_blob_gas.len(), 0);
        assert_eq!(fee_history.blob_gas_used_ratio.len(), 0);
        assert_eq!(fee_history.oldest_block, U256::ZERO);
        assert!(fee_history.reward.is_none());
    }

    #[test]
    fn serialize_deserialize_fee_history() {
        let fee_history = FeeHistory {
            base_fee_per_gas: vec![U256::from(100), U256::from(200)],
            gas_used_ratio: vec![0.5, 0.75],
            base_fee_per_blob_gas: vec![U256::from(150), U256::from(250)],
            blob_gas_used_ratio: vec![0.6, 0.8],
            oldest_block: U256::from(12345),
            reward: Some(vec![vec![U256::from(100)], vec![U256::from(200)]]),
        };

        let serialized = serde_json::to_string(&fee_history).unwrap();
        let deserialized: FeeHistory = serde_json::from_str(&serialized).unwrap();

        assert_eq!(fee_history, deserialized);
    }

    #[test]
    fn tx_gas_and_reward_sorting() {
        let mut txs = vec![
            TxGasAndReward { gas_used: 10000, reward: 100 },
            TxGasAndReward { gas_used: 50000, reward: 300 },
            TxGasAndReward { gas_used: 30000, reward: 200 },
        ];

        txs.sort();
        assert_eq!(txs[0].reward, 100);
        assert_eq!(txs[1].reward, 200);
        assert_eq!(txs[2].reward, 300);
    }

    #[test]
    fn fee_history_edge_case_serialization() {
        let fee_history = FeeHistory {
            base_fee_per_gas: vec![U256::MAX, U256::from(0)],
            gas_used_ratio: vec![1.0, 0.0],
            base_fee_per_blob_gas: vec![U256::MAX, U256::from(0)],
            blob_gas_used_ratio: vec![1.0, 0.0],
            oldest_block: U256::MAX,
            reward: None,
        };

        let serialized = serde_json::to_string(&fee_history).unwrap();
        let deserialized: FeeHistory = serde_json::from_str(&serialized).unwrap();

        assert_eq!(fee_history, deserialized);
    }

    #[test]
    fn fee_history_with_values() {
        let fee_history = FeeHistory {
            base_fee_per_gas: vec![U256::from(100), U256::from(200)],
            gas_used_ratio: vec![0.5, 0.75],
            base_fee_per_blob_gas: vec![U256::from(150), U256::from(250)],
            blob_gas_used_ratio: vec![0.6, 0.8],
            oldest_block: U256::from(12345),
            reward: Some(vec![vec![U256::from(100)], vec![U256::from(200)]]),
        };

        assert!(!fee_history.base_fee_per_gas.is_empty());
        assert!(!fee_history.gas_used_ratio.is_empty());
        assert!(!fee_history.base_fee_per_blob_gas.is_empty());
        assert!(!fee_history.blob_gas_used_ratio.is_empty());
        assert_eq!(fee_history.oldest_block, U256::from(12345));
        assert!(fee_history.reward.is_some());
    }
}
