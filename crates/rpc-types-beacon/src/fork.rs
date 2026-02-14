//! Types for the beacon state fork endpoint.
//!
//! See <https://ethereum.github.io/beacon-APIs/#/Beacon/getStateFork>

use alloy_primitives::FixedBytes;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the `eth/v1/beacon/states/{state_id}/fork` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The fork data.
    pub data: Fork,
}

/// The [`Fork`](https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#fork) object from the CL spec.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fork {
    /// Previous fork version.
    pub previous_version: FixedBytes<4>,
    /// Current fork version.
    pub current_version: FixedBytes<4>,
    /// The epoch at which the fork occurred.
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_fork_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "previous_version": "0x00000000",
                "current_version": "0x04000000",
                "epoch": "0"
            }
        }"#;
        let resp: ForkResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.epoch, 0);
        assert_eq!(resp.data.current_version, FixedBytes::from([0x04, 0x00, 0x00, 0x00]));
    }
}
