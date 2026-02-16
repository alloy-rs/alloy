//! Types for the validator duty endpoints.
//!
//! See <https://ethereum.github.io/beacon-APIs/#/Validator>

use crate::BlsPublicKey;
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the [`/eth/v1/validator/duties/attester/{epoch}`](https://ethereum.github.io/beacon-APIs/#/Validator/getAttesterDuties) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttesterDutiesResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// The dependent root for the response.
    pub dependent_root: B256,
    /// The list of attester duties.
    pub data: Vec<AttesterDuty>,
}

/// A single attester duty entry.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttesterDuty {
    /// The BLS public key of the validator assigned to attest.
    pub pubkey: BlsPublicKey,
    /// The index of the validator in the validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
    /// The committee index.
    #[serde_as(as = "DisplayFromStr")]
    pub committee_index: u64,
    /// The total number of validators in the committee.
    #[serde_as(as = "DisplayFromStr")]
    pub committee_length: u64,
    /// The number of committees at the slot.
    #[serde_as(as = "DisplayFromStr")]
    pub committees_at_slot: u64,
    /// The index of the validator within the committee.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_committee_index: u64,
    /// The slot at which the validator must attest.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
}

/// Response from the [`/eth/v1/validator/duties/sync/{epoch}`](https://ethereum.github.io/beacon-APIs/#/Validator/getSyncCommitteeDuties) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCommitteeDutiesResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// The list of sync committee duties.
    pub data: Vec<SyncCommitteeDuty>,
}

/// A single sync committee duty entry.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCommitteeDuty {
    /// The BLS public key of the validator.
    pub pubkey: BlsPublicKey,
    /// The index of the validator in the validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
    /// The indices of the validator in the sync committee.
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub validator_sync_committee_indices: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_attester_duties_response() {
        let s = r#"{
            "execution_optimistic": false,
            "dependent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "data": [
                {
                    "pubkey": "0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a",
                    "validator_index": "1",
                    "committee_index": "1",
                    "committee_length": "128",
                    "committees_at_slot": "2",
                    "validator_committee_index": "25",
                    "slot": "1"
                }
            ]
        }"#;
        let resp: AttesterDutiesResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].validator_index, 1);
        assert_eq!(resp.data[0].committee_index, 1);
        assert_eq!(resp.data[0].committee_length, 128);
        assert_eq!(resp.data[0].committees_at_slot, 2);
        assert_eq!(resp.data[0].validator_committee_index, 25);
        assert_eq!(resp.data[0].slot, 1);

        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: AttesterDutiesResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(resp, deserialized);
    }

    #[test]
    fn serde_sync_committee_duties_response() {
        let s = r#"{
            "execution_optimistic": false,
            "data": [
                {
                    "pubkey": "0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a",
                    "validator_index": "1",
                    "validator_sync_committee_indices": ["0", "5", "10"]
                }
            ]
        }"#;
        let resp: SyncCommitteeDutiesResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].validator_index, 1);
        assert_eq!(resp.data[0].validator_sync_committee_indices, vec![0, 5, 10]);

        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: SyncCommitteeDutiesResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(resp, deserialized);
    }
}
