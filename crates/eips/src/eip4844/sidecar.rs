//! EIP-4844 sidecar type

use crate::eip4844::{
    kzg_to_versioned_hash, Blob, Bytes48, BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_PROOF,
};
use alloy_primitives::{bytes::BufMut, B256};
use alloy_rlp::{Decodable, Encodable};

/// This represents a set of blobs, and its corresponding commitments and proofs.
///
/// This type encodes and decodes the fields without an rlp header.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobTransactionSidecar {
    /// The blob data.
    pub blobs: Vec<Blob>,
    /// The blob commitments.
    pub commitments: Vec<Bytes48>,
    /// The blob proofs.
    pub proofs: Vec<Bytes48>,
}

impl BlobTransactionSidecar {
    /// Constructs a new [BlobTransactionSidecar] from a set of blobs, commitments, and proofs.
    pub const fn new(blobs: Vec<Blob>, commitments: Vec<Bytes48>, proofs: Vec<Bytes48>) -> Self {
        Self { blobs, commitments, proofs }
    }

    /// Creates a new instance from the given KZG types.
    #[cfg(feature = "kzg")]
    pub fn from_kzg(
        blobs: Vec<c_kzg::Blob>,
        commitments: Vec<c_kzg::Bytes48>,
        proofs: Vec<c_kzg::Bytes48>,
    ) -> Self {
        // transmutes the vec of items, see also [std::mem::transmute](https://doc.rust-lang.org/std/mem/fn.transmute.html)
        unsafe fn transmute_vec<U, T>(input: Vec<T>) -> Vec<U> {
            let mut v = std::mem::ManuallyDrop::new(input);
            Vec::from_raw_parts(v.as_mut_ptr() as *mut U, v.len(), v.capacity())
        }

        // SAFETY: all types have the same size and alignment
        unsafe {
            let blobs = transmute_vec::<Blob, c_kzg::Blob>(blobs);
            let commitments = transmute_vec::<Bytes48, c_kzg::Bytes48>(commitments);
            let proofs = transmute_vec::<Bytes48, c_kzg::Bytes48>(proofs);
            Self { blobs, commitments, proofs }
        }
    }

    /// Returns an iterator over the versioned hashes of the commitments.
    pub fn versioned_hashes(&self) -> impl Iterator<Item = B256> + '_ {
        self.commitments.iter().map(|c| kzg_to_versioned_hash(c.as_slice()))
    }

    /// Returns the versioned hash for the blob at the given index, if it
    /// exists.
    pub fn versioned_hash_for_blob(&self, blob_index: usize) -> Option<B256> {
        self.commitments.get(blob_index).map(|c| kzg_to_versioned_hash(c.as_slice()))
    }

    /// Encodes the inner [BlobTransactionSidecar] fields as RLP bytes, __without__ a RLP header.
    ///
    /// This encodes the fields in the following order:
    /// - `blobs`
    /// - `commitments`
    /// - `proofs`
    #[inline]
    pub(crate) fn encode_inner(&self, out: &mut dyn BufMut) {
        // Encode the blobs, commitments, and proofs
        self.blobs.encode(out);
        self.commitments.encode(out);
        self.proofs.encode(out);
    }

    /// Outputs the RLP length of the [BlobTransactionSidecar] fields, without a RLP header.
    pub fn fields_len(&self) -> usize {
        self.blobs.length() + self.commitments.length() + self.proofs.length()
    }

    /// Calculates a size heuristic for the in-memory size of the [BlobTransactionSidecar].
    #[inline]
    pub fn size(&self) -> usize {
        self.blobs.len() * BYTES_PER_BLOB + // blobs
            self.commitments.len() * BYTES_PER_COMMITMENT + //   commitments
            self.proofs.len() * BYTES_PER_PROOF // proofs
    }
}

impl Encodable for BlobTransactionSidecar {
    /// Encodes the inner [BlobTransactionSidecar] fields as RLP bytes, without a RLP header.
    fn encode(&self, s: &mut dyn BufMut) {
        self.encode_inner(s);
    }

    fn length(&self) -> usize {
        self.fields_len()
    }
}

impl Decodable for BlobTransactionSidecar {
    /// Decodes the inner [BlobTransactionSidecar] fields from RLP bytes, without a RLP header.
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            blobs: Decodable::decode(buf)?,
            commitments: Decodable::decode(buf)?,
            proofs: Decodable::decode(buf)?,
        })
    }
}
