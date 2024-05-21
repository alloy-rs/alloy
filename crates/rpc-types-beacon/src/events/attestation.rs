use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Contains data related to an attestation, including slot, index, beacon block root,
/// source, and target information.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationData {
    /// The slot number in which the attestation was included, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// The committee index of the attestation, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub index: u64,
    /// The root of the beacon block being attested to.
    pub beacon_block_root: B256,
    /// The source checkpoint of the attestation.
    pub source: Source,
    /// The target checkpoint of the attestation.
    pub target: Target,
}

/// Represents the source checkpoint of an attestation.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The epoch number of the source checkpoint, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
    /// The root of the source checkpoint.
    pub root: B256,
}

/// Represents the target checkpoint of an attestation.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// The epoch number of the target checkpoint, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
    /// The root of the target checkpoint.
    pub root: B256,
}
