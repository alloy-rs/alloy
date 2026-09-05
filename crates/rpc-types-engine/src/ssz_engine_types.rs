//! Experimental Engine API v2 REST-SSZ wire types.
//!
//! These types intentionally live apart from the legacy JSON-RPC Engine API types because their
//! SSZ encodings are not always wire-compatible. This module contains the shared endpoint
//! containers and fork-specific payload containers from
//! [execution-apis PR #793](https://github.com/ethereum/execution-apis/pull/793), plus the
//! experimental payload-with-witness response type that extends the same REST-SSZ model.

use crate::{
    BlobAndProofV1, BlobAndProofV2, BlobsBundleV1, BlobsBundleV2,
    ExecutionPayloadBodyV1 as LegacyExecutionPayloadBodyV1,
    ExecutionPayloadBodyV2 as LegacyExecutionPayloadBodyV2,
    ExecutionPayloadEnvelopeV2 as LegacyBuiltPayloadShanghai,
    ExecutionPayloadEnvelopeV4 as LegacyBuiltPayloadPrague,
    ExecutionPayloadEnvelopeV5 as LegacyBuiltPayloadOsaka,
    ExecutionPayloadEnvelopeV6 as LegacyBuiltPayloadAmsterdam, ExecutionPayloadFieldV2,
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, ExecutionPayloadV4,
    ForkchoiceState, ForkchoiceUpdated as LegacyForkchoice,
    PayloadAttributes as LegacyPayloadAttributes, PayloadId, PayloadStatus as LegacyPayloadStatus,
    PayloadStatusEnum,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use alloy_eips::{
    eip4844::{Blob, Bytes48},
    eip4895::Withdrawal,
    eip7594::{Cell, CELLS_PER_EXT_BLOB},
    eip7685::Requests,
};
use alloy_primitives::{Address, Bytes, B128, B256, U256};
#[cfg(feature = "ssz")]
use ssz_types::{
    typenum::{U1, U1024, U1048576, U128, U32},
    VariableList,
};

/// Maximum number of blobs in a REST-SSZ blob request or response.
pub const MAX_BLOBS_REQUEST: usize = 128;

/// Maximum UTF-8 byte length of a payload validation error.
pub const MAX_ERROR_BYTES: usize = 1024;

/// Maximum number of entries in a REST-SSZ historical bodies request or response.
pub const MAX_BODIES_REQUEST: usize = 32;

/// Maximum number of items per execution-witness field (`2^20`).
pub const MAX_WITNESS_ITEMS: usize = 1_048_576;

/// Maximum byte length of a single execution-witness item (`2^20`).
pub const MAX_WITNESS_ITEM_BYTES: usize = 1_048_576;

/// Maximum number of trie nodes in an execution witness.
pub const MAX_WITNESS_NODES: usize = MAX_WITNESS_ITEMS;

/// Maximum byte length of a trie node in an execution witness.
pub const MAX_BYTES_PER_WITNESS_NODE: usize = MAX_WITNESS_ITEM_BYTES;

/// Maximum number of contract bytecodes in an execution witness.
pub const MAX_WITNESS_CODES: usize = MAX_WITNESS_ITEMS;

/// Maximum byte length of a contract bytecode in an execution witness.
pub const MAX_BYTES_PER_WITNESS_CODE: usize = MAX_WITNESS_ITEM_BYTES;

/// Maximum number of RLP-encoded headers in an execution witness.
pub const MAX_WITNESS_HEADERS: usize = MAX_WITNESS_ITEMS;

/// Maximum byte length of an RLP-encoded header in an execution witness.
pub const MAX_BYTES_PER_WITNESS_HEADER: usize = MAX_WITNESS_ITEM_BYTES;

type ErrorBytes = VariableList<u8, U1024>;

/// An Engine API v2 SSZ optional encoded as `List[T, 1]`.
///
/// This differs from [`Option<T>`]'s `ethereum_ssz` encoding, which uses an SSZ union.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Optional<T>(VariableList<T, U1>);

impl<T> Optional<T> {
    /// Creates an absent optional.
    pub fn none() -> Self {
        Self(VariableList::empty())
    }

    /// Creates a present optional.
    pub fn some(value: T) -> Self {
        Self(VariableList::new(vec![value]).expect("one value fits Optional"))
    }

    /// Returns the contained value, if present.
    pub fn as_ref(&self) -> Option<&T> {
        self.0.first()
    }

    /// Returns true if no value is present.
    pub fn is_none(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns true if a value is present.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Converts into a Rust optional.
    pub fn into_option(self) -> Option<T> {
        Vec::from(self.0).pop()
    }
}

impl<T> From<Option<T>> for Optional<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(Self::none, Self::some)
    }
}

impl<T> From<Optional<T>> for Option<T> {
    fn from(value: Optional<T>) -> Self {
        value.into_option()
    }
}

impl<T: ssz::Encode> ssz::Encode for Optional<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        self.0.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.0.ssz_append(buf);
    }
}

impl<T: ssz::Decode + 'static> ssz::Decode for Optional<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        VariableList::from_ssz_bytes(bytes).map(Self)
    }
}

/// Engine API v2 REST-SSZ payload status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadStatus {
    /// Payload validation status.
    pub status: PayloadStatusEnum,
    /// Most recent valid block hash.
    pub latest_valid_hash: Optional<B256>,
}

const fn status_code(status: &PayloadStatusEnum) -> u8 {
    match status {
        PayloadStatusEnum::Valid => 0,
        PayloadStatusEnum::Invalid { .. } => 1,
        PayloadStatusEnum::Syncing => 2,
        PayloadStatusEnum::Accepted => 3,
    }
}

fn status_from_code(code: u8) -> Result<PayloadStatusEnum, ssz::DecodeError> {
    match code {
        0 => Ok(PayloadStatusEnum::Valid),
        1 => Ok(PayloadStatusEnum::Invalid { validation_error: String::new() }),
        2 => Ok(PayloadStatusEnum::Syncing),
        3 => Ok(PayloadStatusEnum::Accepted),
        _ => Err(ssz::DecodeError::BytesInvalid("unknown payload status code".into())),
    }
}

fn validation_error(status: &PayloadStatusEnum) -> Result<Optional<ErrorBytes>, ssz_types::Error> {
    match status {
        PayloadStatusEnum::Invalid { validation_error } => {
            Ok(Optional::some(ErrorBytes::new(validation_error.as_bytes().to_vec())?))
        }
        _ => Ok(Optional::none()),
    }
}

#[cfg(feature = "ssz")]
impl ssz::Encode for PayloadStatus {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        let validation_error =
            validation_error(&self.status).expect("PayloadStatus validation error must be bounded");
        1 + ssz::BYTES_PER_LENGTH_OFFSET * 2
            + self.latest_valid_hash.ssz_bytes_len()
            + validation_error.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let validation_error =
            validation_error(&self.status).expect("PayloadStatus validation error must be bounded");
        let mut encoder = ssz::SszEncoder::container(buf, 1 + ssz::BYTES_PER_LENGTH_OFFSET * 2);
        encoder.append(&status_code(&self.status));
        encoder.append(&self.latest_valid_hash);
        encoder.append(&validation_error);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for PayloadStatus {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<u8>()?;
        builder.register_type::<Optional<B256>>()?;
        builder.register_type::<Optional<ErrorBytes>>()?;
        let mut decoder = builder.build()?;
        let mut status = status_from_code(decoder.decode_next()?)?;
        let latest_valid_hash = decoder.decode_next()?;
        let validation_error: Optional<ErrorBytes> = decoder.decode_next()?;
        if let PayloadStatusEnum::Invalid { validation_error: error } = &mut status {
            *error = match validation_error.as_ref() {
                Some(error) => String::from_utf8(error.to_vec())
                    .map_err(|err| ssz::DecodeError::BytesInvalid(err.to_string()))?,
                None => String::new(),
            };
        } else if validation_error.is_some() {
            return Err(ssz::DecodeError::BytesInvalid(
                "validation error is only valid for INVALID status".into(),
            ));
        }
        Ok(Self { status, latest_valid_hash })
    }
}

/// Error converting legacy Engine API values into v2 REST-SSZ values.
#[derive(Clone, Debug, PartialEq)]
pub enum ConversionError {
    /// A bounded list or byte string exceeded its limit.
    Bounds(ssz_types::Error),
    /// `ACCEPTED` is not permitted in a forkchoice response.
    AcceptedForkchoice,
}

impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bounds(err) => err.fmt(f),
            Self::AcceptedForkchoice => {
                f.write_str("ACCEPTED is not valid in a forkchoice response")
            }
        }
    }
}

impl core::error::Error for ConversionError {}

impl From<ssz_types::Error> for ConversionError {
    fn from(value: ssz_types::Error) -> Self {
        Self::Bounds(value)
    }
}

impl TryFrom<LegacyPayloadStatus> for PayloadStatus {
    type Error = ConversionError;

    fn try_from(value: LegacyPayloadStatus) -> Result<Self, Self::Error> {
        validation_error(&value.status)?;
        Ok(Self { status: value.status, latest_valid_hash: value.latest_valid_hash.into() })
    }
}

impl From<PayloadStatus> for LegacyPayloadStatus {
    fn from(value: PayloadStatus) -> Self {
        Self { status: value.status, latest_valid_hash: value.latest_valid_hash.into() }
    }
}

/// Engine API v2 REST-SSZ forkchoice update response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkchoiceUpdateResponse {
    /// Restricted payload status; `ACCEPTED` is invalid here.
    pub payload_status: PayloadStatus,
    /// Opaque server-assigned payload identifier.
    pub payload_id: Optional<PayloadId>,
}

