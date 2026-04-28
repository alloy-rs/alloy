//! Misc types related to the 4844

use crate::{
    eip4844::{Blob, Bytes48},
    eip7594::{Cell, CELLS_PER_EXT_BLOB},
};
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

#[cfg(feature = "ssz")]
impl ssz::Encode for BlobAndProofV1 {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        <Blob as ssz::Encode>::ssz_fixed_len() + <Bytes48 as ssz::Encode>::ssz_fixed_len()
    }

    fn ssz_bytes_len(&self) -> usize {
        Self::ssz_fixed_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        ssz::Encode::ssz_append(self.blob.as_ref(), buf);
        ssz::Encode::ssz_append(&self.proof, buf);
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobAndProofV1 {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        <Blob as ssz::Decode>::ssz_fixed_len() + <Bytes48 as ssz::Decode>::ssz_fixed_len()
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<Blob>()?;
        builder.register_type::<Bytes48>()?;

        let mut decoder = builder.build()?;
        Ok(Self { blob: Box::new(decoder.decode_next()?), proof: decoder.decode_next()? })
    }
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

#[cfg(feature = "ssz")]
impl ssz::Encode for BlobAndProofV2 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        <Blob as ssz::Encode>::ssz_fixed_len()
            + <Vec<Bytes48> as ssz::Encode>::ssz_fixed_len()
            + ssz::Encode::ssz_bytes_len(&self.proofs)
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let offset =
            <Blob as ssz::Encode>::ssz_fixed_len() + <Vec<Bytes48> as ssz::Encode>::ssz_fixed_len();
        let mut encoder = ssz::SszEncoder::container(buf, offset);
        encoder.append(self.blob.as_ref());
        encoder.append(&self.proofs);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobAndProofV2 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<Blob>()?;
        builder.register_type::<Vec<Bytes48>>()?;

        let mut decoder = builder.build()?;
        let blob = decoder.decode_next()?;
        let proofs: Vec<Bytes48> = decoder.decode_next()?;
        if proofs.len() > crate::eip7594::CELLS_PER_EXT_BLOB {
            return Err(ssz::DecodeError::BytesInvalid(format!(
                "Invalid BlobAndProofV2: expected at most {} proofs, got {}",
                crate::eip7594::CELLS_PER_EXT_BLOB,
                proofs.len()
            )));
        }

        Ok(Self { blob: Box::new(blob), proofs })
    }
}

/// Blob cells type returned in responses to `engine_getBlobsV4`:
/// <https://github.com/ethereum/execution-apis/pull/774>
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobCellsAndProofsV1 {
    /// The requested blob cells.
    pub blob_cells: Vec<Option<Cell>>,
    /// The KZG proofs for the requested blob cells.
    pub proofs: Vec<Option<Bytes48>>,
}

#[cfg(feature = "ssz")]
impl ssz::Encode for BlobCellsAndProofsV1 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        <Vec<Option<Cell>> as ssz::Encode>::ssz_fixed_len()
            + <Vec<Option<Bytes48>> as ssz::Encode>::ssz_fixed_len()
            + ssz::Encode::ssz_bytes_len(&self.blob_cells)
            + ssz::Encode::ssz_bytes_len(&self.proofs)
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let offset = <Vec<Option<Cell>> as ssz::Encode>::ssz_fixed_len()
            + <Vec<Option<Bytes48>> as ssz::Encode>::ssz_fixed_len();
        let mut encoder = ssz::SszEncoder::container(buf, offset);
        encoder.append(&self.blob_cells);
        encoder.append(&self.proofs);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobCellsAndProofsV1 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<Vec<Option<Cell>>>()?;
        builder.register_type::<Vec<Option<Bytes48>>>()?;

        let mut decoder = builder.build()?;
        let blob_cells: Vec<Option<Cell>> = decoder.decode_next()?;
        let proofs: Vec<Option<Bytes48>> = decoder.decode_next()?;

        if blob_cells.len() > CELLS_PER_EXT_BLOB {
            return Err(ssz::DecodeError::BytesInvalid(format!(
                "Invalid BlobCellsAndProofsV1: expected at most {} blob cells, got {}",
                CELLS_PER_EXT_BLOB,
                blob_cells.len()
            )));
        }

        if blob_cells.len() != proofs.len() {
            return Err(ssz::DecodeError::BytesInvalid(format!(
                "Invalid BlobCellsAndProofsV1: blob_cells length {} does not match proofs length {}",
                blob_cells.len(),
                proofs.len()
            )));
        }

        if blob_cells.iter().zip(&proofs).any(|(cell, proof)| cell.is_some() != proof.is_some()) {
            return Err(ssz::DecodeError::BytesInvalid(
                "Invalid BlobCellsAndProofsV1: blob_cells and proofs must have matching null positions".into(),
            ));
        }

        Ok(Self { blob_cells, proofs })
    }
}

#[cfg(all(test, feature = "ssz"))]
mod tests {
    use super::*;
    use crate::eip4844::BYTES_PER_BLOB;

    #[test]
    fn ssz_blob_and_proof_v1_roundtrip() {
        let blob_and_proof = BlobAndProofV1 {
            blob: Box::new(Blob::repeat_byte(0x42)),
            proof: Bytes48::repeat_byte(0x24),
        };

        let encoded = ssz::Encode::as_ssz_bytes(&blob_and_proof);
        assert_eq!(encoded.len(), BYTES_PER_BLOB + 48);
        assert_eq!(&encoded[..BYTES_PER_BLOB], blob_and_proof.blob.as_slice());
        assert_eq!(&encoded[BYTES_PER_BLOB..], blob_and_proof.proof.as_slice());

        let decoded = <BlobAndProofV1 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap();
        assert_eq!(decoded, blob_and_proof);
    }

