//! EIP-8141 frame request type.

use alloy_eips::eip8141::{Frame, FrameAddress, FrameLimits, FrameMode};
use alloy_primitives::{Bytes, U256};

/// An [EIP-8141] frame as supplied in a transaction request.
///
/// Unlike [`Frame`], both gas limits are optional so that a node can fill them in. An omitted
/// limit is distinct from an explicit zero limit, which is kept as is. The JSON representation is
/// flat (`executionGas` and `stateGas` instead of a nested `limits` object) and omits the target
/// when it resolves to the transaction sender.
///
/// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct FrameRequest {
    /// The frame execution mode.
    pub mode: FrameMode,
    /// Frame flags. Bits 0-1 encode approval scope, bit 2 encodes atomic batching.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub flags: u8,
    /// Target account. An empty address resolves to the transaction sender.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_sender"))]
    pub target: FrameAddress,
    /// Maximum execution gas available to the frame, `None` if it still needs to be filled.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub execution_gas: Option<u64>,
    /// Maximum state gas available to the frame, `None` if it still needs to be filled.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub state_gas: Option<u64>,
    /// Wei value transferred by the frame.
    #[cfg_attr(feature = "serde", serde(default))]
    pub value: U256,
    /// Calldata provided to the top-level frame call.
    #[cfg_attr(feature = "serde", serde(default))]
    pub data: Bytes,
}

impl From<Frame> for FrameRequest {
    fn from(frame: Frame) -> Self {
        Self {
            mode: frame.mode,
            flags: frame.flags,
            target: frame.target,
            execution_gas: Some(frame.limits.execution),
            state_gas: Some(frame.limits.state),
            value: frame.value,
            data: frame.data,
        }
    }
}

impl TryFrom<FrameRequest> for Frame {
    type Error = &'static str;

    /// Converts the request into a [`Frame`], failing if either gas limit is omitted.
    fn try_from(frame: FrameRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            mode: frame.mode,
            flags: frame.flags,
            target: frame.target,
            limits: FrameLimits {
                execution: frame.execution_gas.ok_or("missing frame executionGas")?,
                state: frame.state_gas.ok_or("missing frame stateGas")?,
            },
            value: frame.value,
            data: frame.data,
        })
    }
}

#[cfg(feature = "serde")]
const fn is_sender(target: &FrameAddress) -> bool {
    target.is_empty()
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use alloy_primitives::{bytes, Address};
    use serde_json::json;

    #[test]
    fn omitted_limits_stay_distinct_from_zero() {
        let omitted = serde_json::from_value::<FrameRequest>(json!({"mode": "0x1"})).unwrap();
        assert_eq!(omitted, FrameRequest { mode: FrameMode::Verify, ..Default::default() });
        assert_eq!(
            serde_json::to_value(&omitted).unwrap(),
            json!({"mode": "0x1", "flags": "0x0", "value": "0x0", "data": "0x"})
        );
        assert!(Frame::try_from(omitted).is_err());

        let zero = json!({
            "mode": "0x1",
            "flags": "0x0",
            "executionGas": "0x0",
            "stateGas": "0x0",
            "value": "0x0",
            "data": "0x",
        });
        let request = serde_json::from_value::<FrameRequest>(zero.clone()).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), zero);
        assert_eq!(
            Frame::try_from(request).unwrap().limits,
            FrameLimits { execution: 0, state: 0 }
        );
    }

    #[test]
    fn frame_roundtrips_through_flat_request() {
        let frame = Frame {
            mode: FrameMode::Sender,
            flags: 4,
            target: Address::repeat_byte(0x11).into(),
            limits: FrameLimits { execution: 0x123, state: 0x45 },
            value: U256::from(1),
            data: bytes!("abcd"),
        };
        let value = serde_json::to_value(FrameRequest::from(frame.clone())).unwrap();
        assert_eq!(
            value,
            json!({
                "mode": "0x2",
                "flags": "0x4",
                "target": "0x1111111111111111111111111111111111111111",
                "executionGas": "0x123",
                "stateGas": "0x45",
                "value": "0x1",
                "data": "0xabcd",
            })
        );
        let request = serde_json::from_value::<FrameRequest>(value).unwrap();
        assert_eq!(Frame::try_from(request).unwrap(), frame);

        let sender_target =
            serde_json::from_value::<FrameRequest>(json!({"mode": "0x0", "target": null})).unwrap();
        assert_eq!(sender_target.target, FrameAddress::Empty);

        // The nested limits of the consensus frame representation are not accepted.
        let nested = json!({"mode": "0x0", "limits": {"execution": "0x0", "state": "0x0"}});
        assert!(serde_json::from_value::<FrameRequest>(nested).is_err());
    }
}