#[cfg(feature = "ssz")]
impl ssz::Encode for ForkchoiceUpdateResponse {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        ssz::BYTES_PER_LENGTH_OFFSET * 2
            + self.payload_status.ssz_bytes_len()
            + self.payload_id.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = ssz::SszEncoder::container(buf, ssz::BYTES_PER_LENGTH_OFFSET * 2);
        encoder.append(&self.payload_status);
        encoder.append(&self.payload_id);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for ForkchoiceUpdateResponse {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<PayloadStatus>()?;
        builder.register_type::<Optional<PayloadId>>()?;
        let mut decoder = builder.build()?;
        let response =
            Self { payload_status: decoder.decode_next()?, payload_id: decoder.decode_next()? };
        if matches!(response.payload_status.status, PayloadStatusEnum::Accepted) {
            return Err(ssz::DecodeError::BytesInvalid(
                "ACCEPTED is not valid in a forkchoice response".into(),
            ));
        }
        Ok(response)
    }
}

impl TryFrom<LegacyForkchoice> for ForkchoiceUpdateResponse {
    type Error = ConversionError;

    fn try_from(value: LegacyForkchoice) -> Result<Self, Self::Error> {
        let payload_status = PayloadStatus::try_from(value.payload_status)?;
        if matches!(payload_status.status, PayloadStatusEnum::Accepted) {
            return Err(ConversionError::AcceptedForkchoice);
        }
        Ok(Self { payload_status, payload_id: value.payload_id.into() })
    }
}

impl From<ForkchoiceUpdateResponse> for LegacyForkchoice {
    fn from(value: ForkchoiceUpdateResponse) -> Self {
        Self { payload_status: value.payload_status.into(), payload_id: value.payload_id.into() }
    }
}

/// A bounded trie-node byte list in an [`ExecutionWitnessV1`].
pub type WitnessNodeV1 = VariableList<u8, U1048576>;

/// A bounded contract-code byte list in an [`ExecutionWitnessV1`].
pub type WitnessCodeV1 = VariableList<u8, U1048576>;

/// A bounded RLP-encoded header byte list in an [`ExecutionWitnessV1`].
pub type WitnessHeaderV1 = VariableList<u8, U1048576>;

/// Canonical execution witness for `POST /{fork}/payloads/witness`.
///
/// `state` and `codes` are produced in lexicographic ascending byte order. `headers` are
/// RLP-encoded and ordered by ascending block number; consecutive headers must be parent-linked.
/// These ordering rules are producer-side requirements from the execution-specs witness builder.
///
/// This is a REST-SSZ wire container, not the JSON-RPC debug witness shape.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionWitnessV1 {
    /// Hashed trie-node preimages required during execution and state-root recomputation.
    pub state: VariableList<WitnessNodeV1, U1048576>,
    /// Contract bytecode preimages created or accessed during execution.
    pub codes: VariableList<WitnessCodeV1, U1048576>,
    /// RLP-encoded ancestor headers used for pre-state and `BLOCKHASH` correctness proofs.
    pub headers: VariableList<WitnessHeaderV1, U1048576>,
}

/// Canonical execution witness for `POST /{fork}/payloads/witness`.
pub type ExecutionWitness = ExecutionWitnessV1;

/// REST-SSZ response for `POST /{fork}/payloads/witness`.
///
/// This models only the response body. Endpoint routing, request handling, and HTTP error mapping
/// belong to the caller. The witness uses the Engine REST-SSZ `Optional[T]` encoding from
/// execution-apis PR #793 and is present only when the payload status is `VALID`.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode)]
pub struct PayloadStatusWithWitness {
    /// Result of processing the submitted payload.
    pub payload_status: PayloadStatus,
    /// Execution witness produced for a valid payload.
    pub witness: Optional<ExecutionWitnessV1>,
}

impl PayloadStatusWithWitness {
    /// Creates a response, converting the witness into the REST-SSZ `Optional[T]` representation.
    pub fn new(payload_status: PayloadStatus, witness: Option<ExecutionWitnessV1>) -> Self {
        Self { payload_status, witness: witness.into() }
    }
}

/// Backwards-compatible alias for the experimental witness response name.
pub type NewPayloadWithWitnessResponseV1 = PayloadStatusWithWitness;

impl TryFrom<alloy_rpc_types_debug::ExecutionWitness> for ExecutionWitnessV1 {
    type Error = ConversionError;

    fn try_from(value: alloy_rpc_types_debug::ExecutionWitness) -> Result<Self, Self::Error> {
        let state = value
            .state
            .into_iter()
            .map(|bytes| WitnessNodeV1::new(bytes.to_vec()))
            .collect::<Result<Vec<_>, _>>()?;
        let codes = value
            .codes
            .into_iter()
            .map(|bytes| WitnessCodeV1::new(bytes.to_vec()))
            .collect::<Result<Vec<_>, _>>()?;
        let headers = value
            .headers
            .into_iter()
            .map(|bytes| WitnessHeaderV1::new(bytes.to_vec()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            state: VariableList::new(state)?,
            codes: VariableList::new(codes)?,
            headers: VariableList::new(headers)?,
        })
    }
}

impl ssz::Decode for PayloadStatusWithWitness {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<PayloadStatus>()?;
        builder.register_type::<Optional<ExecutionWitnessV1>>()?;
        let mut decoder = builder.build()?;
        let response =
            Self { payload_status: decoder.decode_next()?, witness: decoder.decode_next()? };
        if response.witness.is_some()
            && !matches!(response.payload_status.status, PayloadStatusEnum::Valid)
        {
            return Err(ssz::DecodeError::BytesInvalid(
                "execution witness is only valid for VALID payload status".into(),
            ));
        }
        Ok(response)
    }
}

/// V1-V3 blob request container.
///
/// This single-field container starts with a four-byte SSZ offset and is not wire-equivalent to a
/// top-level list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobsV1Request {
    /// Requested versioned blob hashes.
    pub versioned_hashes: VariableList<B256, U128>,
}

/// V2 uses the V1 request schema.
pub type BlobsV2Request = BlobsV1Request;
/// V3 uses the V1 request schema.
pub type BlobsV3Request = BlobsV1Request;

#[cfg(feature = "ssz")]
impl ssz::Encode for BlobsV1Request {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        ssz::BYTES_PER_LENGTH_OFFSET + self.versioned_hashes.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = ssz::SszEncoder::container(buf, ssz::BYTES_PER_LENGTH_OFFSET);
        encoder.append(&self.versioned_hashes);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobsV1Request {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<VariableList<B256, U128>>()?;
        let mut decoder = builder.build()?;
        Ok(Self { versioned_hashes: decoder.decode_next()? })
    }
}

/// V4 blob request container with a packed 128-bit index bitvector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobsV4Request {
    /// Requested versioned blob hashes.
    pub versioned_hashes: VariableList<B256, U128>,
    /// Requested cell indices, SSZ `Bitvector[128]`.
    pub indices_bitarray: B128,
}

#[cfg(feature = "ssz")]
impl ssz::Encode for BlobsV4Request {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        ssz::BYTES_PER_LENGTH_OFFSET + 16 + self.versioned_hashes.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = ssz::SszEncoder::container(buf, ssz::BYTES_PER_LENGTH_OFFSET + 16);
        encoder.append(&self.versioned_hashes);
        encoder.append(&self.indices_bitarray);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobsV4Request {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<VariableList<B256, U128>>()?;
        builder.register_type::<B128>()?;
        let mut decoder = builder.build()?;
        Ok(Self {
            versioned_hashes: decoder.decode_next()?,
            indices_bitarray: decoder.decode_next()?,
        })
    }
}

/// Blob response entry with explicit outer availability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobEntry<T> {
    /// Whether the complete blob contents are available.
    pub available: bool,
    /// Complete contents, or valid zero-valued contents when unavailable.
    pub contents: T,
}

#[cfg(feature = "ssz")]
impl<T: ssz::Encode> ssz::Encode for BlobEntry<T> {
    fn is_ssz_fixed_len() -> bool {
        T::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        if T::is_ssz_fixed_len() {
            1 + T::ssz_fixed_len()
        } else {
            1 + ssz::BYTES_PER_LENGTH_OFFSET
        }
    }

    fn ssz_bytes_len(&self) -> usize {
        1 + if T::is_ssz_fixed_len() {
            self.contents.ssz_bytes_len()
        } else {
            ssz::BYTES_PER_LENGTH_OFFSET + self.contents.ssz_bytes_len()
        }
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let fixed_len = 1 + if T::is_ssz_fixed_len() {
            T::ssz_fixed_len()
        } else {
            ssz::BYTES_PER_LENGTH_OFFSET
        };
        let mut encoder = ssz::SszEncoder::container(buf, fixed_len);
        encoder.append(&self.available);
        encoder.append(&self.contents);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl<T: ssz::Decode> ssz::Decode for BlobEntry<T> {
    fn is_ssz_fixed_len() -> bool {
        T::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        if T::is_ssz_fixed_len() {
            1 + T::ssz_fixed_len()
        } else {
            1 + ssz::BYTES_PER_LENGTH_OFFSET
        }
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<bool>()?;
        builder.register_type::<T>()?;
        let mut decoder = builder.build()?;
        Ok(Self { available: decoder.decode_next()?, contents: decoder.decode_next()? })
    }
}

/// Bounded blob response container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobsResponse<T> {
    /// One response entry per requested hash.
    pub entries: VariableList<BlobEntry<T>, U128>,
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
    pub blob_cells: VariableList<Optional<Cell>, U128>,
    /// KZG proofs for the requested blob cells.
    pub proofs: VariableList<Optional<Bytes48>, U128>,
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobCellsAndProofs {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        #[derive(ssz_derive::Decode)]
        struct Raw {
            blob_cells: VariableList<Optional<Cell>, U128>,
            proofs: VariableList<Optional<Bytes48>, U128>,
        }

        let raw = Raw::from_ssz_bytes(bytes)?;

        if raw.blob_cells.len() > CELLS_PER_EXT_BLOB {
            return Err(ssz::DecodeError::BytesInvalid(format!(
                "Invalid BlobCellsAndProofs: expected at most {CELLS_PER_EXT_BLOB} blob cells, got {}",
                raw.blob_cells.len()
            )));
        }

        if raw.blob_cells.len() != raw.proofs.len() {
            return Err(ssz::DecodeError::BytesInvalid(format!(
                "Invalid BlobCellsAndProofs: blob_cells length {} does not match proofs length {}",
                raw.blob_cells.len(),
                raw.proofs.len()
            )));
        }

        if raw
            .blob_cells
            .iter()
            .zip(raw.proofs.iter())
            .any(|(cell, proof)| cell.is_some() != proof.is_some())
        {
            return Err(ssz::DecodeError::BytesInvalid(
                "Invalid BlobCellsAndProofs: blob_cells and proofs must have matching optional positions".into(),
            ));
        }

        Ok(Self { blob_cells: raw.blob_cells, proofs: raw.proofs })
    }
}

#[cfg(feature = "ssz")]
impl<T: ssz::Encode> ssz::Encode for BlobsResponse<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        ssz::BYTES_PER_LENGTH_OFFSET + self.entries.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = ssz::SszEncoder::container(buf, ssz::BYTES_PER_LENGTH_OFFSET);
        encoder.append(&self.entries);
        encoder.finalize();
    }
}

