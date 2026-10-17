#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use alloy_primitives::{BlockHash, Bytes, ChainId, B256, U256};
use alloy_rpc_types_eth::TransactionRequest;
use serde::{
    de::{self, IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{collections::BTreeMap, fmt};

/// Represents the params to set forking which can take various forms:
///  - untagged
///  - tagged forking
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forking {
    /// The URL of the JSON-RPC endpoint to fork from.
    pub json_rpc_url: Option<String>,
    /// The block number to fork from.
    pub block_number: Option<u64>,
}

impl<'de> serde::Deserialize<'de> for Forking {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ForkOpts {
            json_rpc_url: Option<String>,
            #[serde(default, with = "alloy_serde::quantity::opt")]
            block_number: Option<u64>,
        }

        struct ForkingVisitor;

        impl<'de> Visitor<'de> for ForkingVisitor {
            type Value = Forking;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("fork options or an object containing tagged fork options")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Forking, A::Error> {
                let mut fields = alloy_serde::OtherFields::default();
                let mut tagged = None;
                let mut duplicate = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "forking" => {
                            if tagged.is_some() {
                                return Err(de::Error::duplicate_field("forking"));
                            }
                            // Decode the tag directly so malformed or duplicate nested options
                            // cannot be hidden by buffering or untagged fallback.
                            tagged = Some(map.next_value::<ForkOpts>()?);
                        }
                        "jsonRpcUrl" | "blockNumber" => {
                            let name =
                                if key == "jsonRpcUrl" { "jsonRpcUrl" } else { "blockNumber" };
                            if fields.insert(key, map.next_value()?).is_some() {
                                duplicate = Some(name);
                            }
                        }
                        _ => {
                            let _ = map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                let options = if let Some(options) = tagged {
                    options
                } else {
                    if let Some(field) = duplicate {
                        return Err(de::Error::duplicate_field(field));
                    }
                    // Root options only apply in the absence of a tag. In particular, malformed
                    // root options must not prevent a valid tagged configuration from decoding.
                    fields.deserialize_into::<ForkOpts>().map_err(de::Error::custom)?
                };
                let ForkOpts { json_rpc_url, block_number } = options;
                Ok(Forking { json_rpc_url, block_number })
            }
        }

        deserializer.deserialize_map(ForkingVisitor)
    }
}

/// Anvil equivalent of `node_info`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    /// The current block number
    #[serde(with = "alloy_serde::quantity")]
    pub current_block_number: u64,
    /// The current block timestamp
    pub current_block_timestamp: u64,
    /// The current block hash
    pub current_block_hash: BlockHash,
    /// The enabled hardfork
    pub hard_fork: String,
    /// How transactions are ordered for mining
    #[doc(alias = "tx_order")]
    pub transaction_order: String,
    /// Info about the node's block environment
    pub environment: NodeEnvironment,
    /// Info about the node's fork configuration
    pub fork_config: NodeForkConfig,
    /// The network type this node is running (e.g., "tempo" for Tempo mode).
    /// `None` for standard Ethereum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

/// The current block environment of the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEnvironment {
    /// Base fee of the current block
    #[serde(with = "alloy_serde::quantity")]
    pub base_fee: u128,
    /// Chain id of the node.
    pub chain_id: ChainId,
    /// Configured block gas limit
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    /// Configured gas price
    #[serde(with = "alloy_serde::quantity")]
    pub gas_price: u128,
}

/// The node's fork configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeForkConfig {
    /// URL of the forked network
    pub fork_url: Option<String>,
    /// Block number of the forked network
    pub fork_block_number: Option<u64>,
    /// Retry backoff for requests
    pub fork_retry_backoff: Option<u128>,
}

/// Anvil equivalent of `hardhat_metadata`.
/// Metadata about the current Anvil instance.
/// See <https://hardhat.org/hardhat-network/docs/reference#hardhat_metadata>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Client version
    pub client_version: String,
    /// Client SemVer compatible version
    pub client_semver: Option<String>,
    /// Client commit SHA hash
    pub client_commit_sha: Option<String>,
    /// Chain id of the node.
    pub chain_id: ChainId,
    /// Unique instance id
    pub instance_id: B256,
    /// Latest block number
    pub latest_block_number: u64,
    /// Latest block hash
    pub latest_block_hash: BlockHash,
    /// Forked network info
    pub forked_network: Option<ForkedNetwork>,
    /// Snapshots of the chain
    pub snapshots: BTreeMap<U256, (u64, B256)>,
}

/// Information about the forked network.
/// See <https://hardhat.org/hardhat-network/docs/reference#hardhat_metadata>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkedNetwork {
    /// Chain id of the node.
    pub chain_id: ChainId,
    /// Block number of the forked chain
    pub fork_block_number: u64,
    /// Block hash of the forked chain
    pub fork_block_hash: BlockHash,
}

/// Additional `evm_mine` options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MineOptions {
    /// The options for mining
    Options {
        /// The timestamp the block should be mined with
        #[serde(default, with = "alloy_serde::quantity::opt")]
        timestamp: Option<u64>,
        /// If `blocks` is given, it will mine exactly blocks number of blocks, regardless of any
        /// other blocks mined or reverted during it's operation
        blocks: Option<u64>,
    },
    /// The timestamp the block should be mined with
    #[serde(with = "alloy_serde::quantity::opt")]
    Timestamp(Option<u64>),
}

impl Default for MineOptions {
    fn default() -> Self {
        Self::Options { timestamp: None, blocks: None }
    }
}

/// Represents the options used in `anvil_reorg`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgOptions {
    /// The depth of the reorg
    pub depth: u64,
    /// List of transaction requests and blocks pairs to be mined into the new chain
    pub tx_block_pairs: Vec<(TransactionData, u64)>,
}

