//! Types for the beacon rewards endpoints.

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the [`/eth/v1/beacon/rewards/blocks/{block_id}`](https://ethereum.github.io/beacon-APIs/#/Rewards/getBlockRewards) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRewardsResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references the finalized history of the chain.
    #[serde(default)]
    pub finalized: bool,
    /// Block rewards data.
    pub data: BlockRewards,
}

/// Rewards info for a single block.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRewards {
    /// Proposer index of the block.
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_index: u64,
    /// Total block reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub total: u64,
    /// Attestation reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub attestations: u64,
    /// Sync aggregate reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub sync_aggregate: u64,
    /// Proposer slashings reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_slashings: u64,
    /// Attester slashings reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub attester_slashings: u64,
}

/// Response from the [`/eth/v1/beacon/rewards/sync_committee/{block_id}`](https://ethereum.github.io/beacon-APIs/#/Rewards/getSyncCommitteeRewards) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCommitteeRewardsResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references the finalized history of the chain.
    #[serde(default)]
    pub finalized: bool,
    /// List of validator sync committee rewards.
    pub data: Vec<SyncCommitteeReward>,
}

/// Reward for a single validator from sync committee participation.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCommitteeReward {
    /// Validator index.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
    /// Sync committee reward in Gwei (can be negative).
    #[serde_as(as = "DisplayFromStr")]
    pub reward: i64,
}

/// Response from the [`/eth/v1/beacon/rewards/attestations/{epoch}`](https://ethereum.github.io/beacon-APIs/#/Rewards/getAttestationsRewards) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationRewardsResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// Whether the response references the finalized history of the chain.
    #[serde(default)]
    pub finalized: bool,
    /// Attestation rewards data.
    pub data: AttestationRewards,
}

/// Attestation rewards broken down into ideal and total rewards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationRewards {
    /// Ideal rewards for each effective balance.
    pub ideal_rewards: Vec<IdealAttestationReward>,
    /// Total rewards for each validator.
    pub total_rewards: Vec<TotalAttestationReward>,
}

/// Ideal attestation reward for a given effective balance.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdealAttestationReward {
    /// Effective balance in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub effective_balance: u64,
    /// Ideal head reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub head: u64,
    /// Ideal target reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub target: u64,
    /// Ideal source reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub source: u64,
    /// Ideal inclusion delay reward in Gwei (Phase0 only).
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub inclusion_delay: Option<u64>,
    /// Ideal inactivity reward in Gwei.
    #[serde_as(as = "DisplayFromStr")]
    pub inactivity: u64,
}