#[cfg(feature = "ssz")]
impl<T: ssz::Decode + 'static> ssz::Decode for BlobsResponse<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<VariableList<BlobEntry<T>, U128>>()?;
        let mut decoder = builder.build()?;
        Ok(Self { entries: decoder.decode_next()? })
    }
}

fn zero_blob_v1() -> BlobAndProofV1 {
    BlobAndProofV1 { blob: Box::new(Blob::ZERO), proof: Bytes48::ZERO }
}

fn zero_blob_v2() -> BlobAndProofV2 {
    BlobAndProofV2 { blob: Box::new(Blob::ZERO), proofs: Vec::new() }
}

impl TryFrom<Vec<Option<BlobAndProofV1>>> for BlobsV1Response {
    type Error = ssz_types::Error;

    fn try_from(value: Vec<Option<BlobAndProofV1>>) -> Result<Self, Self::Error> {
        let entries = value
            .into_iter()
            .map(|value| match value {
                Some(contents) => BlobEntry { available: true, contents },
                None => BlobEntry { available: false, contents: zero_blob_v1() },
            })
            .collect::<Vec<_>>()
            .try_into()?;
        Ok(Self { entries })
    }
}

impl TryFrom<Vec<BlobAndProofV2>> for BlobsV2Response {
    type Error = ssz_types::Error;

    fn try_from(value: Vec<BlobAndProofV2>) -> Result<Self, Self::Error> {
        let entries = value
            .into_iter()
            .map(|contents| BlobEntry { available: true, contents })
            .collect::<Vec<_>>()
            .try_into()?;
        Ok(Self { entries })
    }
}

impl TryFrom<Vec<Option<BlobAndProofV2>>> for BlobsV3Response {
    type Error = ssz_types::Error;

    fn try_from(value: Vec<Option<BlobAndProofV2>>) -> Result<Self, Self::Error> {
        let entries = value
            .into_iter()
            .map(|value| match value {
                Some(contents) => BlobEntry { available: true, contents },
                None => BlobEntry { available: false, contents: zero_blob_v2() },
            })
            .collect::<Vec<_>>()
            .try_into()?;
        Ok(Self { entries })
    }
}

impl TryFrom<Vec<Option<BlobCellsAndProofs>>> for BlobsV4Response {
    type Error = ssz_types::Error;

    fn try_from(value: Vec<Option<BlobCellsAndProofs>>) -> Result<Self, Self::Error> {
        let entries = value
            .into_iter()
            .map(|value| match value {
                Some(contents) => BlobEntry { available: true, contents },
                None => BlobEntry { available: false, contents: BlobCellsAndProofs::default() },
            })
            .collect::<Vec<_>>()
            .try_into()?;
        Ok(Self { entries })
    }
}

// Fork-specific payload containers.

/// Paris execution payload.
pub type ExecutionPayloadParis = ExecutionPayloadV1;
/// Shanghai execution payload.
pub type ExecutionPayloadShanghai = ExecutionPayloadV2;
/// Cancun execution payload.
pub type ExecutionPayloadCancun = ExecutionPayloadV3;
/// Prague execution payload.
pub type ExecutionPayloadPrague = ExecutionPayloadV3;
/// Osaka execution payload.
pub type ExecutionPayloadOsaka = ExecutionPayloadV3;
/// Amsterdam execution payload.
pub type ExecutionPayloadAmsterdam = ExecutionPayloadV4;

/// Paris payload attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesParis {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
}

/// Shanghai payload attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesShanghai {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals to include in the payload.
    pub withdrawals: Vec<Withdrawal>,
}

/// Cancun payload attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesCancun {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals to include in the payload.
    pub withdrawals: Vec<Withdrawal>,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
}

/// Prague uses the Cancun payload-attributes schema.
pub type PayloadAttributesPrague = PayloadAttributesCancun;
/// Osaka uses the Cancun payload-attributes schema.
pub type PayloadAttributesOsaka = PayloadAttributesCancun;

/// Amsterdam payload attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesAmsterdam {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals to include in the payload.
    pub withdrawals: Vec<Withdrawal>,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// Consensus-layer slot number.
    pub slot_number: u64,
    /// Target gas limit.
    pub target_gas_limit: u64,
}

/// Error converting legacy cross-fork payload attributes into a fork-specific SSZ container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadAttributesConversionError {
    /// A field required by the selected fork is absent.
    MissingField(&'static str),
    /// A field from a later fork is populated and would be lost.
    UnexpectedField(&'static str),
}

impl core::fmt::Display for PayloadAttributesConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingField(field) => {
                write!(f, "missing required payload attributes field: {field}")
            }
            Self::UnexpectedField(field) => {
                write!(f, "unexpected later-fork payload attributes field: {field}")
            }
        }
    }
}

impl core::error::Error for PayloadAttributesConversionError {}

const fn ensure_absent<T>(
    value: &Option<T>,
    field: &'static str,
) -> Result<(), PayloadAttributesConversionError> {
    if value.is_some() {
        Err(PayloadAttributesConversionError::UnexpectedField(field))
    } else {
        Ok(())
    }
}

fn require<T>(
    value: Option<T>,
    field: &'static str,
) -> Result<T, PayloadAttributesConversionError> {
    value.ok_or(PayloadAttributesConversionError::MissingField(field))
}

impl From<PayloadAttributesParis> for LegacyPayloadAttributes {
    fn from(value: PayloadAttributesParis) -> Self {
        Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: None,
            parent_beacon_block_root: None,
            slot_number: None,
            target_gas_limit: None,
        }
    }
}

impl TryFrom<LegacyPayloadAttributes> for PayloadAttributesParis {
    type Error = PayloadAttributesConversionError;

    fn try_from(value: LegacyPayloadAttributes) -> Result<Self, Self::Error> {
        ensure_absent(&value.withdrawals, "withdrawals")?;
        ensure_absent(&value.parent_beacon_block_root, "parent_beacon_block_root")?;
        ensure_absent(&value.slot_number, "slot_number")?;
        ensure_absent(&value.target_gas_limit, "target_gas_limit")?;
        Ok(Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
        })
    }
}

impl From<PayloadAttributesShanghai> for LegacyPayloadAttributes {
    fn from(value: PayloadAttributesShanghai) -> Self {
        Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: Some(value.withdrawals),
            parent_beacon_block_root: None,
            slot_number: None,
            target_gas_limit: None,
        }
    }
}

impl TryFrom<LegacyPayloadAttributes> for PayloadAttributesShanghai {
    type Error = PayloadAttributesConversionError;

    fn try_from(value: LegacyPayloadAttributes) -> Result<Self, Self::Error> {
        ensure_absent(&value.parent_beacon_block_root, "parent_beacon_block_root")?;
        ensure_absent(&value.slot_number, "slot_number")?;
        ensure_absent(&value.target_gas_limit, "target_gas_limit")?;
        Ok(Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: require(value.withdrawals, "withdrawals")?,
        })
    }
}

impl From<PayloadAttributesCancun> for LegacyPayloadAttributes {
    fn from(value: PayloadAttributesCancun) -> Self {
        Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: Some(value.withdrawals),
            parent_beacon_block_root: Some(value.parent_beacon_block_root),
            slot_number: None,
            target_gas_limit: None,
        }
    }
}

impl TryFrom<LegacyPayloadAttributes> for PayloadAttributesCancun {
    type Error = PayloadAttributesConversionError;

    fn try_from(value: LegacyPayloadAttributes) -> Result<Self, Self::Error> {
        ensure_absent(&value.slot_number, "slot_number")?;
        ensure_absent(&value.target_gas_limit, "target_gas_limit")?;
        Ok(Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: require(value.withdrawals, "withdrawals")?,
            parent_beacon_block_root: require(
                value.parent_beacon_block_root,
                "parent_beacon_block_root",
            )?,
        })
    }
}

impl From<PayloadAttributesAmsterdam> for LegacyPayloadAttributes {
    fn from(value: PayloadAttributesAmsterdam) -> Self {
        Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: Some(value.withdrawals),
            parent_beacon_block_root: Some(value.parent_beacon_block_root),
            slot_number: Some(value.slot_number),
            target_gas_limit: Some(value.target_gas_limit),
        }
    }
}

impl TryFrom<LegacyPayloadAttributes> for PayloadAttributesAmsterdam {
    type Error = PayloadAttributesConversionError;

    fn try_from(value: LegacyPayloadAttributes) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: require(value.withdrawals, "withdrawals")?,
            parent_beacon_block_root: require(
                value.parent_beacon_block_root,
                "parent_beacon_block_root",
            )?,
            slot_number: require(value.slot_number, "slot_number")?,
            target_gas_limit: require(value.target_gas_limit, "target_gas_limit")?,
        })
    }
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Paris.
///
/// Unlike the legacy `engine_getPayloadV1` response, this includes the expected block value.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadParis {
    /// Execution payload V1.
    pub payload: ExecutionPayloadParis,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Shanghai.
