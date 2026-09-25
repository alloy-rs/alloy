//! Frame transaction request fields.

use alloy_eips::eip8141::{Frame, FrameAddress, FrameLimits, FrameMode};
use alloy_primitives::{Bytes, U256};

/// A frame whose omitted gas limits must be filled before consensus encoding.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct FrameRequest {
    /// Frame execution mode.
    pub mode: FrameMode,
    /// Frame flags.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub flags: u8,
    /// Target address; omission resolves to the sender.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_sender"))]
    pub target: FrameAddress,
    /// Execution gas limit. Zero is an explicit limit.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            with = "alloy_serde::quantity::opt",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub execution_gas: Option<u64>,
    /// State gas limit. Zero is an explicit limit.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            with = "alloy_serde::quantity::opt",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub state_gas: Option<u64>,
    /// Value transferred by the frame.
    #[cfg_attr(feature = "serde", serde(default))]
    pub value: U256,
    /// Frame input data.
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

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn omitted_limits_stay_distinct_from_zero() {
        let omitted: FrameRequest = serde_json::from_value(json!({"mode":"0x1"})).unwrap();
        assert_eq!(omitted.execution_gas, None);
        assert_eq!(omitted.state_gas, None);
        assert!(serde_json::to_value(&omitted).unwrap().get("executionGas").is_none());
        assert!(Frame::try_from(omitted).is_err());
        let zero: FrameRequest =
            serde_json::from_value(json!({"mode":"0x1", "executionGas":"0x0", "stateGas":"0x0"}))
                .unwrap();
        assert_eq!(
            Frame::try_from(zero.clone()).unwrap().limits,
            FrameLimits { execution: 0, state: 0 }
        );
        assert_eq!(serde_json::to_value(zero).unwrap()["executionGas"], "0x0");
    }
}

#[cfg(feature = "serde")]
const fn is_sender(target: &FrameAddress) -> bool {
    target.is_empty()
}
