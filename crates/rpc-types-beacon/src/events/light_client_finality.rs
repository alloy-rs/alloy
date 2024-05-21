use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Contains finality data for the light client, including attested and finalized headers,
/// finality branch, sync aggregate, and the signature slot.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightClientFinalityData {
    /// The attested header containing a `Beacon`.
    pub attested_header: AttestedHeader,
    /// The finalized header containing a `Beacon2`.
    pub finalized_header: FinalizedHeader,
    /// The Merkle branch proof for the finality.
    pub finality_branch: Vec<String>,
    /// The sync aggregate which includes the sync committee bits and signature.
    pub sync_aggregate: SyncAggregate,
    /// The slot in which the signature was included, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub signature_slot: u64,
}

/// Contains the `Beacon` header that was attested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestedHeader {
    /// The `Beacon` object representing the block header.
    pub beacon: Beacon,
}

/// Represents the header of a beacon block.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beacon {
    /// The slot number of the beacon block, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// The index of the proposer of the beacon block, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_index: u64,
    /// The root of the parent block.
    pub parent_root: B256,
    /// The state root after the block is processed.
    pub state_root: B256,
    /// The root of the block body.
    pub body_root: B256,
}

/// Contains the `Beacon2` header that was finalized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizedHeader {
    /// The `Beacon2` object representing the block header.
    pub beacon: Beacon2,
}

/// Represents the header of a finalized beacon block.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beacon2 {
    /// The slot number of the beacon block, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// The index of the proposer of the beacon block, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_index: u64,
    /// The root of the parent block.
    pub parent_root: B256,
    /// The state root after the block is processed.
    pub state_root: B256,
    /// The root of the block body.
    pub body_root: B256,
}

/// Contains the sync committee bits and signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncAggregate {
    /// The bits representing the sync committee's participation.
    pub sync_committee_bits: Bytes,
    /// The aggregated signature of the sync committee.
    pub sync_committee_signature: Bytes,
}
