use crate::common::{Privacy, ProtocolVersion, Validity};

use alloy_eips::BlockId;
use alloy_primitives::{Address, Bytes, Log, TxHash};
use serde::{Deserialize, Serialize};

/// A bundle of transactions to send to the matchmaker.
///
/// Note: this is for `mev_sendBundle` and not `eth_sendBundle`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendBundleRequest {
    /// The version of the MEV-share API to use.
    #[serde(rename = "version")]
    pub protocol_version: ProtocolVersion,
    /// Data used by block builders to check if the bundle should be considered for inclusion.
    #[serde(rename = "inclusion")]
    pub inclusion: Inclusion,
    /// The transactions to include in the bundle.
    #[serde(rename = "body")]
    pub bundle_body: Vec<BundleItem>,
    /// Requirements for the bundle to be included in the block.
    #[serde(rename = "validity", skip_serializing_if = "Option::is_none")]
    pub validity: Option<Validity>,
    /// Preferences on what data should be shared about the bundle and its transactions
    #[serde(rename = "privacy", skip_serializing_if = "Option::is_none")]
    pub privacy: Option<Privacy>,
}

impl SendBundleRequest {
    /// Create a new bundle request.
    pub const fn new(
        block_num: u64,
        max_block: Option<u64>,
        protocol_version: ProtocolVersion,
        bundle_body: Vec<BundleItem>,
    ) -> Self {
        Self {
            protocol_version,
            inclusion: Inclusion { block: block_num, max_block },
            bundle_body,
            validity: None,
            privacy: None,
        }
    }
}

/// Data used by block builders to check if the bundle should be considered for inclusion.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Inclusion {
    /// The first block the bundle is valid for.
    #[serde(with = "alloy_serde::quantity")]
    pub block: u64,
    /// The last block the bundle is valid for.
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub max_block: Option<u64>,
}

impl Inclusion {
    /// Creates a new inclusion with the given min block..
    pub const fn at_block(block: u64) -> Self {
        Self { block, max_block: None }
    }

    /// Returns the block number of the first block the bundle is valid for.
    #[inline]
    pub const fn block_number(&self) -> u64 {
        self.block
    }

    /// Returns the block number of the last block the bundle is valid for.
    #[inline]
    pub fn max_block_number(&self) -> Option<u64> {
        self.max_block.as_ref().map(|b| *b)
    }
}

/// A bundle tx, which can either be a transaction hash, or a full tx.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum BundleItem {
    /// The hash of either a transaction or bundle we are trying to backrun.
    Hash {
        /// Tx hash.
        hash: TxHash,
    },
    /// A new signed transaction.
    #[serde(rename_all = "camelCase")]
    Tx {
        /// Bytes of the signed transaction.
        tx: Bytes,
        /// If true, the transaction can revert without the bundle being considered invalid.
        can_revert: bool,
    },
}

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
    use std::str::FromStr;

    use crate::{common::PrivacyHint, RefundConfig};
    use alloy_primitives::Bytes;

    use super::*;

    #[test]
    fn can_deserialize_simple() {
        let str = r#"
        [{
            "version": "v0.1",
            "inclusion": {
                "block": "0x1"
            },
            "body": [{
                "tx": "0x02f86b0180843b9aca00852ecc889a0082520894c87037874aed04e51c29f582394217a0a2b89d808080c080a0a463985c616dd8ee17d7ef9112af4e6e06a27b071525b42182fe7b0b5c8b4925a00af5ca177ffef2ff28449292505d41be578bebb77110dfc09361d2fb56998260",
                "canRevert": false
            }]
        }]
        "#;
        let res: Result<Vec<SendBundleRequest>, _> = serde_json::from_str(str);
        assert!(res.is_ok());
    }

    #[test]
    fn can_deserialize_complex() {
        let str = r#"
        [{
            "version": "v0.1",
            "inclusion": {
                "block": "0x1"
            },
            "body": [{
                "tx": "0x02f86b0180843b9aca00852ecc889a0082520894c87037874aed04e51c29f582394217a0a2b89d808080c080a0a463985c616dd8ee17d7ef9112af4e6e06a27b071525b42182fe7b0b5c8b4925a00af5ca177ffef2ff28449292505d41be578bebb77110dfc09361d2fb56998260",
                "canRevert": false
            }],
            "privacy": {
                "hints": [
                  "calldata"
                ]
              },
              "validity": {
                "refundConfig": [
                  {
                    "address": "0x8EC1237b1E80A6adf191F40D4b7D095E21cdb18f",
                    "percent": 100
                  }
                ]
              }
        }]
        "#;
        let res: Result<Vec<SendBundleRequest>, _> = serde_json::from_str(str);
        assert!(res.is_ok());
    }

    #[test]
    fn can_serialize_complex() {
        let str = r#"
        [{
            "version": "v0.1",
            "inclusion": {
                "block": "0x1"
            },
            "body": [{
                "tx": "0x02f86b0180843b9aca00852ecc889a0082520894c87037874aed04e51c29f582394217a0a2b89d808080c080a0a463985c616dd8ee17d7ef9112af4e6e06a27b071525b42182fe7b0b5c8b4925a00af5ca177ffef2ff28449292505d41be578bebb77110dfc09361d2fb56998260",
                "canRevert": false
            }],
            "privacy": {
                "hints": [
                  "calldata"
                ]
              },
              "validity": {
                "refundConfig": [
                  {
                    "address": "0x8EC1237b1E80A6adf191F40D4b7D095E21cdb18f",
                    "percent": 100
                  }
                ]
              }
        }]
        "#;
        let bundle_body = vec![BundleItem::Tx {
            tx: Bytes::from_str("0x02f86b0180843b9aca00852ecc889a0082520894c87037874aed04e51c29f582394217a0a2b89d808080c080a0a463985c616dd8ee17d7ef9112af4e6e06a27b071525b42182fe7b0b5c8b4925a00af5ca177ffef2ff28449292505d41be578bebb77110dfc09361d2fb56998260").unwrap(),
            can_revert: false,
        }];

        let validity = Some(Validity {
            refund_config: Some(vec![RefundConfig {
                address: "0x8EC1237b1E80A6adf191F40D4b7D095E21cdb18f".parse().unwrap(),
                percent: 100,
            }]),
            ..Default::default()
        });

        let privacy = Some(Privacy {
            hints: Some(PrivacyHint { calldata: true, ..Default::default() }),
            ..Default::default()
        });

        let bundle = SendBundleRequest {
            protocol_version: ProtocolVersion::V0_1,
            inclusion: Inclusion { block: 1, max_block: None },
            bundle_body,
            validity,
            privacy,
        };
        let expected = serde_json::from_str::<Vec<SendBundleRequest>>(str).unwrap();
        assert_eq!(bundle, expected[0]);
    }

    #[test]
    fn can_serialize_privacy_hint() {
        let hint = PrivacyHint {
            calldata: true,
            contract_address: true,
            logs: true,
            function_selector: true,
            hash: true,
            tx_hash: true,
        };
        let expected =
            r#"["calldata","contract_address","logs","function_selector","hash","tx_hash"]"#;
        let actual = serde_json::to_string(&hint).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_deserialize_privacy_hint() {
        let hint = PrivacyHint {
            calldata: true,
            contract_address: false,
            logs: true,
            function_selector: false,
            hash: true,
            tx_hash: false,
        };
        let expected = r#"["calldata","logs","hash"]"#;
        let actual: PrivacyHint = serde_json::from_str(expected).unwrap();
        assert_eq!(actual, hint);
    }

    #[test]
    fn can_deserialize_sim_response() {
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
