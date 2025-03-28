use crate::{
    eip4844::{
        kzg_to_versioned_hash, BlobTransactionSidecarEip4844, BlobTransactionValidationError,
    },
    eip7594::CELLS_PER_EXT_BLOB,
};
use c_kzg::Bytes48;

/// This represents a set of blobs, and its corresponding commitments and proofs.
///
/// This type encodes and decodes the fields without an rlp header.
#[derive(Clone, Default, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlobTransactionSidecar {
    Eip4844(BlobTransactionSidecarEip4844),
    Eip7594(BlobTransactionSidecarEip7594),
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobTransactionSidecarEip7594 {
    /// The blob data.
    #[cfg_attr(
        all(debug_assertions, feature = "serde"),
        serde(deserialize_with = "deserialize_blobs")
    )]
    pub blobs: Vec<Blob>,
    /// The blob commitments.
    pub commitments: Vec<Bytes48>,
    /// List of cell proofs for all blobs in the sidecar, including the proofs for the extension
    /// indices, for a total of `CELLS_PER_EXT_BLOB` proofs per blob (`CELLS_PER_EXT_BLOB` is the
    /// number of cells for an extended blob, defined in
    /// [the consensus specs](https://github.com/ethereum/consensus-specs/tree/9d377fd53d029536e57cfda1a4d2c700c59f86bf/specs/fulu/polynomial-commitments-sampling.md#cells))
    pub cell_proofs: Vec<Bytes48>,
}

impl core::fmt::Debug for BlobTransactionSidecarEip7594 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobTransactionSidecarEip7594")
            .field("blobs", &self.blobs.len())
            .field("commitments", &self.commitments)
            .field("cell_proofs", &self.cell_proofs)
            .finish()
    }
}

impl BlobTransactionSidecarEip7594 {
    /// Constructs a new [BlobTransactionSidecarEip7594] from a set of blobs, commitments, and
    /// cell proofs.
    pub const fn new(
        blobs: Vec<Blob>,
        commitments: Vec<Bytes48>,
        cell_proofs: Vec<Bytes48>,
    ) -> Self {
        Self { blobs, commitments, cell_proofs }
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

        let expected_cell_proofs_len = self.blobs.len() * CELLS_PER_EXT_BLOB;
        if self.cell_proofs.len() != expected_cell_proofs_len {
            return Err(c_kzg::Error::MismatchLength(format!(
                "There are {} cell proofs and {} blobs. Expected {} cell proofs.",
                self.cell_proofs.len(),
                self.blobs.len(),
                expected_cell_proofs_len
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

        // TODO:
        // // SAFETY: ALL types have the same size
        // let res = unsafe {
        //     c_kzg::KzgProof::verify_blob_kzg_proof_batch(
        //         // blobs
        //         core::mem::transmute::<&[Blob], &[c_kzg::Blob]>(self.blobs.as_slice()),
        //         // commitments
        //         core::mem::transmute::<&[Bytes48],
        // &[c_kzg::Bytes48]>(self.commitments.as_slice()),         // proofs
        //         core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.proofs.as_slice()),
        //         proof_settings,
        //     )
        // }
        // .map_err(BlobTransactionValidationError::KZGError)?;

        // res.then_some(()).ok_or(BlobTransactionValidationError::InvalidProof)
        todo!()
    }
}
