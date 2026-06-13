//! Experimental Engine API v2 REST-SSZ wire types.
//!
//! These types intentionally live apart from the legacy JSON-RPC Engine API types because their
//! SSZ encodings are not wire-compatible. In particular, the draft
//! [`refactor.md`](https://github.com/ethereum/execution-apis/blob/refs/pull/793/head/src/engine/refactor.md)
//! specifies a top-level list for V1-V3 blob requests while
//! [`refactor-ssz.md`](https://github.com/ethereum/execution-apis/blob/refs/pull/793/head/src/engine/refactor-ssz.md)
//! specifies the single-field containers implemented here. See
//! [execution-apis PR #793](https://github.com/ethereum/execution-apis/pull/793).

use crate::{
    BlobAndProofV1, BlobAndProofV2, BlobCellsAndProofsV1, ForkchoiceUpdated as LegacyForkchoice,
    PayloadId, PayloadStatus as LegacyPayloadStatus, PayloadStatusEnum,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use alloy_eips::eip4844::{Blob, Bytes48};
use alloy_primitives::{B128, B256};
#[cfg(feature = "ssz")]
use ssz_types::{
    typenum::{U1, U1024, U128},
    VariableList,
};

/// Maximum number of blobs in a REST-SSZ blob request or response.
pub const MAX_BLOBS_REQUEST: usize = 128;

/// Maximum UTF-8 byte length of a payload validation error.
pub const MAX_ERROR_BYTES: usize = 1024;

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

fn status_code(status: &PayloadStatusEnum) -> u8 {
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
pub type BlobsV4Response = BlobsResponse<BlobCellsAndProofsV1>;

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

impl TryFrom<Vec<Option<BlobCellsAndProofsV1>>> for BlobsV4Response {
    type Error = ssz_types::Error;

    fn try_from(value: Vec<Option<BlobCellsAndProofsV1>>) -> Result<Self, Self::Error> {
        let entries = value
            .into_iter()
            .map(|value| match value {
                Some(contents) => BlobEntry { available: true, contents },
                None => BlobEntry { available: false, contents: BlobCellsAndProofsV1::default() },
            })
            .collect::<Vec<_>>()
            .try_into()?;
        Ok(Self { entries })
    }
}

#[cfg(all(test, feature = "ssz"))]
mod tests {
    use super::*;
    use alloy_eips::eip7594::Cell;
    use ssz::{Decode, Encode};

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
            status: status.status.clone(),
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

        let partial = BlobCellsAndProofsV1 {
            blob_cells: vec![Some(Cell::repeat_byte(1)), None],
            proofs: vec![Some(Bytes48::repeat_byte(2)), None],
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
