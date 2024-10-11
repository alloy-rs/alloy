use std::vec::Vec;
use alloy_primitives::{map::HashMap, Address, Bytes, ChainId, U256};

/// Type of permisssion values
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PermissionValue {
    /// Permission of boolean type
    Bool(bool),
    /// Array of permission values of String type 
    Array(Vec<String>),
    /// Map of rpc call's capabilities
    Dictionary(HashMap<String, String>),
    /// Value of String type
    Text(String)
}

/// Request that a wallet submits a batch of calls 
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SendCallsRequest {
    /// RPC version
    pub version: String,
    /// Sender's address
    pub from: Address,
    /// A batch of calls to be submitted
    pub calls: Vec<CallParams>,
    /// Enabled permissions per chain
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub capabilities: Option<HashMap<String, PermissionValue>>
}

/// Call parameters
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CallParams {
    /// Recepient address
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub to: Option<Address>,
    /// Tx data field
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub data: Option<Bytes>,
    /// Transfered value
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub value: Option<U256>,
    /// Id of target chain 
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub chain_id: Option<ChainId>
}

impl SendCallsRequest {
    /// Creates new [SendCallsRequest]
    pub const fn new(version: String, from: Address, calls: Vec<CallParams>, capabilities: Option<HashMap<String, PermissionValue>>) -> Self {
        Self {version, from, calls, capabilities}
    }
}

impl CallParams {
    /// Creates new [CallParams]
    pub const fn new(to: Option<Address>, data: Option<Bytes>, value: Option<U256>, chain_id: Option<ChainId>) -> Self {
        Self {to, data, value, chain_id}
    }
}

/// Response type for RPC call.
/// 
/// See [EIP-5792](https://eips.ethereum.org/EIPS/eip-5792#wallet_getcapabilities)
pub type GetCapabilitiesResult = HashMap<ChainId, HashMap<String, PermissionValue>>;

/// Response type of wallet_sendCalls
pub type SendCallsResult = String;

/// Request params of RPC call wallet_getCapabilities 
pub type GetCapabilitiesParams = Vec<Address>;

/// Alias for wallet_sendCalls params
pub type SendCallsParams = CallParams;
 
