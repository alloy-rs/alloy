//! Types for the beacon state validators endpoints.
//!
//! See <https://ethereum.github.io/beacon-APIs/#/Beacon/getStateValidators>

use crate::BlsPublicKey;
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the [`/eth/v1/beacon/states/{state_id}/validators`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateValidators) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorsResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The list of validator entries.
    pub data: Vec<ValidatorData>,
}

/// Response from the [`/eth/v1/beacon/states/{state_id}/validators/{validator_id}`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateValidator) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The validator entry.
    pub data: ValidatorData,
}

/// A single validator entry in the state validators response.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorData {
    /// The index of the validator in the validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub index: u64,
    /// The balance of the validator in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub balance: u64,
    /// The status of the validator (e.g. `"active_ongoing"`, `"pending_initialized"`).
    pub status: String,
    /// The validator details.
    pub validator: Validator,
}

/// Validator details from the beacon state.
///
/// See <https://github.com/ethereum/consensus-specs/blob/v1.5.0/specs/phase0/beacon-chain.md#validator>
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validator {
    /// The BLS public key of the validator.
    pub pubkey: BlsPublicKey,
    /// The withdrawal credentials.
    pub withdrawal_credentials: B256,
    /// The effective balance of the validator in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub effective_balance: u64,
    /// Whether the validator has been slashed.
    pub slashed: bool,
    /// The epoch at which the validator becomes eligible for activation.
    #[serde_as(as = "DisplayFromStr")]
    pub activation_eligibility_epoch: u64,
    /// The epoch at which the validator was activated.
    #[serde_as(as = "DisplayFromStr")]
    pub activation_epoch: u64,
    /// The epoch at which the validator will exit.
    #[serde_as(as = "DisplayFromStr")]
    pub exit_epoch: u64,
    /// The earliest epoch at which the validator can withdraw.
    #[serde_as(as = "DisplayFromStr")]
    pub withdrawable_epoch: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_validator_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "index": "1",
                "balance": "32000000000",
                "status": "active_ongoing",
                "validator": {
                    "pubkey": "0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a",
                    "withdrawal_credentials": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "effective_balance": "32000000000",
                    "slashed": false,
                    "activation_eligibility_epoch": "0",
                    "activation_epoch": "0",
                    "exit_epoch": "18446744073709551615",
                    "withdrawable_epoch": "18446744073709551615"
                }
            }
        }"#;
        let resp: ValidatorResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.index, 1);
        assert_eq!(resp.data.balance, 32000000000);
        assert_eq!(resp.data.status, "active_ongoing");
        assert!(!resp.data.validator.slashed);
    }

    #[test]
    fn serde_validators_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": [
                {
                    "index": "1",
                    "balance": "32000000000",
                    "status": "active_ongoing",
                    "validator": {
                        "pubkey": "0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a",
                        "withdrawal_credentials": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                        "effective_balance": "32000000000",
                        "slashed": false,
                        "activation_eligibility_epoch": "0",
                        "activation_epoch": "0",
                        "exit_epoch": "18446744073709551615",
                        "withdrawable_epoch": "18446744073709551615"
                    }
                }
            ]
        }"#;
        let resp: ValidatorsResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 1);
    }
}
