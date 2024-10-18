use alloy_primitives::{map::HashMap, Address, Bytes, ChainId, U256};
use std::vec::Vec;

/// Request that a wallet submits a batch of calls in `wallet_sendCalls`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendCallsRequest {
    /// RPC version
    pub version: String,
    /// Sender's address
    pub from: Address,
    /// A batch of calls to be submitted
    pub calls: Vec<CallParams>,
    /// Enabled permissions per chain
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<HashMap<String, serde_json::Value>>,
}

/// Call parameters for `wallet_sendCalls`
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallParams {
    /// Recipient address
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// Tx data field
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
    /// Transferred value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Id of target chain
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<ChainId>,
}
