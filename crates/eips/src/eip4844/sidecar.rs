//! EIP-4844 sidecar type

use crate::eip4844::{
    kzg_to_versioned_hash, Blob, Bytes48, BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_PROOF,
};
use alloy_primitives::{bytes::BufMut, B256};
use alloy_rlp::{Decodable, Encodable};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

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
        // transmutes the vec of items, see also [core::mem::transmute](https://doc.rust-lang.org/std/mem/fn.transmute.html)
        unsafe fn transmute_vec<U, T>(input: Vec<T>) -> Vec<U> {
            let mut v = core::mem::ManuallyDrop::new(input);
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

    /// Verifies that the versioned hashes are valid for this sidecar's blob data, commitments, and
    /// proofs.
    ///
    /// Takes as input the [KzgSettings](c_kzg::KzgSettings), which should contain the parameters
    /// derived from the KZG trusted setup.
    ///
    /// This ensures that the blob transaction payload has the same number of blob data elements,
    /// commitments, and proofs. Each blob data element is verified against its commitment and
    /// proof.
    ///
    /// Returns [BlobTransactionValidationError::InvalidProof] if any blob KZG proof in the response
    /// fails to verify, or if the versioned hashes in the transaction do not match the actual
    /// commitment versioned hashes.
    #[cfg(feature = "kzg")]
    pub fn validate(
        &self,
        blob_versioned_hashes: &[B256],
        proof_settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        // Ensure the versioned hashes and commitments have the same length.
        if blob_versioned_hashes.len() != self.commitments.len() {
            return Err(c_kzg::Error::MismatchLength(format!(
                "There are {} versioned commitment hashes and {} commitments",
                blob_versioned_hashes.len(),
                self.commitments.len()
            ))
            .into());
        }

        // calculate versioned hashes by zipping & iterating
        for (versioned_hash, commitment) in
            blob_versioned_hashes.iter().zip(self.commitments.iter())
        {
            let commitment = c_kzg::KzgCommitment::from(commitment.0);

            // calculate & verify versioned hash
            let calculated_versioned_hash = kzg_to_versioned_hash(commitment.as_slice());
            if *versioned_hash != calculated_versioned_hash {
                return Err(BlobTransactionValidationError::WrongVersionedHash {
                    have: *versioned_hash,
                    expected: calculated_versioned_hash,
                });
            }
        }

        // SAFETY: ALL types have the same size
        let res = unsafe {
            c_kzg::KzgProof::verify_blob_kzg_proof_batch(
                // blobs
                core::mem::transmute::<&[Blob], &[c_kzg::Blob]>(self.blobs.as_slice()),
                // commitments
                core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.commitments.as_slice()),
                // proofs
                core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.proofs.as_slice()),
                proof_settings,
            )
        }
        .map_err(BlobTransactionValidationError::KZGError)?;

        if res {
            Ok(())
        } else {
            Err(BlobTransactionValidationError::InvalidProof)
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
            self.commitments.len() * BYTES_PER_COMMITMENT + // commitments
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

/// An error that can occur when validating a [BlobTransactionSidecar::validate].
#[derive(Debug)]
#[cfg(feature = "kzg")]
pub enum BlobTransactionValidationError {
    /// Proof validation failed.
    InvalidProof,
    /// An error returned by [`c_kzg`].
    KZGError(c_kzg::Error),
    /// The inner transaction is not a blob transaction.
    NotBlobTransaction(u8),
    /// Error variant for thrown by EIP-4844 tx variants without a sidecar.
    MissingSidecar,
    /// The versioned hash is incorrect.
    WrongVersionedHash {
        /// The versioned hash we got
        have: B256,
        /// The versioned hash we expected
        expected: B256,
    },
}

#[cfg(all(feature = "kzg", feature = "std"))]
impl std::error::Error for BlobTransactionValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BlobTransactionValidationError::InvalidProof { .. } => None,
            BlobTransactionValidationError::KZGError(source) => Some(source),
            BlobTransactionValidationError::NotBlobTransaction { .. } => None,
            BlobTransactionValidationError::MissingSidecar { .. } => None,
            BlobTransactionValidationError::WrongVersionedHash { .. } => None,
        }
    }
}

#[cfg(feature = "kzg")]
impl core::fmt::Display for BlobTransactionValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BlobTransactionValidationError::InvalidProof => f.write_str("invalid KZG proof"),
            BlobTransactionValidationError::KZGError(err) => {
                write!(f, "KZG error: {:?}", err)
            }
            BlobTransactionValidationError::NotBlobTransaction(err) => {
                write!(f, "unable to verify proof for non blob transaction: {}", err)
            }
            BlobTransactionValidationError::MissingSidecar => {
                f.write_str("eip4844 tx variant without sidecar being used for verification.")
            }
            BlobTransactionValidationError::WrongVersionedHash { have, expected } => {
                write!(f, "wrong versioned hash: have {}, expected {}", have, expected)
            }
        }
    }
}

#[cfg(feature = "kzg")]
impl From<c_kzg::Error> for BlobTransactionValidationError {
    fn from(source: c_kzg::Error) -> Self {
        BlobTransactionValidationError::KZGError(source)
    }
}
