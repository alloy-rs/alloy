use alloy_eips::eip4844::{Blob, Bytes48};
use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Bundle of blobs for a given block
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlobBundle {
    /// Vec of individual blob data
    pub data: Vec<BlobData>,
}

/// Individual Blob data that belongs to a 4844 transaction.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobData {
    /// Blob index
    #[serde_as(as = "DisplayFromStr")]
    pub index: u64,
    #[serde(deserialize_with = "deserialize_blob")]
    /// Blob data
    pub blob: Box<Blob>,
    /// The blob's commitment
    pub kzg_commitment: Bytes48,
    /// The blob's proof
    pub kzg_proof: Bytes48,
    /// The block header containing the blob
    pub signed_block_header: SignedBlockHeader,
    /// The blob's inclusion proofs
    pub kzg_commitment_inclusion_proof: Vec<B256>,
}

/// The Block data for a set of blobs
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedBlockHeader {
    pub message: BlockHeaderMessage,
    pub signature: Bytes,
}

/// Detailed Block data
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeaderMessage {
    /// The block slot.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_index: u64,
    pub parent_root: B256,
    pub state_root: B256,
    pub body_root: B256,
}

// Helper function to deserialize boxed blobs
fn deserialize_blob<'de, D>(deserializer: D) -> Result<Box<Blob>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let raw_blob = <Bytes>::deserialize(deserializer)?;

    let blob = Box::new(Blob::try_from(raw_blob.as_ref()).map_err(serde::de::Error::custom)?);

    Ok(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Should deserialise json containing 6 blobs
    #[test]
    fn serde_sidecar_bundle() {
        let s = include_str!("examples/sidecar.json");
        let resp: BeaconBlobBundle = serde_json::from_str(s).unwrap();
        let json: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(json, serde_json::to_value(resp.clone()).unwrap());
        assert_eq!(6, resp.data.len());
    }
}
