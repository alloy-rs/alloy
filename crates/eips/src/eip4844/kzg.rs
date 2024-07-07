use super::{BlobTransactionSidecar, BlobTransactionValidationError};

/// r
pub trait KzgProofVerifier {
    /// f
    type KzgSettings;

    /// f
    fn verify_blob_kzg_proof_batch(
        &self,
        kzg_settings: &KzgSettings,
    ) -> Result<bool, BlobTransactionValidationError>;

    /// g
    fn check_blob_versioned_hashes_length(
        &self,
        blob_versioned_hashes_len: usize,
    ) -> Result<(), BlobTransactionValidationError>;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "kzg")] {
        use super::{Blob, Bytes48};
        pub use c_kzg::{Error as KzgError, KzgSettings};

        impl KzgProofVerifier for BlobTransactionSidecar {
            type KzgSettings = KzgSettings;

            fn verify_blob_kzg_proof_batch(&self, kzg_settings: &KzgSettings) -> Result<bool, BlobTransactionValidationError> {
                // SAFETY: ALL types have the same size
                unsafe {
                    c_kzg::KzgProof::verify_blob_kzg_proof_batch(
                        // blobs
                        core::mem::transmute::<&[Blob], &[c_kzg::Blob]>(self.blobs.as_slice()),
                        // commitments
                        core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.commitments.as_slice()),
                        // proofs
                        core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.proofs.as_slice()),
                        kzg_settings,
                    )
                }
                .map_err(BlobTransactionValidationError::KZGError)
            }

            fn check_blob_versioned_hashes_length(
                &self,
                blob_versioned_hashes_len: usize,
            ) -> Result<(), BlobTransactionValidationError> {
                if blob_versioned_hashes_len != self.commitments.len() {
                    return Err(KzgError::MismatchLength(format!(
                        "There are {} versioned commitment hashes and {} commitments",
                        blob_versioned_hashes_len,
                        self.commitments.len()
                    ))
                    .into());
                }

                Ok(())
            }
        }
    } else if #[cfg(feature = "kzg-rs")] {
        pub use kzg_rs::{KzgError, KzgSettings};

        impl KzgProofVerifier for BlobTransactionSidecar {
            type KzgSettings = KzgSettings;

            fn verify_blob_kzg_proof_batch(
                &self,
                kzg_settings: &KzgSettings
            ) -> Result<bool, BlobTransactionValidationError> {
                kzg_rs::KzgProof::verify_blob_kzg_proof_batch(
                    self.blobs.iter().map(|b| kzg_rs::Blob::from_slice(b.as_slice())).collect::<Result<Vec<_>, KzgError>>()?,
                    self.commitments.iter().map(|c| kzg_rs::Bytes48::from_slice(c.as_slice())).collect::<Result<Vec<_>, KzgError>>()?,
                    self.proofs.iter().map(|p| kzg_rs::Bytes48::from_slice(p.as_slice())).collect::<Result<Vec<_>, KzgError>>()?,
                    kzg_settings
                )
                .map_err(BlobTransactionValidationError::KZGError)
            }

            fn check_blob_versioned_hashes_length(
                &self,
                blob_versioned_hashes_len: usize,
            ) -> Result<(), BlobTransactionValidationError> {
                if blob_versioned_hashes_len != self.commitments.len() {
                    return Err(KzgError::InvalidBytesLength(format!(
                        "There are {} versioned commitment hashes and {} commitments",
                        blob_versioned_hashes_len,
                        self.commitments.len()
                    ))
                    .into());
                }

                Ok(())
            }
        }
    }
}
