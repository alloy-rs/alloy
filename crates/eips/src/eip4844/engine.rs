//! Misc types related to the 4844

use crate::eip4844::{Blob, Bytes48};
use alloc::{boxed::Box, vec::Vec};

/// Blob type returned in responses to `engine_getBlobsV1`: <https://github.com/ethereum/execution-apis/pull/559>
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobAndProofV1 {
    /// The blob data.
    pub blob: Box<Blob>,
    /// The KZG proof for the blob.
    pub proof: Bytes48,
}

/// Blob type returned in responses to `engine_getBlobsV2`: <https://github.com/ethereum/execution-apis/pull/630>
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BlobAndProofV2 {
    /// The blob data.
    pub blob: Box<Blob>,
    /// The cell proofs for the blob.
    pub proofs: Vec<Bytes48>,
}
