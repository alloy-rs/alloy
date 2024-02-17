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
    use std::str::FromStr;

    use super::*;

    #[test]
    fn deserialize_and_validate_fee_history() {
        let json_response = r#"
        {
            "id": "1",
            "jsonrpc": "2.0",
            "result": {
                "oldestBlock": 10762137,
                "reward": [
                    ["0x4a817c7ee", "0x4a817c7ee"],
                    ["0x773593f0", "0x773593f5"],
                    ["0x0", "0x0"],
                    ["0x773593f5", "0x773bae75"]
                ],
                "baseFeePerGas": ["0x12", "0x10", "0x10", "0xe", "0xd"],
                "gasUsedRatio": [0.026089875, 0.406803, 0, 0.0866665]
            }
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(json_response).unwrap();
        let fee_history: FeeHistory = serde_json::from_value(parsed["result"].clone()).unwrap();

        assert_eq!(fee_history.oldest_block, U256::from_str("10762137").unwrap());

        let expected_rewards = vec![
            vec![
                U256::from_str_radix("4a817c7ee", 16).unwrap(),
                U256::from_str_radix("4a817c7ee", 16).unwrap(),
            ],
            vec![
                U256::from_str_radix("773593f0", 16).unwrap(),
                U256::from_str_radix("773593f5", 16).unwrap(),
            ],
            vec![U256::from(0), U256::from(0)],
            vec![
                U256::from_str_radix("773593f5", 16).unwrap(),
                U256::from_str_radix("773bae75", 16).unwrap(),
            ],
        ];
        assert_eq!(fee_history.reward.unwrap(), expected_rewards);

        let expected_base_fees = vec![
            U256::from_str_radix("12", 16).unwrap(),
            U256::from_str_radix("10", 16).unwrap(),
            U256::from_str_radix("10", 16).unwrap(),
            U256::from_str_radix("e", 16).unwrap(),
            U256::from_str_radix("d", 16).unwrap(),
        ];
        assert_eq!(fee_history.base_fee_per_gas, expected_base_fees);

        let expected_ratios = vec![0.026089875, 0.406803, 0.0, 0.0866665];
        assert_eq!(fee_history.gas_used_ratio, expected_ratios);
    }
}