    #[test]
    fn ssz_blob_and_proof_v2_roundtrip() {
        let proofs =
            (0..CELLS_PER_EXT_BLOB).map(|i| Bytes48::repeat_byte(i as u8)).collect::<Vec<_>>();
        let blob_and_proof = BlobAndProofV2 { blob: Box::new(Blob::repeat_byte(0x42)), proofs };

        let encoded = ssz::Encode::as_ssz_bytes(&blob_and_proof);
        let expected_offset = BYTES_PER_BLOB + ssz::BYTES_PER_LENGTH_OFFSET;
        assert_eq!(encoded.len(), expected_offset + CELLS_PER_EXT_BLOB * 48);
        assert_eq!(
            u32::from_le_bytes(encoded[BYTES_PER_BLOB..expected_offset].try_into().unwrap())
                as usize,
            expected_offset
        );
        assert_eq!(&encoded[..BYTES_PER_BLOB], blob_and_proof.blob.as_slice());

        let mut proof_chunks = encoded[expected_offset..].chunks_exact(48);
        for (proof, chunk) in blob_and_proof.proofs.iter().zip(&mut proof_chunks) {
            assert_eq!(chunk, proof.as_slice());
        }
        assert!(proof_chunks.remainder().is_empty());

        let decoded = <BlobAndProofV2 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap();
        assert_eq!(decoded, blob_and_proof);
    }

    #[test]
    fn ssz_blob_and_proof_v2_rejects_too_many_proofs() {
        let blob_and_proof = BlobAndProofV2 {
            blob: Box::new(Blob::repeat_byte(0x42)),
            proofs: vec![Bytes48::ZERO; CELLS_PER_EXT_BLOB + 1],
        };
        let encoded = ssz::Encode::as_ssz_bytes(&blob_and_proof);

        let err = <BlobAndProofV2 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap_err();
        assert!(
            matches!(err, ssz::DecodeError::BytesInvalid(message) if message.contains("BlobAndProofV2"))
        );
    }

    #[test]
    fn ssz_blob_cells_and_proofs_v1_roundtrip() {
        let blob_cells = vec![
            Some(Cell::repeat_byte(0x01)),
            None,
            Some(Cell::repeat_byte(0x03)),
            Some(Cell::repeat_byte(0x04)),
        ];
        let proofs = vec![
            Some(Bytes48::repeat_byte(0x11)),
            None,
            Some(Bytes48::repeat_byte(0x33)),
            Some(Bytes48::repeat_byte(0x44)),
        ];
        let blob_cells_and_proofs = BlobCellsAndProofsV1 { blob_cells, proofs };

        let encoded = ssz::Encode::as_ssz_bytes(&blob_cells_and_proofs);
        let decoded = <BlobCellsAndProofsV1 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap();
        assert_eq!(decoded, blob_cells_and_proofs);
    }

    #[test]
    fn ssz_blob_cells_and_proofs_v1_rejects_too_many_cells() {
        let blob_cells_and_proofs = BlobCellsAndProofsV1 {
            blob_cells: vec![Some(Cell::ZERO); CELLS_PER_EXT_BLOB + 1],
            proofs: vec![Some(Bytes48::ZERO); CELLS_PER_EXT_BLOB + 1],
        };
        let encoded = ssz::Encode::as_ssz_bytes(&blob_cells_and_proofs);

        let err = <BlobCellsAndProofsV1 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap_err();
        assert!(
            matches!(err, ssz::DecodeError::BytesInvalid(message) if message.contains("expected at most"))
        );
    }

    #[test]
    fn ssz_blob_cells_and_proofs_v1_rejects_mismatched_lengths() {
        let blob_cells_and_proofs = BlobCellsAndProofsV1 {
            blob_cells: vec![Some(Cell::ZERO)],
            proofs: vec![Some(Bytes48::ZERO), Some(Bytes48::ZERO)],
        };
        let encoded = ssz::Encode::as_ssz_bytes(&blob_cells_and_proofs);

        let err = <BlobCellsAndProofsV1 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap_err();
        assert!(
            matches!(err, ssz::DecodeError::BytesInvalid(message) if message.contains("does not match"))
        );
    }

    #[test]
    fn ssz_blob_cells_and_proofs_v1_rejects_mismatched_null_positions() {
        let blob_cells_and_proofs = BlobCellsAndProofsV1 {
            blob_cells: vec![Some(Cell::ZERO), None],
            proofs: vec![None, Some(Bytes48::ZERO)],
        };
        let encoded = ssz::Encode::as_ssz_bytes(&blob_cells_and_proofs);

        let err = <BlobCellsAndProofsV1 as ssz::Decode>::from_ssz_bytes(&encoded).unwrap_err();
        assert!(
            matches!(err, ssz::DecodeError::BytesInvalid(message) if message.contains("matching null positions"))
        );
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn blob_cells_and_proofs_v1_uses_spec_field_name() {
        let blob_cells_and_proofs = BlobCellsAndProofsV1 {
            blob_cells: vec![Some(Cell::ZERO), None],
            proofs: vec![Some(Bytes48::ZERO), None],
        };

        let json = serde_json::to_string(&blob_cells_and_proofs).unwrap();
        assert!(json.contains("\"blob_cells\""));
        assert!(!json.contains("\"blobCells\""));
    }
}
