//! Types for the beacon state endpoints.
//!
//! See <https://ethereum.github.io/beacon-APIs/#/Beacon>

use crate::block::Checkpoint;
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the [`/eth/v1/beacon/states/{state_id}/committees`](https://ethereum.github.io/beacon-APIs/#/Beacon/getEpochCommittees) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitteesResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The list of committees.
    pub data: Vec<Committee>,
}

/// A single committee entry.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committee {
    /// The committee index at a slot.
    #[serde_as(as = "DisplayFromStr")]
    pub index: u64,
    /// The slot at which the committee was assigned.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// List of validator indices assigned to this committee.
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub validators: Vec<u64>,
}

/// Response from the [`/eth/v1/beacon/states/{state_id}/sync_committees`](https://ethereum.github.io/beacon-APIs/#/Beacon/getEpochSyncCommittees) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCommitteesResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The sync committee data.
    pub data: SyncCommittee,
}

/// Sync committee validator indices.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCommittee {
    /// All validators in the current sync committee.
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub validators: Vec<u64>,
    /// Subcommittee slices of the sync committee.
    #[serde_as(as = "Vec<Vec<DisplayFromStr>>")]
    pub validator_aggregates: Vec<Vec<u64>>,
}

/// Response from the [`/eth/v1/beacon/states/{state_id}/finality_checkpoints`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateFinalityCheckpoints) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityCheckpointsResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The finality checkpoint data.
    pub data: FinalityCheckpoints,
}

/// Finality checkpoints for the beacon state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityCheckpoints {
    /// The previous justified checkpoint.
    pub previous_justified: Checkpoint,
    /// The current justified checkpoint.
    pub current_justified: Checkpoint,
    /// The finalized checkpoint.
    pub finalized: Checkpoint,
}

/// Response from the [`/eth/v1/beacon/states/{state_id}/validator_balances`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateValidatorBalances) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorBalancesResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The list of validator balances.
    pub data: Vec<ValidatorBalance>,
}

/// A single validator balance entry.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorBalance {
    /// The index of the validator in the validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub index: u64,
    /// The balance of the validator in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub balance: u64,
}

/// Response from the [`/eth/v1/beacon/states/{state_id}/randao`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateRandao) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandaoResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The RANDAO mix data.
    pub data: RandaoData,
}

/// The RANDAO mix for the requested state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandaoData {
    /// The RANDAO mix value.
    pub randao: B256,
}

/// Response from the [`/eth/v1/beacon/states/{state_id}/root`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateRoot) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRootResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references finalized history.
    #[serde(default)]
    pub finalized: bool,
    /// The state root data.
    pub data: StateRootData,
}

/// The state root for the requested state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRootData {
    /// The state root hash.
    pub root: B256,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_committees_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": [
                {
                    "index": "1",
                    "slot": "2",
                    "validators": ["0", "1", "2"]
                }
            ]
        }"#;
        let resp: CommitteesResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].index, 1);
        assert_eq!(resp.data[0].slot, 2);
        assert_eq!(resp.data[0].validators, vec![0, 1, 2]);
        assert!(resp.finalized);
        assert!(!resp.execution_optimistic);

        let roundtrip: CommitteesResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, roundtrip);
    }

    #[test]
    fn serde_sync_committees_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "validators": ["0", "1", "2", "3"],
                "validator_aggregates": [
                    ["0", "1"],
                    ["2", "3"]
                ]
            }
        }"#;
        let resp: SyncCommitteesResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.validators, vec![0, 1, 2, 3]);
        assert_eq!(resp.data.validator_aggregates, vec![vec![0, 1], vec![2, 3]]);

        let roundtrip: SyncCommitteesResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, roundtrip);
    }

    #[test]
    fn serde_finality_checkpoints_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "previous_justified": {
                    "epoch": "10",
                    "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                },
                "current_justified": {
                    "epoch": "11",
                    "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                },
                "finalized": {
                    "epoch": "9",
                    "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                }
            }
        }"#;
        let resp: FinalityCheckpointsResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.previous_justified.epoch, 10);
        assert_eq!(resp.data.current_justified.epoch, 11);
        assert_eq!(resp.data.finalized.epoch, 9);

        let roundtrip: FinalityCheckpointsResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, roundtrip);
    }

    #[test]
    fn serde_validator_balances_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": [
                {
                    "index": "1",
                    "balance": "32000000000"
                }
            ]
        }"#;
        let resp: ValidatorBalancesResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].index, 1);
        assert_eq!(resp.data[0].balance, 32000000000);

        let roundtrip: ValidatorBalancesResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, roundtrip);
    }

    #[test]
    fn serde_randao_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "randao": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
            }
        }"#;
        let resp: RandaoResponse = serde_json::from_str(s).unwrap();
        assert_eq!(
            resp.data.randao,
            "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                .parse::<B256>()
                .unwrap()
        );

        let roundtrip: RandaoResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, roundtrip);
    }

    #[test]
    fn serde_state_root_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
            }
        }"#;
        let resp: StateRootResponse = serde_json::from_str(s).unwrap();
        assert_eq!(
            resp.data.root,
            "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                .parse::<B256>()
                .unwrap()
        );

        let roundtrip: StateRootResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, roundtrip);
    }

    #[test]
    fn serde_committees_defaults() {
        let s = r#"{
            "data": [
                {
                    "index": "0",
                    "slot": "0",
                    "validators": []
                }
            ]
        }"#;
        let resp: CommitteesResponse = serde_json::from_str(s).unwrap();
        assert!(!resp.execution_optimistic);
        assert!(!resp.finalized);
    }
}
