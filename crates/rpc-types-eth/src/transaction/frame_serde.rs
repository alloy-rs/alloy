//! JSON adapters for frame transaction requests.

use alloc::vec::Vec;
use alloy_eips::eip8141::{
    Frame, FrameAddress, FrameLimits, FrameMode, FrameSignature, SignatureMessage, SignatureScheme,
};
use alloy_primitives::{Bytes, U256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FrameRequest {
    mode: FrameMode,
    #[serde(default, with = "alloy_serde::quantity")]
    flags: u8,
    #[serde(default, skip_serializing_if = "is_sender")]
    target: FrameAddress,
    #[serde(default, with = "alloy_serde::quantity")]
    execution_gas: u64,
    #[serde(default, with = "alloy_serde::quantity")]
    state_gas: u64,
    #[serde(default)]
    value: U256,
    #[serde(default)]
    data: Bytes,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureRequest {
    scheme: SignatureScheme,
    #[serde(default, skip_serializing_if = "is_sender")]
    signer: FrameAddress,
    #[serde(default)]
    msg: SignatureMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<Bytes>,
}

fn is_sender(address: &FrameAddress) -> bool {
    address.address().is_none()
}

pub(super) mod frames {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(
        value: &Option<Vec<Frame>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .as_ref()
            .map(|frames| {
                frames
                    .iter()
                    .map(|frame| FrameRequest {
                        mode: frame.mode,
                        flags: frame.flags,
                        target: frame.target,
                        execution_gas: frame.limits.execution,
                        state_gas: frame.limits.state,
                        value: frame.value,
                        data: frame.data.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<Frame>>, D::Error> {
        Option::<Vec<FrameRequest>>::deserialize(deserializer).map(|frames| {
            frames.map(|frames| {
                frames
                    .into_iter()
                    .map(|frame| Frame {
                        mode: frame.mode,
                        flags: frame.flags,
                        target: frame.target,
                        limits: FrameLimits {
                            execution: frame.execution_gas,
                            state: frame.state_gas,
                        },
                        value: frame.value,
                        data: frame.data,
                    })
                    .collect()
            })
        })
    }
}

pub(super) mod signatures {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(
        value: &Option<Vec<FrameSignature>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .as_ref()
            .map(|signatures| {
                signatures
                    .iter()
                    .map(|signature| SignatureRequest {
                        scheme: signature.scheme,
                        signer: signature.signer,
                        msg: signature.msg,
                        signature: if signature.signature.is_empty()
                            && u8::from(signature.scheme) != 0
                        {
                            None
                        } else {
                            Some(signature.signature.clone())
                        },
                    })
                    .collect::<Vec<_>>()
            })
            .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<FrameSignature>>, D::Error> {
        Option::<Vec<SignatureRequest>>::deserialize(deserializer).map(|signatures| {
            signatures.map(|signatures| {
                signatures
                    .into_iter()
                    .map(|signature| FrameSignature {
                        scheme: signature.scheme,
                        signer: signature.signer,
                        msg: signature.msg,
                        signature: signature.signature.unwrap_or_default(),
                    })
                    .collect()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transaction, TransactionRequest};
    use alloy_consensus::{transaction::Recovered, TxEip8141, TxEnvelope};
    use alloy_primitives::Address;
    use serde_json::json;

    #[test]
    fn request_accepts_flat_frames_and_scheme_only_placeholders() {
        let request: TransactionRequest = serde_json::from_value(json!({
            "type": "0x6", "from": Address::repeat_byte(0x11),
            "frames": [{"mode":"0x1", "executionGas":"0x123", "stateGas":"0x45"}],
            "signatures": [{"scheme":"0x1"}]
        }))
        .unwrap();
        let frame = &request.frames.as_ref().unwrap()[0];
        assert_eq!(frame.limits, FrameLimits { execution: 0x123, state: 0x45 });
        assert!(frame.target.address().is_none());
        let signature = &request.signatures.as_ref().unwrap()[0];
        assert!(signature.signature.is_empty());
        assert!(signature.signer.address().is_none());
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["frames"][0]["executionGas"], "0x123");
        assert!(value["frames"][0].get("limits").is_none());
        assert!(value["signatures"][0].get("signature").is_none());
        assert!(value["signatures"][0].get("signer").is_none());
    }

    #[test]
    fn frame_transaction_response_round_trips_with_one_from_field() {
        let sender = Address::repeat_byte(0x11);
        let tx = TxEip8141 { sender, frames: vec![Frame::default()], ..Default::default() };
        let rpc = Transaction {
            inner: Recovered::new_unchecked(
                TxEnvelope::Eip8141(alloy_primitives::Sealed::new_unchecked(
                    tx,
                    Default::default(),
                )),
                sender,
            ),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            effective_gas_price: None,
            block_timestamp: None,
        };
        let encoded = serde_json::to_string(&rpc).unwrap();
        assert_eq!(encoded.matches("\"from\":").count(), 1);
        assert!(!encoded.contains("\"sender\":"));
        let decoded: Transaction = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, rpc);
    }
}
