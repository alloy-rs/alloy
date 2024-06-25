use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};

/// Bundle of transactions for `eth_sendBundle`
///
/// Note: this is for `eth_sendBundle` and not `mev_sendBundle`
///
/// <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendbundle>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EthSendBundle {
    /// A list of hex-encoded signed transactions
    pub(crate) txs: Vec<Bytes>,
    /// hex-encoded block number for which this bundle is valid
    #[serde(with = "alloy_serde::quantity")]
    pub(crate) block_number: u64,
    /// unix timestamp when this bundle becomes active
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub(crate) min_timestamp: Option<u64>,
    /// unix timestamp how long this bundle stays valid
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub(crate) max_timestamp: Option<u64>,
    /// list of hashes of possibly reverting txs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) reverting_tx_hashes: Vec<B256>,
    /// UUID that can be used to cancel/replace this bundle
    #[serde(default, rename = "replacementUuid", skip_serializing_if = "Option::is_none")]
    pub(crate) replacement_uuid: Option<String>,
}

/// Response from the matchmaker after sending a bundle.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EthBundleHash {
    /// Hash of the bundle bodies.
    pub(crate) bundle_hash: B256,
}

/// Response from the matchmaker after sending a bundle.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendBundleResponse {
    /// Hash of the bundle bodies.
    pub bundle_hash: B256,
}