///
/// This follows the legacy `engine_getPayloadV2` payload-build response shape: execution payload
/// plus block value only. `should_override_builder` starts at Cancun.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadShanghai {
    /// Execution payload V2.
    pub payload: ExecutionPayloadShanghai,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
}

/// Engine API v2 REST-SSZ payload-build response for Cancun.
///
/// This is wire-compatible with the legacy `engine_getPayloadV3` response envelope,
/// [`crate::ExecutionPayloadEnvelopeV3`].
pub type BuiltPayloadCancun = crate::ExecutionPayloadEnvelopeV3;

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Prague.
///
/// Unlike the legacy [`crate::ExecutionPayloadEnvelopeV4`], `execution_requests` precedes
/// `should_override_builder` in the normative SSZ field order.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadPrague {
    /// Execution payload V3.
    pub payload: ExecutionPayloadPrague,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
    /// The blobs, commitments, and proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV1,
    /// A list of opaque [EIP-7685][eip7685] requests.
    ///
    /// [eip7685]: https://eips.ethereum.org/EIPS/eip-7685
    pub execution_requests: Requests,
    /// A suggestion from the execution layer whether this payload should be used instead of an
    /// externally provided one.
    pub should_override_builder: bool,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Osaka.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadOsaka {
    /// Execution payload V3.
    pub payload: ExecutionPayloadOsaka,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
    /// The blobs, commitments, and EIP-7594 cell proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV2,
    /// A list of opaque [EIP-7685][eip7685] requests.
    ///
    /// [eip7685]: https://eips.ethereum.org/EIPS/eip-7685
    pub execution_requests: Requests,
    /// A suggestion from the execution layer whether this payload should be used instead of an
    /// externally provided one.
    pub should_override_builder: bool,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Amsterdam.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadAmsterdam {
    /// Execution payload V4.
    pub payload: ExecutionPayloadAmsterdam,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
    /// The blobs, commitments, and EIP-7594 cell proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV2,
    /// A list of opaque [EIP-7685][eip7685] requests.
    ///
    /// [eip7685]: https://eips.ethereum.org/EIPS/eip-7685
    pub execution_requests: Requests,
    /// A suggestion from the execution layer whether this payload should be used instead of an
    /// externally provided one.
    pub should_override_builder: bool,
}

/// Error converting legacy payload-build envelopes into fork-specific REST-SSZ containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltPayloadConversionError {
    /// The legacy envelope carried an execution payload from a different fork.
    UnexpectedPayloadFork(&'static str),
}

impl core::fmt::Display for BuiltPayloadConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedPayloadFork(fork) => {
                write!(f, "unexpected execution payload fork: {fork}")
            }
        }
    }
}

impl core::error::Error for BuiltPayloadConversionError {}

impl From<BuiltPayloadShanghai> for LegacyBuiltPayloadShanghai {
    fn from(value: BuiltPayloadShanghai) -> Self {
        Self {
            execution_payload: ExecutionPayloadFieldV2::V2(value.payload),
            block_value: value.block_value,
        }
    }
}

impl TryFrom<LegacyBuiltPayloadShanghai> for BuiltPayloadShanghai {
    type Error = BuiltPayloadConversionError;

    fn try_from(value: LegacyBuiltPayloadShanghai) -> Result<Self, Self::Error> {
        match value.execution_payload {
            ExecutionPayloadFieldV2::V2(payload) => {
                Ok(Self { payload, block_value: value.block_value })
            }
            ExecutionPayloadFieldV2::V1(_) => {
                Err(BuiltPayloadConversionError::UnexpectedPayloadFork("Paris"))
            }
        }
    }
}

impl From<LegacyBuiltPayloadPrague> for BuiltPayloadPrague {
    fn from(value: LegacyBuiltPayloadPrague) -> Self {
        Self {
            payload: value.envelope_inner.execution_payload,
            block_value: value.envelope_inner.block_value,
            blobs_bundle: value.envelope_inner.blobs_bundle,
            execution_requests: value.execution_requests,
            should_override_builder: value.envelope_inner.should_override_builder,
        }
    }
}

impl From<BuiltPayloadPrague> for LegacyBuiltPayloadPrague {
    fn from(value: BuiltPayloadPrague) -> Self {
        Self {
            envelope_inner: crate::ExecutionPayloadEnvelopeV3 {
                execution_payload: value.payload,
                block_value: value.block_value,
                blobs_bundle: value.blobs_bundle,
                should_override_builder: value.should_override_builder,
            },
            execution_requests: value.execution_requests,
        }
    }
}

impl From<LegacyBuiltPayloadOsaka> for BuiltPayloadOsaka {
    fn from(value: LegacyBuiltPayloadOsaka) -> Self {
        Self {
            payload: value.execution_payload,
            block_value: value.block_value,
            blobs_bundle: value.blobs_bundle,
            execution_requests: value.execution_requests,
            should_override_builder: value.should_override_builder,
        }
    }
}

impl From<BuiltPayloadOsaka> for LegacyBuiltPayloadOsaka {
    fn from(value: BuiltPayloadOsaka) -> Self {
        Self {
            execution_payload: value.payload,
            block_value: value.block_value,
            blobs_bundle: value.blobs_bundle,
            should_override_builder: value.should_override_builder,
            execution_requests: value.execution_requests,
        }
    }
}

impl From<LegacyBuiltPayloadAmsterdam> for BuiltPayloadAmsterdam {
    fn from(value: LegacyBuiltPayloadAmsterdam) -> Self {
        Self {
            payload: value.execution_payload,
            block_value: value.block_value,
            blobs_bundle: value.blobs_bundle,
            execution_requests: value.execution_requests,
            should_override_builder: value.should_override_builder,
        }
    }
}

impl From<BuiltPayloadAmsterdam> for LegacyBuiltPayloadAmsterdam {
    fn from(value: BuiltPayloadAmsterdam) -> Self {
        Self {
            execution_payload: value.payload,
            block_value: value.block_value,
            blobs_bundle: value.blobs_bundle,
            should_override_builder: value.should_override_builder,
            execution_requests: value.execution_requests,
        }
    }
}

/// Paris payload-submission request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeParis {
    /// Submitted execution payload.
    pub payload: ExecutionPayloadParis,
}

/// Shanghai payload-submission request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeShanghai {
    /// Submitted execution payload.
    pub payload: ExecutionPayloadShanghai,
}

/// Cancun payload-submission request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeCancun {
    /// Submitted execution payload.
    pub payload: ExecutionPayloadCancun,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
}

/// Prague payload-submission request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopePrague {
    /// Submitted execution payload.
    pub payload: ExecutionPayloadPrague,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// EIP-7685 execution requests.
    pub execution_requests: Requests,
}

/// Osaka payload-submission request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeOsaka {
    /// Submitted execution payload.
    pub payload: ExecutionPayloadOsaka,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// EIP-7685 execution requests.
    pub execution_requests: Requests,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-submission request for Amsterdam.
///
/// This is distinct from the legacy [`crate::ExecutionPayloadEnvelopeV6`], which is the
/// `engine_getPayloadV6` response.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeAmsterdam {
    /// Submitted execution payload.
    pub payload: ExecutionPayloadAmsterdam,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// EIP-7685 execution requests.
    pub execution_requests: Requests,
}

impl From<ExecutionPayloadParis> for ExecutionPayloadEnvelopeParis {
    fn from(payload: ExecutionPayloadParis) -> Self {
        Self { payload }
    }
}

impl From<ExecutionPayloadShanghai> for ExecutionPayloadEnvelopeShanghai {
    fn from(payload: ExecutionPayloadShanghai) -> Self {
        Self { payload }
    }
}

impl From<(ExecutionPayloadCancun, B256)> for ExecutionPayloadEnvelopeCancun {
    fn from((payload, parent_beacon_block_root): (ExecutionPayloadCancun, B256)) -> Self {
        Self { payload, parent_beacon_block_root }
    }
}

impl From<(ExecutionPayloadPrague, B256, Requests)> for ExecutionPayloadEnvelopePrague {
    fn from(
        (payload, parent_beacon_block_root, execution_requests): (
            ExecutionPayloadPrague,
            B256,
            Requests,
        ),
    ) -> Self {
        Self { payload, parent_beacon_block_root, execution_requests }
    }
}

impl From<(ExecutionPayloadOsaka, B256, Requests)> for ExecutionPayloadEnvelopeOsaka {
    fn from(
        (payload, parent_beacon_block_root, execution_requests): (
            ExecutionPayloadOsaka,
            B256,
            Requests,
        ),
    ) -> Self {
        Self { payload, parent_beacon_block_root, execution_requests }
    }
}

impl From<(ExecutionPayloadAmsterdam, B256, Requests)> for ExecutionPayloadEnvelopeAmsterdam {
    fn from(
        (payload, parent_beacon_block_root, execution_requests): (
            ExecutionPayloadAmsterdam,
            B256,
            Requests,
        ),
    ) -> Self {
        Self { payload, parent_beacon_block_root, execution_requests }
    }
}

/// Paris forkchoice-update request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ForkchoiceUpdateParis {
    /// Current forkchoice state.
    pub forkchoice_state: ForkchoiceState,
    /// Optional Paris payload attributes.
    pub payload_attributes: Optional<PayloadAttributesParis>,
}

/// Shanghai forkchoice-update request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ForkchoiceUpdateShanghai {
    /// Current forkchoice state.
    pub forkchoice_state: ForkchoiceState,
    /// Optional Shanghai payload attributes.
    pub payload_attributes: Optional<PayloadAttributesShanghai>,
}

/// Cancun forkchoice-update request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ForkchoiceUpdateCancun {
    /// Current forkchoice state.
    pub forkchoice_state: ForkchoiceState,
    /// Optional Cancun payload attributes.
    pub payload_attributes: Optional<PayloadAttributesCancun>,
}

