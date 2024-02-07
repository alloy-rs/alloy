//! EIP-4844 related types.

use crate::kzg::{Blob, Bytes48};
use serde::{Deserialize, Serialize};

/// This represents a set of blobs, and its corresponding commitments and proofs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct BlobTransactionSidecar {
    /// The blob data.
    pub blobs: Vec<Blob>,
    /// The blob commitments.
    pub commitments: Vec<Bytes48>,
    /// The blob proofs.
    pub proofs: Vec<Bytes48>,
}
