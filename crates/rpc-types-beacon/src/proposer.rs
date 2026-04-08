//! Types for the proposer duties endpoint.
//!
//! See <https://ethereum.github.io/beacon-APIs/#/Validator/getProposerDuties>

use crate::BlsPublicKey;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the [`/eth/v1/validator/duties/proposer/{epoch}`](https://ethereum.github.io/beacon-APIs/#/Validator/getProposerDuties) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposerDutiesResponse {
    /// Whether the response references an unverified execution payload.
    #[serde(default)]
    pub execution_optimistic: bool,
    /// The dependent root for the response.
    pub dependent_root: alloy_primitives::B256,
    /// The list of proposer duties.
    pub data: Vec<ProposerDuty>,
}

/// A single proposer duty entry.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposerDuty {
    /// The BLS public key of the validator assigned to propose.
    pub pubkey: BlsPublicKey,
    /// The index of the validator in the validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub validator_index: u64,
    /// The slot at which the validator is expected to propose.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_proposer_duties_response() {
        let s = r#"{
            "execution_optimistic": false,
            "dependent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "data": [
                {
                    "pubkey": "0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a",
                    "validator_index": "1",
                    "slot": "1"
                }
            ]
        }"#;
        let resp: ProposerDutiesResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].validator_index, 1);
        assert_eq!(resp.data[0].slot, 1);
    }
}
