#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![allow(missing_docs)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use alloy_primitives::{TxHash, B256, U256};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Represents the params to set forking which can take various forms
///  - untagged
///  - tagged `forking`
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forking {
    pub json_rpc_url: Option<String>,
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
            #[serde(default, with = "alloy_serde::u64_hex_or_decimal_opt")]
            block_number: Option<u64>,
        }

        #[derive(serde::Deserialize)]
        struct Tagged {
            forking: ForkOpts,
        }
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum ForkingVariants {
            Tagged(Tagged),
            Fork(ForkOpts),
        }
        let f = match ForkingVariants::deserialize(deserializer)? {
            ForkingVariants::Fork(ForkOpts { json_rpc_url, block_number }) => {
                Forking { json_rpc_url, block_number }
            }
            ForkingVariants::Tagged(f) => Forking {
                json_rpc_url: f.forking.json_rpc_url,
                block_number: f.forking.block_number,
            },
        };
        Ok(f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    #[serde(with = "alloy_serde::u64_hex")]
    pub current_block_number: u64,
    pub current_block_timestamp: u64,
    pub current_block_hash: B256,
    pub hard_fork: String,
    pub transaction_order: String,
    pub environment: NodeEnvironment,
    pub fork_config: NodeForkConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEnvironment {
    pub base_fee: U256,
    pub chain_id: u64,
    pub gas_limit: U256,
    pub gas_price: U256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeForkConfig {
    pub fork_url: Option<String>,
    pub fork_block_number: Option<u64>,
    pub fork_retry_backoff: Option<u128>,
}

/// Anvil equivalent of `hardhat_metadata`.
/// Metadata about the current Anvil instance.
/// See <https://hardhat.org/hardhat-network/docs/reference#hardhat_metadata>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub client_version: String,
    pub chain_id: u64,
    pub instance_id: B256,
    pub latest_block_number: u64,
    pub latest_block_hash: B256,
    pub forked_network: Option<ForkedNetwork>,
    pub snapshots: BTreeMap<U256, (u64, B256)>,
}

/// Information about the forked network.
/// See <https://hardhat.org/hardhat-network/docs/reference#hardhat_metadata>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkedNetwork {
    pub chain_id: u64,
    pub fork_block_number: u64,
    pub fork_block_hash: TxHash,
}

/// Additional `evm_mine` options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MineOptions {
    Options {
        #[serde(with = "alloy_serde::u64_hex_or_decimal_opt")]
        timestamp: Option<u64>,
        // If `blocks` is given, it will mine exactly blocks number of blocks, regardless of any
        // other blocks mined or reverted during it's operation
        blocks: Option<u64>,
    },
    /// The timestamp the block should be mined with
    #[serde(with = "alloy_serde::u64_hex_or_decimal_opt")]
    Timestamp(Option<u64>),
}

impl Default for MineOptions {
    fn default() -> Self {
        MineOptions::Options { timestamp: None, blocks: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_forking() {
        let s = r#"{"forking": {"jsonRpcUrl": "https://ethereumpublicnode.com",
        "blockNumber": "18441649"
      }
    }"#;
        let f: Forking = serde_json::from_str(s).unwrap();
        assert_eq!(
            f,
            Forking {
                json_rpc_url: Some("https://ethereumpublicnode.com".into()),
                block_number: Some(18441649)
            }
        );
    }
}
