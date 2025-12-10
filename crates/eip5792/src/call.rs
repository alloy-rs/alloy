use alloy_primitives::{map::HashMap, Address, Bytes, ChainId, U256};

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
#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, Bytes, ChainId, U256};

    #[test]
    fn test_serialization_deserialization() {
        let sample_request = SendCallsRequest {
            version: "1.0".to_string(),
            from: Address::default(),
            calls: vec![
                CallParams {
                    to: Some(Address::default()),
                    data: Some(Bytes::from("d46e8dd67c5d32be8d46e8dd67c5d32be8058bb8eb970870f072445675058bb8eb970870f072445675"
                    )),
                    value: Some(U256::from(0x9184e72au64)),
                    chain_id: Some(ChainId::from(1u64)),
                },
                CallParams {
                    to: Some(Address::default()),
                    data: Some(Bytes::from(
                       "fbadbaf01"),
                    ),
                    value: Some(U256::from(0x182183u64)),
                    chain_id: Some(ChainId::from(1u64)),
                },
            ],
            capabilities: None,
        };

        let serialized = serde_json::to_string(&sample_request).unwrap();
        let deserialized: SendCallsRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(sample_request, deserialized);
    }
}
