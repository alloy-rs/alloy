use alloy_eips::{
    eip4844::BlobTransactionSidecar,
    eip7594::{BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant},
};
use alloy_rlp::BufMut;

#[cfg(feature = "kzg")]
#[doc(inline)]
pub use alloy_eips::eip4844::BlobTransactionValidationError;

/// An EIP-4844 transaction sidecar.
pub trait TxEip4844Sidecar {
    /// Verifies that the versioned hashes are valid for this sidecar's blob data, commitments, and
    /// proofs.
    #[cfg(feature = "kzg")]
    fn validate(
        &self,
        blob_versioned_hashes: &[alloy_primitives::B256],
        proof_settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError>;

    /// Outputs the RLP length of the sidecar fields, without a RLP header.
    fn rlp_encoded_fields_length(&self) -> usize;

    /// Encodes the inner sidecar fields as RLP bytes, __without__ a RLP header.
    fn rlp_encode_fields(&self, out: &mut dyn BufMut);

    /// RLP decode the fields of a sidecar.
    fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self>
    where
        Self: Sized;

    /// Calculate a size heuristic for the in-memory size of the sidecar.
    fn size(&self) -> usize;
}

impl TxEip4844Sidecar for BlobTransactionSidecar {
    #[cfg(feature = "kzg")]
    fn validate(
        &self,
        blob_versioned_hashes: &[alloy_primitives::B256],
        proof_settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        Self::validate(self, blob_versioned_hashes, proof_settings)
    }

    fn rlp_encoded_fields_length(&self) -> usize {
        Self::rlp_encoded_fields_length(self)
    }

    fn rlp_encode_fields(&self, out: &mut dyn BufMut) {
        Self::rlp_encode_fields(self, out);
    }

    fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::rlp_decode_fields(buf)
    }

    fn size(&self) -> usize {
        Self::size(self)
    }
}

impl TxEip4844Sidecar for BlobTransactionSidecarEip7594 {
    #[cfg(feature = "kzg")]
    fn validate(
        &self,
        blob_versioned_hashes: &[alloy_primitives::B256],
        proof_settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        Self::validate(self, blob_versioned_hashes, proof_settings)
    }

    fn rlp_encoded_fields_length(&self) -> usize {
        Self::rlp_encoded_fields_length(self)
    }

    fn rlp_encode_fields(&self, out: &mut dyn BufMut) {
        Self::rlp_encode_fields(self, out);
    }

    fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::rlp_decode_fields(buf)
    }

    fn size(&self) -> usize {
        Self::size(self)
    }
}

impl TxEip4844Sidecar for BlobTransactionSidecarVariant {
    #[cfg(feature = "kzg")]
    fn validate(
        &self,
        blob_versioned_hashes: &[alloy_primitives::B256],
        proof_settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        Self::validate(self, blob_versioned_hashes, proof_settings)
    }

    fn rlp_encoded_fields_length(&self) -> usize {
        Self::rlp_encoded_fields_length(self)
    }

    fn rlp_encode_fields(&self, out: &mut dyn BufMut) {
        Self::rlp_encode_fields(self, out);
    }

    fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::rlp_decode_fields(buf)
    }

    fn size(&self) -> usize {
        Self::size(self)
    }
}
