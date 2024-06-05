//! Types for the `anvil` api

use alloy_primitives::{TxHash, B256, U256, U64};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Specification IDs and their activation block.
///
/// Information was obtained from the [Ethereum Execution Specifications](https://github.com/ethereum/execution-specs)
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SpecId {
    /// Frontier: 0
    FRONTIER = 0,
    /// Frontier Thawing: 200000
    FRONTIER_THAWING = 1,
    /// Homestead: 1150000
    HOMESTEAD = 2,
    /// DAO Fork: 1920000
    DAO_FORK = 3,
    /// Tangerine Whistle: 2463000
    TANGERINE = 4,
    /// Spurious Dragon: 2675000
    SPURIOUS_DRAGON = 5,
    /// Byzantium: 4370000
    BYZANTIUM = 6,
    /// Constantinople: 7280000 is overwritten with PETERSBURG
    CONSTANTINOPLE = 7,
    /// Petersburg: 7280000
    PETERSBURG = 8,
    /// Istanbul: 9069000
    ISTANBUL = 9,
    /// Muir Glacier: 9200000
    MUIR_GLACIER = 10,
    /// Berlin: 12244000
    BERLIN = 11,
    /// London: 12965000
    LONDON = 12,
    /// Arrow Glacier: 13773000
    ARROW_GLACIER = 13,
    /// Gray Glacier: 15050000
    GRAY_GLACIER = 14,
    /// Paris/Merge: 15537394 (TTD: 58750000000000000000000)
    MERGE = 15,
    /// Shanghai: 17034870 (Timestamp: 1681338455)
    SHANGHAI = 16,
    /// Cancun: 19426587 (Timestamp: 1710338135)
    CANCUN = 17,
    /// Praque: TBD
    PRAGUE = 18,
    /// -1
    #[default]
    LATEST = u8::MAX,
}

/// Additional `evm_mine` options
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum EvmMineOptions {
    Options {
        #[cfg_attr(feature = "serde", serde(with = "alloy_serde::num::u64_opt_via_ruint"))]
        timestamp: Option<u64>,
        // If `blocks` is given, it will mine exactly blocks number of blocks, regardless of any
        // other blocks mined or reverted during it's operation
        blocks: Option<u64>,
    },
    /// The timestamp the block should be mined with
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::num::u64_opt_via_ruint"))]
    Timestamp(Option<u64>),
}

impl Default for EvmMineOptions {
    fn default() -> Self {
        EvmMineOptions::Options { timestamp: None, blocks: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NodeEnvironment {
    pub base_fee: u128,
    pub chain_id: u64,
    pub gas_limit: u128,
    pub gas_price: u128,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NodeForkConfig {
    pub fork_url: Option<String>,
    pub fork_block_number: Option<u64>,
    pub fork_retry_backoff: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NodeInfo {
    pub current_block_number: U64,
    pub current_block_timestamp: u64,
    pub current_block_hash: B256,
    pub hard_fork: SpecId,
    pub transaction_order: String,
    pub environment: NodeEnvironment,
    pub fork_config: NodeForkConfig,
}

/// Anvil equivalent of `hardhat_metadata`.
/// Metadata about the current Anvil instance.
/// See <https://hardhat.org/hardhat-network/docs/reference#hardhat_metadata>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AnvilMetadata {
    pub client_version: &'static str,
    pub chain_id: u64,
    pub instance_id: B256,
    pub latest_block_number: u64,
    pub latest_block_hash: B256,
    pub forked_network: Option<ForkedNetwork>,
    pub snapshots: BTreeMap<U256, (u64, B256)>,
}

/// Information about the forked network.
/// See <https://hardhat.org/hardhat-network/docs/reference#hardhat_metadata>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ForkedNetwork {
    pub chain_id: u64,
    pub fork_block_number: u64,
    pub fork_block_hash: TxHash,
}

/// Represents the params to set forking which can take various forms
///  - untagged
///  - tagged `forking`
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Forking {
    pub json_rpc_url: Option<String>,
    pub block_number: Option<u64>,
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Forking {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ForkOpts {
            pub json_rpc_url: Option<String>,
            #[serde(default, with = "alloy_serde::num::u64_opt_via_ruint")]
            pub block_number: Option<u64>,
        }

        #[derive(Deserialize)]
        struct Tagged {
            forking: ForkOpts,
        }
        #[derive(Deserialize)]
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