/// Prague forkchoice-update request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ForkchoiceUpdatePrague {
    /// Current forkchoice state.
    pub forkchoice_state: ForkchoiceState,
    /// Optional Prague payload attributes.
    pub payload_attributes: Optional<PayloadAttributesPrague>,
}

/// Osaka forkchoice-update request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ForkchoiceUpdateOsaka {
    /// Current forkchoice state.
    pub forkchoice_state: ForkchoiceState,
    /// Optional Osaka payload attributes.
    pub payload_attributes: Optional<PayloadAttributesOsaka>,
}

/// Amsterdam forkchoice-update request.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ForkchoiceUpdateAmsterdam {
    /// Current forkchoice state.
    pub forkchoice_state: ForkchoiceState,
    /// Optional Amsterdam payload attributes.
    pub payload_attributes: Optional<PayloadAttributesAmsterdam>,
    /// Optional `Bitvector[128]` custody-column selection.
    pub custody_columns: Optional<B128>,
}

/// Fork-specific execution payload body for Paris.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadBodyParis {
    /// Enveloped encoded transactions.
    pub transactions: Vec<Bytes>,
}

/// Fork-specific execution payload body for Shanghai.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadBodyShanghai {
    /// Enveloped encoded transactions.
    pub transactions: Vec<Bytes>,
    /// Withdrawals included in the block.
    pub withdrawals: Vec<Withdrawal>,
}

/// Cancun uses the Shanghai execution-payload-body schema.
pub type ExecutionPayloadBodyCancun = ExecutionPayloadBodyShanghai;
/// Prague uses the Shanghai execution-payload-body schema.
pub type ExecutionPayloadBodyPrague = ExecutionPayloadBodyShanghai;
/// Osaka uses the Shanghai execution-payload-body schema.
pub type ExecutionPayloadBodyOsaka = ExecutionPayloadBodyShanghai;

/// Fork-specific execution payload body for Amsterdam.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadBodyAmsterdam {
    /// Enveloped encoded transactions.
    pub transactions: Vec<Bytes>,
    /// Withdrawals included in the block.
    pub withdrawals: Vec<Withdrawal>,
    /// RLP-encoded EIP-7928 block access list.
    pub block_access_list: Bytes,
}

/// Error converting legacy cross-fork execution payload bodies into fork-specific containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionPayloadBodyConversionError {
    /// A field required by the selected fork is absent.
    MissingField(&'static str),
    /// A field from a later fork is populated and would be lost.
    UnexpectedField(&'static str),
}

impl core::fmt::Display for ExecutionPayloadBodyConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingField(field) => {
                write!(f, "missing required execution payload body field: {field}")
            }
            Self::UnexpectedField(field) => {
                write!(f, "unexpected later-fork execution payload body field: {field}")
            }
        }
    }
}

impl core::error::Error for ExecutionPayloadBodyConversionError {}

impl From<ExecutionPayloadBodyParis> for LegacyExecutionPayloadBodyV1 {
    fn from(value: ExecutionPayloadBodyParis) -> Self {
        Self { transactions: value.transactions, withdrawals: None }
    }
}

impl TryFrom<LegacyExecutionPayloadBodyV1> for ExecutionPayloadBodyParis {
    type Error = ExecutionPayloadBodyConversionError;

    fn try_from(value: LegacyExecutionPayloadBodyV1) -> Result<Self, Self::Error> {
        if value.withdrawals.is_some() {
            return Err(ExecutionPayloadBodyConversionError::UnexpectedField("withdrawals"));
        }
        Ok(Self { transactions: value.transactions })
    }
}

impl From<ExecutionPayloadBodyShanghai> for LegacyExecutionPayloadBodyV1 {
    fn from(value: ExecutionPayloadBodyShanghai) -> Self {
        Self { transactions: value.transactions, withdrawals: Some(value.withdrawals) }
    }
}

impl TryFrom<LegacyExecutionPayloadBodyV1> for ExecutionPayloadBodyShanghai {
    type Error = ExecutionPayloadBodyConversionError;

    fn try_from(value: LegacyExecutionPayloadBodyV1) -> Result<Self, Self::Error> {
        Ok(Self {
            transactions: value.transactions,
            withdrawals: value
                .withdrawals
                .ok_or(ExecutionPayloadBodyConversionError::MissingField("withdrawals"))?,
        })
    }
}

impl From<ExecutionPayloadBodyAmsterdam> for LegacyExecutionPayloadBodyV2 {
    fn from(value: ExecutionPayloadBodyAmsterdam) -> Self {
        Self {
            transactions: value.transactions,
            withdrawals: Some(value.withdrawals),
            block_access_list: Some(value.block_access_list),
        }
    }
}

impl TryFrom<LegacyExecutionPayloadBodyV2> for ExecutionPayloadBodyAmsterdam {
    type Error = ExecutionPayloadBodyConversionError;

    fn try_from(value: LegacyExecutionPayloadBodyV2) -> Result<Self, Self::Error> {
        Ok(Self {
            transactions: value.transactions,
            withdrawals: value
                .withdrawals
                .ok_or(ExecutionPayloadBodyConversionError::MissingField("withdrawals"))?,
            block_access_list: value
                .block_access_list
                .ok_or(ExecutionPayloadBodyConversionError::MissingField("block_access_list"))?,
        })
    }
}

/// REST-SSZ historical bodies-by-hash request.
///
/// This is a single-field container, not a bare SSZ list.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BodiesByHashRequest {
    /// Requested block hashes.
    pub block_hashes: VariableList<B256, U32>,
}

/// Historical body response entry with explicit availability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyEntry<T> {
    /// Whether the body is available and belongs to the requested fork.
    pub available: bool,
    /// Fork-specific body, ignored when `available` is false.
    pub body: T,
}

impl<T> BodyEntry<T> {
    /// Creates an available body entry.
    pub const fn available(body: T) -> Self {
        Self { available: true, body }
    }
}

impl<T: Default> BodyEntry<T> {
    /// Creates an unavailable body entry.
    pub fn unavailable() -> Self {
        Self { available: false, body: T::default() }
    }
}

impl<T: ssz::Encode> ssz::Encode for BodyEntry<T> {
    fn is_ssz_fixed_len() -> bool {
        T::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        if T::is_ssz_fixed_len() {
            1 + T::ssz_fixed_len()
        } else {
            1 + ssz::BYTES_PER_LENGTH_OFFSET
        }
    }

    fn ssz_bytes_len(&self) -> usize {
        1 + if T::is_ssz_fixed_len() {
            self.body.ssz_bytes_len()
        } else {
            ssz::BYTES_PER_LENGTH_OFFSET + self.body.ssz_bytes_len()
        }
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let fixed_len = 1 + if T::is_ssz_fixed_len() {
            T::ssz_fixed_len()
        } else {
            ssz::BYTES_PER_LENGTH_OFFSET
        };
        let mut encoder = ssz::SszEncoder::container(buf, fixed_len);
        encoder.append(&self.available);
        encoder.append(&self.body);
        encoder.finalize();
    }
}

impl<T: ssz::Decode> ssz::Decode for BodyEntry<T> {
    fn is_ssz_fixed_len() -> bool {
        T::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        if T::is_ssz_fixed_len() {
            1 + T::ssz_fixed_len()
        } else {
            1 + ssz::BYTES_PER_LENGTH_OFFSET
        }
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<bool>()?;
        builder.register_type::<T>()?;
        let mut decoder = builder.build()?;
        Ok(Self { available: decoder.decode_next()?, body: decoder.decode_next()? })
    }
}

/// Bounded REST-SSZ historical bodies response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodiesResponse<T> {
    /// Body entries in request or range order.
    pub entries: VariableList<BodyEntry<T>, U32>,
}

/// Error constructing a bounded REST-SSZ historical bodies response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodiesResponseConversionError {
    /// The response contains more entries than the SSZ response limit.
    TooManyEntries,
}

impl core::fmt::Display for BodiesResponseConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooManyEntries => f.write_str("too many payload body entries"),
        }
    }
}

impl core::error::Error for BodiesResponseConversionError {}

impl<T: Default> BodiesResponse<T> {
    /// Creates a response from optional legacy bodies.
    ///
    /// Missing bodies, or bodies that do not convert to the requested fork container, are encoded
    /// as unavailable entries.
    pub fn from_optional_bodies<LegacyBody>(
        bodies: Vec<Option<LegacyBody>>,
        convert: impl Fn(LegacyBody) -> Option<T>,
    ) -> Result<Self, BodiesResponseConversionError> {
        let entries = bodies
            .into_iter()
            .map(|body| match body.and_then(&convert) {
                Some(body) => BodyEntry::available(body),
                None => BodyEntry::unavailable(),
            })
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| BodiesResponseConversionError::TooManyEntries)?;

        Ok(Self { entries })
    }
}

/// Paris historical bodies response.
pub type BodiesResponseParis = BodiesResponse<ExecutionPayloadBodyParis>;
/// Shanghai historical bodies response.
pub type BodiesResponseShanghai = BodiesResponse<ExecutionPayloadBodyShanghai>;
/// Cancun historical bodies response.
pub type BodiesResponseCancun = BodiesResponse<ExecutionPayloadBodyCancun>;
/// Prague historical bodies response.
pub type BodiesResponsePrague = BodiesResponse<ExecutionPayloadBodyPrague>;
/// Osaka historical bodies response.
pub type BodiesResponseOsaka = BodiesResponse<ExecutionPayloadBodyOsaka>;
/// Amsterdam historical bodies response.
pub type BodiesResponseAmsterdam = BodiesResponse<ExecutionPayloadBodyAmsterdam>;

