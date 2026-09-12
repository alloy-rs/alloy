use super::*;

/// V1-V3 blob request container.
///
/// This single-field container starts with a four-byte SSZ offset and is not wire-equivalent to a
/// top-level list.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BlobsV1Request {
    /// Requested versioned blob hashes.
    #[ssz(with = "blob_hashes")]
    pub versioned_hashes: Vec<B256>,
}

/// V2 uses the V1 request schema.
pub type BlobsV2Request = BlobsV1Request;

/// V3 uses the V1 request schema.
pub type BlobsV3Request = BlobsV1Request;

/// V4 blob request container with a packed 128-bit index bitvector.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BlobsV4Request {
    /// Requested versioned blob hashes.
    #[ssz(with = "blob_hashes")]
    pub versioned_hashes: Vec<B256>,
    /// Requested cell indices, SSZ `Bitvector[128]`.
    pub indices_bitarray: B128,
}

/// Blob response entry with explicit outer availability.
///
/// REST-SSZ keeps availability separate from the blob contents instead of using a legacy option.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BlobEntry<T: ssz::Encode + ssz::Decode> {
    /// Whether the complete blob contents are available.
    pub available: bool,
    /// Complete contents, or valid zero-valued contents when unavailable.
    pub contents: T,
}

/// Bounded blob response container.
///
/// The outer container and entry availability match the REST-SSZ blob endpoint contract.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BlobsResponse<T: ssz::Encode + ssz::Decode> {
    /// One response entry per requested hash.
    #[ssz(with = "blob_entries")]
    pub entries: Vec<BlobEntry<T>>,
}

/// V1 whole-blob response.
pub type BlobsV1Response = BlobsResponse<BlobAndProofV1>;

/// V2 all-or-nothing cell-proof response.
pub type BlobsV2Response = BlobsResponse<BlobAndProofV2>;

/// V3 partial cell-proof response.
pub type BlobsV3Response = BlobsResponse<BlobAndProofV2>;

/// V4 partial cell-range response.
pub type BlobsV4Response = BlobsResponse<BlobCellsAndProofs>;

/// Blob cells and proofs with REST-SSZ optional cell positions.
///
/// This uses [`Optional`] (`List[T, 1]`) for per-cell nullability, not Rust [`Option`]'s SSZ
/// union encoding.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode)]
pub struct BlobCellsAndProofs {
    /// Requested blob cells.
    #[ssz(with = "cells")]
    pub blob_cells: Vec<Optional<Cell>>,
    /// KZG proofs for the requested blob cells.
    #[ssz(with = "proofs")]
    pub proofs: Vec<Optional<Bytes48>>,
}

impl ssz::Decode for BlobCellsAndProofs {
    fn is_ssz_fixed_len() -> bool {
        false
    }
    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        #[derive(ssz_derive::Decode)]
        struct Fields {
            #[ssz(with = "cells")]
            blob_cells: Vec<Optional<Cell>>,
            #[ssz(with = "proofs")]
            proofs: Vec<Optional<Bytes48>>,
        }
        let Fields { blob_cells, proofs } = Fields::from_ssz_bytes(bytes)?;
        if blob_cells.len() != proofs.len()
            || blob_cells.iter().zip(&proofs).any(|(cell, proof)| cell.is_some() != proof.is_some())
        {
            return Err(ssz::DecodeError::BytesInvalid(
                "blob cells and proofs must have matching lengths and optional positions".into(),
            ));
        }
        Ok(Self { blob_cells, proofs })
    }
}

fn zero_blob_v1() -> BlobAndProofV1 {
    BlobAndProofV1 { blob: Box::new(Blob::ZERO), proof: Bytes48::ZERO }
}

fn zero_blob_v2() -> BlobAndProofV2 {
    BlobAndProofV2 { blob: Box::new(Blob::ZERO), proofs: Vec::new() }
}

impl<T: ssz::Encode + ssz::Decode> BlobEntry<T> {
    fn from_optional(value: Option<T>, unavailable: impl FnOnce() -> T) -> Self {
        match value {
            Some(contents) => Self { available: true, contents },
            None => Self { available: false, contents: unavailable() },
        }
    }
}

impl<T: ssz::Encode + ssz::Decode> BlobsResponse<T> {
    fn from_values<V>(
        values: Vec<V>,
        convert: impl FnMut(V) -> BlobEntry<T>,
    ) -> Result<Self, ConversionError> {
        if values.len() > MAX_BLOBS_REQUEST {
            return Err(ConversionError::TooManyItems {
                field: "blobs",
                max: MAX_BLOBS_REQUEST,
                actual: values.len(),
            });
        }
        Ok(Self { entries: values.into_iter().map(convert).collect() })
    }
}

impl TryFrom<Vec<Option<BlobAndProofV1>>> for BlobsV1Response {
    type Error = ConversionError;
    fn try_from(value: Vec<Option<BlobAndProofV1>>) -> Result<Self, Self::Error> {
        Self::from_values(value, |value| BlobEntry::from_optional(value, zero_blob_v1))
    }
}

impl TryFrom<Vec<BlobAndProofV2>> for BlobsV2Response {
    type Error = ConversionError;
    fn try_from(value: Vec<BlobAndProofV2>) -> Result<Self, Self::Error> {
        Self::from_values(value, |contents| BlobEntry { available: true, contents })
    }
}

impl TryFrom<Vec<Option<BlobAndProofV2>>> for BlobsV3Response {
    type Error = ConversionError;
    fn try_from(value: Vec<Option<BlobAndProofV2>>) -> Result<Self, Self::Error> {
        Self::from_values(value, |value| BlobEntry::from_optional(value, zero_blob_v2))
    }
}

impl TryFrom<Vec<Option<BlobCellsAndProofsV1>>> for BlobsV4Response {
    type Error = ConversionError;
    fn try_from(value: Vec<Option<BlobCellsAndProofsV1>>) -> Result<Self, Self::Error> {
        Self::from_values(value, |value| {
            BlobEntry::from_optional(
                value.map(|contents| BlobCellsAndProofs {
                    blob_cells: contents.blob_cells.into_iter().map(Optional::from).collect(),
                    proofs: contents.proofs.into_iter().map(Optional::from).collect(),
                }),
                BlobCellsAndProofs::default,
            )
        })
    }
}
