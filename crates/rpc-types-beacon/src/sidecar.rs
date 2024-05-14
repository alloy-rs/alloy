use crate::header::Header;
use alloy_eips::eip4844::{Blob, BlobTransactionSidecar, Bytes48};
use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::vec::IntoIter;

/// Bundle of blobs for a given block
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlobBundle {
    /// Vec of individual blob data
    data: Vec<BlobData>,
}

/// Yields an iterator for BlobData
impl IntoIterator for BeaconBlobBundle {
    type Item = BlobData;
    type IntoIter = IntoIter<BlobData>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

/// Intermediate type for BlobTransactionSidecar matching
#[derive(Debug, Clone)]
pub struct SidecarIterator {
    pub iter: IntoIter<BlobData>,
}

impl Iterator for SidecarIterator {
    type Item = BlobData;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl SidecarIterator {
    pub fn new(bundle: BeaconBlobBundle) -> Self {
        SidecarIterator { iter: bundle.into_iter() }
    }

    /// Returns a BlobTransactionSidecar of len num_hashes.
    pub fn next_sidecar(&mut self, num_hashes: usize) -> Option<BlobTransactionSidecar> {
        let mut blobs = Vec::with_capacity(num_hashes);
        let mut commitments = Vec::with_capacity(num_hashes);
        let mut proofs = Vec::with_capacity(num_hashes);
        for _ in 0..num_hashes {
            let next = self.next()?;
            blobs.push(*next.blob);
            commitments.push(next.kzg_commitment);
            proofs.push(next.kzg_proof);
        }
        Some(BlobTransactionSidecar { blobs, commitments, proofs })
    }
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
    pub signed_block_header: Header,
    /// The blob's inclusion proofs
    pub kzg_commitment_inclusion_proof: Vec<B256>,
}

/// Helper function to deserialize boxed blobs
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

    /// Should deserialise json containing 6 blobs
    #[test]
    fn serde_sidecar_bundle() {
        let s = include_str!("examples/sidecar.json");
        let resp: BeaconBlobBundle = serde_json::from_str(s).unwrap();
        let json: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(json, serde_json::to_value(resp.clone()).unwrap());
        assert_eq!(6, resp.data.len());
    }
}