impl<T: ssz::Encode> ssz::Encode for BodiesResponse<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        ssz::BYTES_PER_LENGTH_OFFSET + self.entries.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = ssz::SszEncoder::container(buf, ssz::BYTES_PER_LENGTH_OFFSET);
        encoder.append(&self.entries);
        encoder.finalize();
    }
}

impl<T: ssz::Decode + 'static> ssz::Decode for BodiesResponse<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<VariableList<BodyEntry<T>, U32>>()?;
        let mut decoder = builder.build()?;
        Ok(Self { entries: decoder.decode_next()? })
    }
}

#[cfg(all(test, feature = "ssz"))]
mod tests {
    use super::*;
    use alloy_primitives::{Address, Bloom, Bytes};
    use ssz::{Decode, Encode};

    fn payload_v1() -> ExecutionPayloadV1 {
        ExecutionPayloadV1 {
            parent_hash: B256::repeat_byte(1),
            fee_recipient: Address::repeat_byte(2),
            state_root: B256::repeat_byte(3),
            receipts_root: B256::repeat_byte(4),
            logs_bloom: Bloom::repeat_byte(5),
            prev_randao: B256::repeat_byte(6),
            block_number: 7,
            gas_limit: 8,
            gas_used: 9,
            timestamp: 10,
            extra_data: Bytes::from_static(&[11, 12]),
            base_fee_per_gas: U256::from(13),
            block_hash: B256::repeat_byte(14),
            transactions: vec![Bytes::from_static(&[15, 16])],
        }
    }

    fn payload_v2() -> ExecutionPayloadV2 {
        ExecutionPayloadV2 { payload_inner: payload_v1(), withdrawals: vec![Withdrawal::default()] }
    }

    fn payload_v3() -> ExecutionPayloadV3 {
        ExecutionPayloadV3 { payload_inner: payload_v2(), blob_gas_used: 17, excess_blob_gas: 18 }
    }

    fn payload_v4() -> ExecutionPayloadV4 {
        ExecutionPayloadV4 {
            payload_inner: payload_v3(),
            block_access_list: Bytes::from_static(&[19, 20]),
            slot_number: 21,
        }
    }

    fn attributes_cancun() -> PayloadAttributesCancun {
        PayloadAttributesCancun {
            timestamp: 1,
            prev_randao: B256::repeat_byte(2),
            suggested_fee_recipient: Address::repeat_byte(3),
            withdrawals: vec![Withdrawal::default()],
            parent_beacon_block_root: B256::repeat_byte(4),
        }
    }

    fn state() -> ForkchoiceState {
        ForkchoiceState {
            head_block_hash: B256::repeat_byte(1),
            safe_block_hash: B256::repeat_byte(2),
            finalized_block_hash: B256::repeat_byte(3),
        }
    }

    fn assert_roundtrip<T>(value: &T)
    where
        T: Encode + Decode + PartialEq + core::fmt::Debug,
    {
        assert_eq!(T::from_ssz_bytes(&value.as_ssz_bytes()).unwrap(), *value);
    }

    #[test]
    fn execution_payload_envelopes_roundtrip() {
        assert_roundtrip(&ExecutionPayloadEnvelopeParis { payload: payload_v1() });
        assert_roundtrip(&ExecutionPayloadEnvelopeShanghai { payload: payload_v2() });
        assert_roundtrip(&ExecutionPayloadEnvelopeCancun {
            payload: payload_v3(),
            parent_beacon_block_root: B256::repeat_byte(1),
        });
        assert_roundtrip(&ExecutionPayloadEnvelopePrague {
            payload: payload_v3(),
            parent_beacon_block_root: B256::repeat_byte(1),
            execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
        });
        assert_roundtrip(&ExecutionPayloadEnvelopeOsaka {
            payload: payload_v3(),
            parent_beacon_block_root: B256::repeat_byte(1),
            execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
        });
        assert_roundtrip(&ExecutionPayloadEnvelopeAmsterdam {
            payload: payload_v4(),
            parent_beacon_block_root: B256::repeat_byte(1),
            execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
        });
    }

    #[test]
    fn paris_submission_is_a_single_field_container() {
        let payload = payload_v1();
        let payload_bytes = payload.as_ssz_bytes();
        let envelope = ExecutionPayloadEnvelopeParis { payload };
        let encoded = envelope.as_ssz_bytes();
        assert_eq!(&encoded[..4], &4u32.to_le_bytes());
        assert_eq!(&encoded[4..], payload_bytes);
    }

    #[test]
    fn built_payloads_roundtrip() {
        assert_roundtrip(&BuiltPayloadParis { payload: payload_v1(), block_value: U256::from(1) });
        assert_roundtrip(&BuiltPayloadShanghai {
            payload: payload_v2(),
            block_value: U256::from(1),
        });
        assert_roundtrip(&BuiltPayloadCancun {
            execution_payload: payload_v3(),
            block_value: U256::from(1),
            blobs_bundle: BlobsBundleV1::empty(),
            should_override_builder: true,
        });
        assert_roundtrip(&BuiltPayloadPrague {
            payload: payload_v3(),
            block_value: U256::from(1),
            blobs_bundle: BlobsBundleV1::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
            should_override_builder: true,
        });
        assert_roundtrip(&BuiltPayloadOsaka {
            payload: payload_v3(),
            block_value: U256::from(1),
            blobs_bundle: BlobsBundleV2::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
            should_override_builder: true,
        });
        assert_roundtrip(&BuiltPayloadAmsterdam {
            payload: payload_v4(),
            block_value: U256::from(1),
            blobs_bundle: BlobsBundleV2::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
            should_override_builder: true,
        });
    }

    #[test]
    fn shanghai_built_payload_has_no_builder_override() {
        let payload = payload_v2();
        let payload_len = payload.ssz_bytes_len();
        let value = BuiltPayloadShanghai { payload, block_value: U256::from(1) };
        let encoded = value.as_ssz_bytes();

        assert_eq!(&encoded[..4], &36u32.to_le_bytes());
        assert_eq!(encoded.len(), 36 + payload_len);
    }

