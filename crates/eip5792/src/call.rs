use alloy_primitives::{map::HashMap, Address, Bytes, ChainId, U256};

/// Legacy sequencer-specific request for submitting a batch of wallet calls.
///
/// This shape predates and is incompatible with the Final [EIP-5792] `wallet_sendCalls` request:
/// it requires `from`, stores chain IDs on individual calls, and omits the final schema's
/// top-level `id`, `chainId`, and `atomicRequired` fields.
///
/// [EIP-5792]: https://eips.ethereum.org/EIPS/eip-5792
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendCallsRequest {
    /// RPC version
    pub version: String,
    /// Sender's address
    pub from: Address,
    /// A batch of calls to be submitted
    pub calls: Vec<CallParams>,
    /// Opaque capability requests keyed by capability name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<HashMap<String, serde_json::Value>>,
}

/// Call parameters for the legacy sequencer-specific request shape.
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
    /// ID of the target chain.
    ///
    /// Final EIP-5792 instead places one required `chainId` on the enclosing request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<ChainId>,
}
