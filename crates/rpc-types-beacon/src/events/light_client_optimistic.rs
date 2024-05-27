use crate::header::BeaconBlockHeader;
use alloy_primitives::Bytes;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Contains optimistic data for the light client, including the attested header,
/// sync aggregate, and the signature slot.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightClientOptimisticData {
    /// The attested header containing a [`BeaconBlockHeader`].
    pub attested_header: AttestedHeader,
    /// The sync aggregate which includes the sync committee bits and signature.
    pub sync_aggregate: SyncAggregate,
    /// The slot in which the signature was included, serialized as a string.
    #[serde_as(as = "DisplayFromStr")]
    pub signature_slot: u64,
}

/// Contains the [`BeaconBlockHeader`] that was attested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestedHeader {
    /// The [`BeaconBlockHeader`] object from the CL spec.
    pub beacon: BeaconBlockHeader,
}

/// Contains the sync committee bits and signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncAggregate {
    /// The bits representing the sync committee's participation.
    pub sync_committee_bits: Bytes,
    /// The aggregated signature of the sync committee.
    pub sync_committee_signature: Bytes,
}