    #[test]
    fn legacy_built_payload_conversions_preserve_fields() {
        let shanghai = BuiltPayloadShanghai { payload: payload_v2(), block_value: U256::from(1) };
        let legacy = LegacyBuiltPayloadShanghai::from(shanghai.clone());
        assert_eq!(BuiltPayloadShanghai::try_from(legacy).unwrap(), shanghai);

        let prague = BuiltPayloadPrague {
            payload: payload_v3(),
            block_value: U256::from(2),
            blobs_bundle: BlobsBundleV1::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[3, 4])]),
            should_override_builder: true,
        };
        let legacy = LegacyBuiltPayloadPrague::from(prague.clone());
        assert_eq!(BuiltPayloadPrague::from(legacy), prague);

        let osaka = BuiltPayloadOsaka {
            payload: payload_v3(),
            block_value: U256::from(5),
            blobs_bundle: BlobsBundleV2::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[6, 7])]),
            should_override_builder: true,
        };
        let legacy = LegacyBuiltPayloadOsaka::from(osaka.clone());
        assert_eq!(BuiltPayloadOsaka::from(legacy), osaka);

        let amsterdam = BuiltPayloadAmsterdam {
            payload: payload_v4(),
            block_value: U256::from(8),
            blobs_bundle: BlobsBundleV2::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[9, 10])]),
            should_override_builder: true,
        };
        let legacy = LegacyBuiltPayloadAmsterdam::from(amsterdam.clone());
        assert_eq!(BuiltPayloadAmsterdam::from(legacy), amsterdam);
    }

    #[test]
    fn legacy_shanghai_built_payload_rejects_paris_payload() {
        let legacy = LegacyBuiltPayloadShanghai {
            execution_payload: ExecutionPayloadFieldV2::V1(payload_v1()),
            block_value: U256::from(1),
        };
        assert_eq!(
            BuiltPayloadShanghai::try_from(legacy),
            Err(BuiltPayloadConversionError::UnexpectedPayloadFork("Paris"))
        );
    }

    #[test]
    fn prague_requests_precede_should_override_builder() {
        let value = BuiltPayloadPrague {
            payload: payload_v3(),
            block_value: U256::from(1),
            blobs_bundle: BlobsBundleV1::empty(),
            execution_requests: Requests::from_requests([Bytes::from_static(&[0xaa, 0xbb])]),
            should_override_builder: true,
        };
        let encoded = value.as_ssz_bytes();
        assert_eq!(encoded[48], 1);
        assert_eq!(&encoded[encoded.len() - 2..], &[0xaa, 0xbb]);
    }

    #[test]
    fn forkchoice_updates_roundtrip() {
        let paris = PayloadAttributesParis {
            timestamp: 1,
            prev_randao: B256::repeat_byte(2),
            suggested_fee_recipient: Address::repeat_byte(3),
        };
        let shanghai = PayloadAttributesShanghai {
            timestamp: 1,
            prev_randao: B256::repeat_byte(2),
            suggested_fee_recipient: Address::repeat_byte(3),
            withdrawals: vec![Withdrawal::default()],
        };
        let amsterdam = PayloadAttributesAmsterdam {
            timestamp: 1,
            prev_randao: B256::repeat_byte(2),
            suggested_fee_recipient: Address::repeat_byte(3),
            withdrawals: vec![Withdrawal::default()],
            parent_beacon_block_root: B256::repeat_byte(4),
            slot_number: 5,
            target_gas_limit: 6,
        };
        assert_roundtrip(&ForkchoiceUpdateParis {
            forkchoice_state: state(),
            payload_attributes: Optional::some(paris),
        });
        assert_roundtrip(&ForkchoiceUpdateShanghai {
            forkchoice_state: state(),
            payload_attributes: Optional::some(shanghai),
        });
        assert_roundtrip(&ForkchoiceUpdateCancun {
            forkchoice_state: state(),
            payload_attributes: Optional::some(attributes_cancun()),
        });
        assert_roundtrip(&ForkchoiceUpdatePrague {
            forkchoice_state: state(),
            payload_attributes: Optional::some(attributes_cancun()),
        });
        assert_roundtrip(&ForkchoiceUpdateOsaka {
            forkchoice_state: state(),
            payload_attributes: Optional::some(attributes_cancun()),
        });
        assert_roundtrip(&ForkchoiceUpdateAmsterdam {
            forkchoice_state: state(),
            payload_attributes: Optional::some(amsterdam),
            custody_columns: Optional::some(B128::repeat_byte(0xa5)),
        });
    }

    #[test]
    fn payload_attributes_legacy_conversions_preserve_fork_shape() {
        let cancun = attributes_cancun();
        let legacy = LegacyPayloadAttributes::from(cancun.clone());
        assert_eq!(PayloadAttributesCancun::try_from(legacy).unwrap(), cancun);

        let amsterdam = PayloadAttributesAmsterdam {
            timestamp: 1,
            prev_randao: B256::repeat_byte(2),
            suggested_fee_recipient: Address::repeat_byte(3),
            withdrawals: vec![Withdrawal::default()],
            parent_beacon_block_root: B256::repeat_byte(4),
            slot_number: 5,
            target_gas_limit: 6,
        };
        let legacy = LegacyPayloadAttributes::from(amsterdam.clone());
        assert_eq!(PayloadAttributesAmsterdam::try_from(legacy).unwrap(), amsterdam);
    }

    #[test]
    fn payload_attributes_legacy_conversions_reject_loss() {
        let mut legacy = LegacyPayloadAttributes::default();
        assert_eq!(
            PayloadAttributesShanghai::try_from(legacy.clone()),
            Err(PayloadAttributesConversionError::MissingField("withdrawals"))
        );

        legacy.withdrawals = Some(vec![]);
        legacy.parent_beacon_block_root = Some(B256::ZERO);
        assert_eq!(
            PayloadAttributesShanghai::try_from(legacy),
            Err(PayloadAttributesConversionError::UnexpectedField("parent_beacon_block_root"))
        );
    }

    #[test]
    fn bodies_by_hash_is_a_bounded_single_field_container() {
        let request =
            BodiesByHashRequest { block_hashes: vec![B256::repeat_byte(0x42)].try_into().unwrap() };
        let encoded = request.as_ssz_bytes();
        assert_eq!(&encoded[..4], &4u32.to_le_bytes());
        assert_eq!(&encoded[4..], B256::repeat_byte(0x42).as_slice());
        assert_eq!(BodiesByHashRequest::from_ssz_bytes(&encoded).unwrap(), request);

        assert!(
            VariableList::<B256, U32>::try_from(vec![B256::ZERO; MAX_BODIES_REQUEST + 1]).is_err()
        );
    }

    #[test]
    fn fork_specific_bodies_roundtrip_without_optional_fields() {
        let paris = BodiesResponseParis {
            entries: vec![BodyEntry {
                available: true,
                body: ExecutionPayloadBodyParis { transactions: vec![Bytes::from_static(&[1, 2])] },
            }]
            .try_into()
            .unwrap(),
        };
        assert_roundtrip(&paris);

        let shanghai = BodiesResponseShanghai {
            entries: vec![BodyEntry {
                available: true,
                body: ExecutionPayloadBodyShanghai {
                    transactions: vec![Bytes::from_static(&[3, 4])],
                    withdrawals: vec![Withdrawal::default()],
                },
            }]
            .try_into()
            .unwrap(),
        };
        assert_roundtrip(&shanghai);

        let amsterdam = BodiesResponseAmsterdam {
            entries: vec![BodyEntry {
                available: false,
                body: ExecutionPayloadBodyAmsterdam {
                    transactions: vec![Bytes::from_static(&[5, 6])],
                    withdrawals: vec![Withdrawal::default()],
                    block_access_list: Bytes::from_static(&[7, 8]),
                },
            }]
            .try_into()
            .unwrap(),
        };
        assert_roundtrip(&amsterdam);
    }

    #[test]
    fn legacy_body_conversions_preserve_fork_shape() {
        let paris = ExecutionPayloadBodyParis { transactions: vec![Bytes::from_static(&[1, 2])] };
        let legacy = LegacyExecutionPayloadBodyV1::from(paris.clone());
        assert_eq!(ExecutionPayloadBodyParis::try_from(legacy).unwrap(), paris);

        let shanghai = ExecutionPayloadBodyShanghai {
            transactions: vec![Bytes::from_static(&[3, 4])],
            withdrawals: vec![Withdrawal::default()],
        };
        let legacy = LegacyExecutionPayloadBodyV1::from(shanghai.clone());
        assert_eq!(ExecutionPayloadBodyShanghai::try_from(legacy).unwrap(), shanghai);

        let amsterdam = ExecutionPayloadBodyAmsterdam {
            transactions: vec![Bytes::from_static(&[5, 6])],
            withdrawals: vec![Withdrawal::default()],
            block_access_list: Bytes::from_static(&[7, 8]),
        };
        let legacy = LegacyExecutionPayloadBodyV2::from(amsterdam.clone());
        assert_eq!(ExecutionPayloadBodyAmsterdam::try_from(legacy).unwrap(), amsterdam);
    }

    #[test]
    fn legacy_body_conversions_reject_wrong_fork_shape() {
        let legacy =
            LegacyExecutionPayloadBodyV1 { transactions: vec![], withdrawals: Some(vec![]) };
        assert_eq!(
            ExecutionPayloadBodyParis::try_from(legacy),
            Err(ExecutionPayloadBodyConversionError::UnexpectedField("withdrawals"))
        );

        let legacy = LegacyExecutionPayloadBodyV2 {
            transactions: vec![],
            withdrawals: Some(vec![]),
            block_access_list: None,
        };
        assert_eq!(
            ExecutionPayloadBodyAmsterdam::try_from(legacy),
            Err(ExecutionPayloadBodyConversionError::MissingField("block_access_list"))
        );
    }

    #[test]
    fn bodies_response_rejects_more_than_32_entries() {
        let entry = BodyEntry { available: false, body: ExecutionPayloadBodyParis::default() };
        assert!(VariableList::<BodyEntry<ExecutionPayloadBodyParis>, U32>::try_from(vec![
            entry;
            MAX_BODIES_REQUEST
                + 1
        ])
        .is_err());
    }

    #[test]
    fn amsterdam_forkchoice_appends_custody_columns() {
        let value = ForkchoiceUpdateAmsterdam {
            forkchoice_state: state(),
            payload_attributes: Optional::none(),
            custody_columns: Optional::some(B128::repeat_byte(0xa5)),
        };
        let encoded = value.as_ssz_bytes();
        assert_eq!(&encoded[..96], &state().as_ssz_bytes());
        assert_eq!(&encoded[104..], B128::repeat_byte(0xa5).as_slice());
    }

    fn witness() -> ExecutionWitnessV1 {
        ExecutionWitnessV1 {
            state: vec![WitnessNodeV1::try_from(vec![0x01, 0x02]).unwrap()].try_into().unwrap(),
            codes: vec![WitnessCodeV1::try_from(vec![0x03, 0x04, 0x05]).unwrap()]
                .try_into()
                .unwrap(),
            headers: vec![WitnessHeaderV1::try_from(vec![0x06]).unwrap()].try_into().unwrap(),
        }
    }

    fn valid_status() -> PayloadStatus {
        PayloadStatus { status: PayloadStatusEnum::Valid, latest_valid_hash: Optional::none() }
    }

    #[test]
    fn debug_execution_witness_converts_to_ssz_witness() {
        let debug_witness = alloy_rpc_types_debug::ExecutionWitness {
            state: vec![Bytes::from_static(&[0x01, 0x02])],
            codes: vec![Bytes::from_static(&[0x03])],
            keys: vec![],
            headers: vec![Bytes::from_static(&[0x04, 0x05, 0x06])],
        };

        let witness = ExecutionWitnessV1::try_from(debug_witness).unwrap();

        assert_eq!(Vec::from(witness.state[0].clone()), vec![0x01, 0x02]);
        assert_eq!(Vec::from(witness.codes[0].clone()), vec![0x03]);
        assert_eq!(Vec::from(witness.headers[0].clone()), vec![0x04, 0x05, 0x06]);
    }

    #[test]
    fn debug_execution_witness_keys_are_ignored() {
        let without_keys = alloy_rpc_types_debug::ExecutionWitness {
            state: vec![Bytes::from_static(&[0x01])],
            codes: vec![],
            keys: vec![],
            headers: vec![],
        };
        let with_keys = alloy_rpc_types_debug::ExecutionWitness {
            keys: vec![Bytes::from_static(&[0xaa, 0xbb])],
            ..without_keys.clone()
        };

        assert_eq!(
            ExecutionWitnessV1::try_from(with_keys).unwrap(),
            ExecutionWitnessV1::try_from(without_keys).unwrap()
        );
    }

    #[test]
    fn debug_execution_witness_rejects_oversized_inner_item() {
        let debug_witness = alloy_rpc_types_debug::ExecutionWitness {
            state: vec![Bytes::from(vec![0; MAX_WITNESS_ITEM_BYTES + 1])],
            codes: vec![],
            keys: vec![],
            headers: vec![],
        };

        assert!(matches!(
            ExecutionWitnessV1::try_from(debug_witness),
            Err(ConversionError::Bounds(_))
        ));
    }

    #[test]
    fn debug_execution_witness_rejects_too_many_items() {
        let debug_witness = alloy_rpc_types_debug::ExecutionWitness {
            state: vec![Bytes::new(); MAX_WITNESS_ITEMS + 1],
            codes: vec![],
            keys: vec![],
            headers: vec![],
        };

        assert!(matches!(
            ExecutionWitnessV1::try_from(debug_witness),
            Err(ConversionError::Bounds(_))
        ));
    }

    #[test]
    fn payload_status_with_witness_new_encodes_present_rest_optional() {
        let witness = witness();
        let response = PayloadStatusWithWitness::new(valid_status(), Some(witness.clone()));

        assert!(response.witness.is_some());
        assert_eq!(response.witness.as_ssz_bytes(), Optional::some(witness).as_ssz_bytes());
    }

    #[test]
    fn payload_status_with_witness_new_encodes_empty_rest_optional() {
        let response = PayloadStatusWithWitness::new(valid_status(), None);

        assert!(response.witness.is_none());
        assert_eq!(
            response.witness.as_ssz_bytes(),
            Optional::<ExecutionWitnessV1>::none().as_ssz_bytes()
        );
    }

    #[test]
    fn execution_witness_roundtrips_empty_and_nonempty() {
        let empty = ExecutionWitnessV1::default();
        assert_eq!(empty.as_ssz_bytes(), vec![12, 0, 0, 0, 12, 0, 0, 0, 12, 0, 0, 0]);
        assert_roundtrip(&empty);
        assert_roundtrip(&witness());
    }

    #[test]
    fn payload_status_with_witness_roundtrips() {
        let valid = PayloadStatusWithWitness {
            payload_status: PayloadStatus {
                status: PayloadStatusEnum::Valid,
                latest_valid_hash: Optional::some(B256::repeat_byte(0x42)),
            },
            witness: Optional::some(witness()),
        };
        assert_roundtrip(&valid);

        let without_witness = PayloadStatusWithWitness {
            payload_status: PayloadStatus {
                status: PayloadStatusEnum::Syncing,
                latest_valid_hash: Optional::none(),
            },
            witness: Optional::none(),
        };
        assert_roundtrip(&without_witness);
    }

    #[test]
    fn payload_status_with_witness_rejects_witness_for_nonvalid_status() {
        let response = PayloadStatusWithWitness {
            payload_status: PayloadStatus {
                status: PayloadStatusEnum::Syncing,
                latest_valid_hash: Optional::none(),
            },
            witness: Optional::some(witness()),
        };
        assert!(PayloadStatusWithWitness::from_ssz_bytes(&response.as_ssz_bytes()).is_err());
    }

    #[test]
    fn execution_witness_rejects_truncated_ssz() {
        let witness_bytes = witness().as_ssz_bytes();
        assert!(ExecutionWitnessV1::from_ssz_bytes(&witness_bytes[..11]).is_err());

        let response = PayloadStatusWithWitness {
            payload_status: PayloadStatus {
                status: PayloadStatusEnum::Valid,
                latest_valid_hash: Optional::none(),
            },
            witness: Optional::some(witness()),
        };
        let response_bytes = response.as_ssz_bytes();
        assert!(PayloadStatusWithWitness::from_ssz_bytes(&response_bytes[..7]).is_err());
    }
    fn request(hashes: Vec<B256>) -> BlobsV1Request {
        BlobsV1Request { versioned_hashes: hashes.try_into().unwrap() }
    }

    fn blob_v2(byte: u8) -> BlobAndProofV2 {
        BlobAndProofV2 {
            blob: Box::new(Blob::repeat_byte(byte)),
            proofs: vec![Bytes48::repeat_byte(byte)],
        }
    }

    #[test]
    fn v1_v3_requests_are_single_field_containers() {
        let request = request(vec![B256::repeat_byte(0x42)]);
        let encoded = request.as_ssz_bytes();
        assert_eq!(&encoded[..4], &4u32.to_le_bytes());
        assert_eq!(&encoded[4..], B256::repeat_byte(0x42).as_slice());

        let _: BlobsV2Request = BlobsV2Request::from_ssz_bytes(&encoded).unwrap();
        let _: BlobsV3Request = BlobsV3Request::from_ssz_bytes(&encoded).unwrap();
    }

    #[test]
    fn requests_reject_more_than_128_hashes() {
        assert!(
            VariableList::<B256, U128>::try_from(vec![B256::ZERO; MAX_BLOBS_REQUEST + 1]).is_err()
        );

        let mut encoded = 4u32.to_le_bytes().to_vec();
        encoded.extend(vec![0; (MAX_BLOBS_REQUEST + 1) * 32]);
        assert!(BlobsV1Request::from_ssz_bytes(&encoded).is_err());
    }

    #[test]
    fn v4_request_roundtrip_preserves_bitvector() {
        let request = BlobsV4Request {
            versioned_hashes: vec![B256::repeat_byte(0x11)].try_into().unwrap(),
            indices_bitarray: B128::repeat_byte(0xa5),
        };
        assert_eq!(BlobsV4Request::from_ssz_bytes(&request.as_ssz_bytes()).unwrap(), request);
    }

    #[test]
    fn optional_hash_distinguishes_none_from_zero() {
        assert_ne!(
            Optional::<B256>::none().as_ssz_bytes(),
            Optional::some(B256::ZERO).as_ssz_bytes()
        );
    }

    #[test]
    fn optional_error_distinguishes_none_from_empty() {
        assert_ne!(
            Optional::<ErrorBytes>::none().as_ssz_bytes(),
            Optional::some(ErrorBytes::empty()).as_ssz_bytes()
        );
    }

    #[test]
    fn validation_error_enforces_max_bytes() {
        let status = PayloadStatus {
            status: PayloadStatusEnum::Invalid {
                validation_error: "x".repeat(MAX_ERROR_BYTES + 1),
            },
            latest_valid_hash: Optional::none(),
        };
        assert!(PayloadStatus::try_from(LegacyPayloadStatus {
            status: status.status,
            latest_valid_hash: None,
        })
        .is_err());
        assert!(ErrorBytes::from_ssz_bytes(&vec![b'x'; MAX_ERROR_BYTES + 1]).is_err());
    }

    #[test]
    fn every_payload_status_roundtrips() {
        for status in [
            PayloadStatusEnum::Valid,
            PayloadStatusEnum::Invalid { validation_error: "invalid".into() },
            PayloadStatusEnum::Syncing,
            PayloadStatusEnum::Accepted,
        ] {
            let value = PayloadStatus { status, latest_valid_hash: Optional::some(B256::ZERO) };
            assert_eq!(PayloadStatus::from_ssz_bytes(&value.as_ssz_bytes()).unwrap(), value);
        }
    }

    #[test]
    fn optional_payload_id_distinguishes_none_from_zero() {
        let status =
            PayloadStatus { status: PayloadStatusEnum::Valid, latest_valid_hash: Optional::none() };
        let none = ForkchoiceUpdateResponse {
            payload_status: status.clone(),
            payload_id: Optional::none(),
        };
        let zero = ForkchoiceUpdateResponse {
            payload_status: status,
            payload_id: Optional::some(PayloadId::default()),
        };
        assert_ne!(none.as_ssz_bytes(), zero.as_ssz_bytes());
    }

    #[test]
    fn forkchoice_conversion_rejects_accepted() {
        let legacy = LegacyForkchoice::from_status(PayloadStatusEnum::Accepted);
        assert_eq!(
            ForkchoiceUpdateResponse::try_from(legacy),
            Err(ConversionError::AcceptedForkchoice)
        );
    }

    #[test]
    fn blob_response_conversions_preserve_availability_and_order() {
        let v1 = BlobsV1Response::try_from(vec![None]).unwrap();
        assert!(!v1.entries[0].available);
        assert_eq!(v1.entries[0].contents, zero_blob_v1());

        let v2 = BlobsV2Response::try_from(vec![blob_v2(1), blob_v2(2)]).unwrap();
        assert!(v2.entries.iter().all(|entry| entry.available));

        let v3 = BlobsV3Response::try_from(vec![Some(blob_v2(1)), None, Some(blob_v2(3))]).unwrap();
        assert_eq!(
            v3.entries.iter().map(|entry| entry.available).collect::<Vec<_>>(),
            [true, false, true]
        );
        assert_eq!(v3.entries[2].contents.blob.as_slice(), Blob::repeat_byte(3).as_slice());

        let partial = BlobCellsAndProofs {
            blob_cells: vec![Optional::some(Cell::repeat_byte(1)), Optional::none()]
                .try_into()
                .unwrap(),
            proofs: vec![Optional::some(Bytes48::repeat_byte(2)), Optional::none()]
                .try_into()
                .unwrap(),
        };
        let v4 = BlobsV4Response::try_from(vec![None, Some(partial.clone())]).unwrap();
        assert!(!v4.entries[0].available);
        assert!(v4.entries[1].available);
        assert_eq!(v4.entries[1].contents, partial);
    }

    #[test]
    fn variable_blob_response_roundtrips_with_entry_offsets() {
        let response =
            BlobsV3Response::try_from(vec![Some(blob_v2(1)), None, Some(blob_v2(3))]).unwrap();
        let encoded = response.as_ssz_bytes();
        assert_eq!(&encoded[..4], &4u32.to_le_bytes());
        assert_eq!(BlobsV3Response::from_ssz_bytes(&encoded).unwrap(), response);
    }

    #[test]
    fn blob_cells_and_proofs_uses_rest_optional() {
        let value = BlobCellsAndProofs {
            blob_cells: vec![Optional::some(Cell::repeat_byte(1))].try_into().unwrap(),
            proofs: vec![Optional::some(Bytes48::repeat_byte(2))].try_into().unwrap(),
        };
        let encoded = value.as_ssz_bytes();

        assert_eq!(BlobCellsAndProofs::from_ssz_bytes(&encoded).unwrap(), value);
        assert!(!encoded[8..].starts_with(&[1, 0, 0, 0]));
    }

    #[test]
    fn blob_responses_reject_more_than_128_entries() {
        assert!(BlobsV4Response::try_from(vec![None; MAX_BLOBS_REQUEST + 1]).is_err());
    }

    #[test]
    fn optional_decoding_rejects_more_than_one_value() {
        assert!(Optional::<B256>::from_ssz_bytes(&[0; 64]).is_err());
        let variable_values = [8, 0, 0, 0, 8, 0, 0, 0];
        assert!(Optional::<ErrorBytes>::from_ssz_bytes(&variable_values).is_err());
    }
}
