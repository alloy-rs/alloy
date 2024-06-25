use alloy_eips::BlockId;
use alloy_primitives::{Address, Log};
use serde::{Deserialize, Serialize};

/// Optional fields to override simulation state.
#[derive(Deserialize, Debug, Serialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SimBundleOverrides {
    /// Block used for simulation state. Defaults to latest block.
    /// Block header data will be derived from parent block by default.
    /// Specify other params to override the default values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_block: Option<BlockId>,
    /// Block number used for simulation, defaults to parentBlock.number + 1
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub block_number: Option<u64>,
    /// Coinbase used for simulation, defaults to parentBlock.coinbase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coinbase: Option<Address>,
    /// Timestamp used for simulation, defaults to parentBlock.timestamp + 12
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    /// Gas limit used for simulation, defaults to parentBlock.gasLimit
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<u64>,
    /// Base fee used for simulation, defaults to parentBlock.baseFeePerGas
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub base_fee: Option<u64>,
    /// Timeout in seconds, defaults to 5
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Response from the matchmaker after sending a simulation request.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SimBundleResponse {
    /// Whether the simulation was successful.
    pub success: bool,
    /// Error message if the simulation failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The block number of the simulated block.
    #[serde(with = "alloy_serde::quantity")]
    pub state_block: u64,
    /// The gas price of the simulated block.
    #[serde(with = "alloy_serde::quantity")]
    pub mev_gas_price: u64,
    /// The profit of the simulated block.
    #[serde(with = "alloy_serde::quantity")]
    pub profit: u64,
    /// The refundable value of the simulated block.
    #[serde(with = "alloy_serde::quantity")]
    pub refundable_value: u64,
    /// The gas used by the simulated block.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// Logs returned by `mev_simBundle`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<SimBundleLogs>>,
}

/// Logs returned by `mev_simBundle`.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SimBundleLogs {
    /// Logs for transactions in bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_logs: Option<Vec<Log>>,
    /// Logs for bundles in bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_logs: Option<Vec<SimBundleLogs>>,
}

#[cfg(test)]
mod tests {
    use super::SimBundleResponse;

    #[test]
    fn can_dererialize_sim_response() {
        let expected = r#"
        {
            "success": true,
            "stateBlock": "0x8b8da8",
            "mevGasPrice": "0x74c7906005",
            "profit": "0x4bc800904fc000",
            "refundableValue": "0x4bc800904fc000",
            "gasUsed": "0xa620",
            "logs": [{},{}]
          }
        "#;
        let actual: SimBundleResponse = serde_json::from_str(expected).unwrap();
        assert!(actual.success);
    }
}