/// Type representing txs in `ReorgOptions`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionData {
    /// Transaction request
    JSON(TransactionRequest),
    /// Raw transaction bytes
    Raw(Bytes),
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_serde_forking_deserialization() {
        // Test full forking object
        let json_data = r#"{"forking": {"jsonRpcUrl": "https://ethereumpublicnode.com","blockNumber": "18441649"}}"#;
        let forking: Forking = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            forking,
            Forking {
                json_rpc_url: Some("https://ethereumpublicnode.com".into()),
                block_number: Some(18441649)
            }
        );

        // Test forking object with only jsonRpcUrl
        let json_data = r#"{"forking": {"jsonRpcUrl": "https://ethereumpublicnode.com"}}"#;
        let forking: Forking = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            forking,
            Forking {
                json_rpc_url: Some("https://ethereumpublicnode.com".into()),
                block_number: None
            }
        );

        // Test forking object with only blockNumber
        let json_data = r#"{"forking": {"blockNumber": "18441649"}}"#;
        let forking: Forking =
            serde_json::from_str(json_data).expect("Failed to deserialize forking object");
        assert_eq!(forking, Forking { json_rpc_url: None, block_number: Some(18441649) });
    }

    #[test]
    fn test_serde_deserialize_mine_options() {
        let options = |timestamp, blocks| MineOptions::Options { timestamp, blocks };
        for (data, expected) in [
            (r#"{"timestamp": 1620000000, "blocks": 10}"#, options(Some(1620000000), Some(10))),
            (r#"{"timestamp": "0x608f3d00", "blocks": 10}"#, options(Some(1620000000), Some(10))),
            (r#"{"blocks": 5}"#, options(None, Some(5))),
            (r#"{}"#, MineOptions::default()),
            // Unknown keys are ignored, matching maps with a `timestamp` key.
            (r#"{"foo": 1}"#, MineOptions::default()),
            (r#"{"timestamp":"1620000000"}"#, options(Some(1620000000), None)),
            (r#"{"timestamp":"0x608f3d00"}"#, options(Some(1620000000), None)),
            (r#""1620000000""#, MineOptions::Timestamp(Some(1620000000))),
            (r#""0x608f3d00""#, MineOptions::Timestamp(Some(1620000000))),
        ] {
            let deserialized: MineOptions = serde_json::from_str(data).unwrap();
            assert_eq!(deserialized, expected, "{data}");
        }
    }

    #[test]
    fn test_serde_forking_rejects_invalid_tagged_options() {
        for data in [
            r#"{"forking":{"blockNumber":"invalid"}}"#,
            r#"{"forking":{"jsonRpcUrl":42}}"#,
            r#"{"forking":null}"#,
            r#"{"forking":false}"#,
            r#"{"forking":"invalid"}"#,
        ] {
            assert!(serde_json::from_str::<Forking>(data).is_err(), "{data}");
        }
    }

    #[test]
    fn test_serde_forking_untagged_and_empty_options() {
        for data in [r#"{"blockNumber":"0x11965b1"}"#, r#"{"blockNumber":18441649}"#] {
            assert_eq!(
                serde_json::from_str::<Forking>(data).unwrap(),
                Forking { json_rpc_url: None, block_number: Some(18441649) }
            );
        }
        for data in
            [r#"{}"#, r#"{"forking":{}}"#, r#"{"unknown":true}"#, r#"{"forking":{"unknown":true}}"#]
        {
            assert_eq!(serde_json::from_str::<Forking>(data).unwrap(), Forking::default());
        }
        assert_eq!(
            serde_json::from_str::<Forking>(r#"{"jsonRpcUrl":"http://localhost:8545"}"#).unwrap(),
            Forking { json_rpc_url: Some("http://localhost:8545".into()), block_number: None }
        );
        assert_eq!(
            serde_json::from_str::<Forking>(
                r#"{"forking":{"blockNumber":"0x1"},"blockNumber":"invalid","jsonRpcUrl":42}"#
            )
            .unwrap(),
            Forking { json_rpc_url: None, block_number: Some(1) }
        );
    }

    #[test]
    fn test_serde_forking_rejects_duplicate_options() {
        for data in [
            r#"{"jsonRpcUrl":"http://first:8545","jsonRpcUrl":"http://second:8545"}"#,
            r#"{"blockNumber":"0x1","blockNumber":"0x2"}"#,
            r#"{"jsonRpcUrl":null,"jsonRpcUrl":"http://second:8545"}"#,
            r#"{"blockNumber":null,"blockNumber":"0x2"}"#,
            r#"{"forking":{},"forking":{"blockNumber":"0x1"}}"#,
            r#"{"forking":{"jsonRpcUrl":"http://first:8545","jsonRpcUrl":"http://second:8545"}}"#,
            r#"{"forking":{"blockNumber":"0x1","blockNumber":"0x2"}}"#,
        ] {
            assert!(serde_json::from_str::<Forking>(data).is_err(), "{data}");
        }
    }

    #[test]
    fn test_serde_forking_tag_ignores_duplicate_root_options() {
        for data in [
            r#"{"jsonRpcUrl":false,"jsonRpcUrl":42,"blockNumber":false,"forking":{"blockNumber":"0x1"}}"#,
            r#"{"forking":{"blockNumber":"0x1"},"blockNumber":false,"blockNumber":"invalid","jsonRpcUrl":42}"#,
        ] {
            assert_eq!(
                serde_json::from_str::<Forking>(data).unwrap(),
                Forking { json_rpc_url: None, block_number: Some(1) },
                "{data}"
            );
        }
    }
}