/// Total attestation reward for a single validator.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotalAttestationReward {
    /// Validator index.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
    /// Head reward in Gwei (can be negative).
    #[serde_as(as = "DisplayFromStr")]
    pub head: i64,
    /// Target reward in Gwei (can be negative).
    #[serde_as(as = "DisplayFromStr")]
    pub target: i64,
    /// Source reward in Gwei (can be negative).
    #[serde_as(as = "DisplayFromStr")]
    pub source: i64,
    /// Inclusion delay reward in Gwei (Phase0 only, can be negative).
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub inclusion_delay: Option<i64>,
    /// Inactivity reward in Gwei (can be negative).
    #[serde_as(as = "DisplayFromStr")]
    pub inactivity: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_block_rewards_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "proposer_index": "123",
                "total": "456",
                "attestations": "100",
                "sync_aggregate": "200",
                "proposer_slashings": "50",
                "attester_slashings": "106"
            }
        }"#;

        let response: BlockRewardsResponse = serde_json::from_str(s).unwrap();

        assert!(!response.execution_optimistic);
        assert!(response.finalized);
        assert_eq!(response.data.proposer_index, 123);
        assert_eq!(response.data.total, 456);
        assert_eq!(response.data.attestations, 100);
        assert_eq!(response.data.sync_aggregate, 200);
        assert_eq!(response.data.proposer_slashings, 50);
        assert_eq!(response.data.attester_slashings, 106);

        let roundtrip = serde_json::to_string(&response).unwrap();
        let deserialized: BlockRewardsResponse = serde_json::from_str(&roundtrip).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn serde_block_rewards_response_defaults() {
        let s = r#"{
            "data": {
                "proposer_index": "0",
                "total": "0",
                "attestations": "0",
                "sync_aggregate": "0",
                "proposer_slashings": "0",
                "attester_slashings": "0"
            }
        }"#;

        let response: BlockRewardsResponse = serde_json::from_str(s).unwrap();

        assert!(!response.execution_optimistic);
        assert!(!response.finalized);
    }

    #[test]
    fn serde_sync_committee_rewards_response() {
        let s = r#"{
            "execution_optimistic": true,
            "finalized": false,
            "data": [
                {
                    "validator_index": "1",
                    "reward": "2000"
                },
                {
                    "validator_index": "2",
                    "reward": "-500"
                }
            ]
        }"#;

        let response: SyncCommitteeRewardsResponse = serde_json::from_str(s).unwrap();

        assert!(response.execution_optimistic);
        assert!(!response.finalized);
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].validator_index, 1);
        assert_eq!(response.data[0].reward, 2000);
        assert_eq!(response.data[1].validator_index, 2);
        assert_eq!(response.data[1].reward, -500);

        let roundtrip = serde_json::to_string(&response).unwrap();
        let deserialized: SyncCommitteeRewardsResponse = serde_json::from_str(&roundtrip).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn serde_attestation_rewards_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": true,
            "data": {
                "ideal_rewards": [
                    {
                        "effective_balance": "32000000000",
                        "head": "2500",
                        "target": "5000",
                        "source": "3000",
                        "inclusion_delay": "1500",
                        "inactivity": "0"
                    }
                ],
                "total_rewards": [
                    {
                        "validator_index": "10",
                        "head": "2500",
                        "target": "-1000",
                        "source": "3000",
                        "inclusion_delay": "-200",
                        "inactivity": "0"
                    }
                ]
            }
        }"#;

        let response: AttestationRewardsResponse = serde_json::from_str(s).unwrap();

        assert!(!response.execution_optimistic);
        assert!(response.finalized);

        let ideal = &response.data.ideal_rewards[0];
        assert_eq!(ideal.effective_balance, 32000000000);
        assert_eq!(ideal.head, 2500);
        assert_eq!(ideal.target, 5000);
        assert_eq!(ideal.source, 3000);
        assert_eq!(ideal.inclusion_delay, Some(1500));
        assert_eq!(ideal.inactivity, 0);

        let total = &response.data.total_rewards[0];
        assert_eq!(total.validator_index, 10);
        assert_eq!(total.head, 2500);
        assert_eq!(total.target, -1000);
        assert_eq!(total.source, 3000);
        assert_eq!(total.inclusion_delay, Some(-200));
        assert_eq!(total.inactivity, 0);

        let roundtrip = serde_json::to_string(&response).unwrap();
        let deserialized: AttestationRewardsResponse = serde_json::from_str(&roundtrip).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn serde_attestation_rewards_without_inclusion_delay() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": false,
            "data": {
                "ideal_rewards": [
                    {
                        "effective_balance": "32000000000",
                        "head": "2500",
                        "target": "5000",
                        "source": "3000",
                        "inactivity": "0"
                    }
                ],
                "total_rewards": [
                    {
                        "validator_index": "10",
                        "head": "2500",
                        "target": "-1000",
                        "source": "3000",
                        "inactivity": "0"
                    }
                ]
            }
        }"#;

        let response: AttestationRewardsResponse = serde_json::from_str(s).unwrap();

        assert_eq!(response.data.ideal_rewards[0].inclusion_delay, None);
        assert_eq!(response.data.total_rewards[0].inclusion_delay, None);

        let roundtrip = serde_json::to_string(&response).unwrap();
        let deserialized: AttestationRewardsResponse = serde_json::from_str(&roundtrip).unwrap();
        assert_eq!(response, deserialized);
    }
}
