use super::*;

/// Engine API v2 REST-SSZ payload status.
///
/// This is separate from the legacy status because REST-SSZ uses `Optional` fields instead of
/// zero-value sentinels and legacy byte lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadStatus {
    /// Payload validation status.
    pub status: PayloadStatusKind,
    /// Most recent valid block hash.
    pub latest_valid_hash: Optional<B256>,
    /// Optional payload validation error bytes.
    pub validation_error: Optional<ValidationError>,
}

impl ssz::Encode for PayloadStatus {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        1 + ssz::BYTES_PER_LENGTH_OFFSET * 2
            + self.latest_valid_hash.ssz_bytes_len()
            + self.validation_error.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let mut encoder = ssz::SszEncoder::container(buf, 1 + ssz::BYTES_PER_LENGTH_OFFSET * 2);
        encoder.append(&self.status);
        encoder.append(&self.latest_valid_hash);
        encoder.append(&self.validation_error);
        encoder.finalize();
    }
}

impl ssz::Decode for PayloadStatus {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);
        builder.register_type::<PayloadStatusKind>()?;
        builder.register_type::<Optional<B256>>()?;
        builder.register_type::<Optional<ValidationError>>()?;
        let mut decoder = builder.build()?;
        let status: PayloadStatusKind = decoder.decode_next()?;
        let latest_valid_hash = decoder.decode_next()?;
        let validation_error: Optional<ValidationError> = decoder.decode_next()?;
        if status != PayloadStatusKind::Invalid && validation_error.is_some() {
            return Err(ssz::DecodeError::BytesInvalid(
                "validation error is only valid for INVALID status".into(),
            ));
        }
        Ok(Self { status, latest_valid_hash, validation_error })
    }
}

/// Error converting legacy Engine API values into v2 REST-SSZ values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionError {
    /// Payload validation error exceeded the REST-SSZ byte bound.
    ErrorBytesTooLong,
    /// Payload validation error was not UTF-8.
    InvalidUtf8,
    /// `ACCEPTED` is not permitted in a forkchoice response.
    AcceptedForkchoice,
    /// A bounded REST-SSZ list exceeded its maximum length.
    TooManyItems {
        /// Name of the field that exceeded its bound.
        field: &'static str,
        /// Maximum permitted item count.
        max: usize,
        /// Actual item count.
        actual: usize,
    },
}

impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ErrorBytesTooLong => f.write_str("payload validation error is too long"),
            Self::InvalidUtf8 => f.write_str("payload validation error is not UTF-8"),
            Self::AcceptedForkchoice => {
                f.write_str("ACCEPTED is not valid in a forkchoice response")
            }
            Self::TooManyItems { field, max, actual } => {
                write!(f, "too many {field}: expected at most {max}, got {actual}")
            }
        }
    }
}

impl core::error::Error for ConversionError {}

impl TryFrom<LegacyPayloadStatus> for PayloadStatus {
    type Error = ConversionError;

    fn try_from(value: LegacyPayloadStatus) -> Result<Self, Self::Error> {
        let (status, validation_error) = match value.status {
            PayloadStatusEnum::Valid => (PayloadStatusKind::Valid, Optional::none()),
            PayloadStatusEnum::Syncing => (PayloadStatusKind::Syncing, Optional::none()),
            PayloadStatusEnum::Accepted => (PayloadStatusKind::Accepted, Optional::none()),
            PayloadStatusEnum::Invalid { validation_error } => (
                PayloadStatusKind::Invalid,
                Optional::some(ValidationError::try_from(Bytes::from(
                    validation_error.into_bytes(),
                ))?),
            ),
        };
        Ok(Self { status, latest_valid_hash: value.latest_valid_hash.into(), validation_error })
    }
}

impl From<PayloadStatus> for LegacyPayloadStatus {
    fn from(value: PayloadStatus) -> Self {
        let status = match value.status {
            PayloadStatusKind::Valid => PayloadStatusEnum::Valid,
            PayloadStatusKind::Syncing => PayloadStatusEnum::Syncing,
            PayloadStatusKind::Accepted => PayloadStatusEnum::Accepted,
            PayloadStatusKind::Invalid => PayloadStatusEnum::Invalid {
                validation_error: value
                    .validation_error
                    .into_option()
                    .map(|error| {
                        String::from_utf8(error.0.to_vec()).expect("validation error is UTF-8")
                    })
                    .unwrap_or_default(),
            },
        };
        Self { status, latest_valid_hash: value.latest_valid_hash.into() }
    }
}

/// UTF-8 payload validation error, bounded to 1024 bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError(Bytes);

impl TryFrom<Bytes> for ValidationError {
    type Error = ConversionError;
    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        if bytes.len() > MAX_ERROR_BYTES {
            return Err(ConversionError::ErrorBytesTooLong);
        }
        core::str::from_utf8(&bytes).map_err(|_| ConversionError::InvalidUtf8)?;
        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for ValidationError {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl ssz::Encode for ValidationError {
    fn is_ssz_fixed_len() -> bool {
        false
    }
    fn ssz_bytes_len(&self) -> usize {
        self.0.len()
    }
    fn ssz_append(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.0);
    }
}

impl ssz::Decode for ValidationError {
    fn is_ssz_fixed_len() -> bool {
        false
    }
    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        // Validate before allocating the error buffer.
        bounds::check(bytes.len(), MAX_ERROR_BYTES, "validation_error")?;
        core::str::from_utf8(bytes)
            .map_err(|err| ssz::DecodeError::BytesInvalid(err.to_string()))?;
        Ok(Self(Bytes::copy_from_slice(bytes)))
    }
}

/// REST-SSZ payload validation status tag. Error text lives in `PayloadStatus::validation_error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
#[ssz(enum_behaviour = "tag")]
pub enum PayloadStatusKind {
    /// The payload is valid.
    Valid,
    /// The payload is invalid.
    Invalid,
    /// The engine is syncing.
    Syncing,
    /// The payload was accepted for later validation.
    Accepted,
}

/// An Engine API v2 SSZ optional encoded as `List[T, 1]`.
///
/// This differs from [`Option<T>`]'s `ethereum_ssz` encoding, which uses an SSZ union.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Optional<T>(Option<T>);

impl<T> Optional<T> {
    /// Creates an absent optional.
    pub const fn none() -> Self {
        Self(None)
    }

    /// Creates a present optional.
    pub const fn some(value: T) -> Self {
        Self(Some(value))
    }

    /// Returns the contained value, if present.
    pub const fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Returns true if no value is present.
    pub const fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Returns true if a value is present.
    pub const fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Converts into a Rust optional.
    pub fn into_option(self) -> Option<T> {
        self.0
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
        self.0.as_ref().map_or(0, |value| {
            value.ssz_bytes_len()
                + if T::is_ssz_fixed_len() { 0 } else { ssz::BYTES_PER_LENGTH_OFFSET }
        })
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        if let Some(value) = &self.0 {
            if !T::is_ssz_fixed_len() {
                buf.extend_from_slice(&(ssz::BYTES_PER_LENGTH_OFFSET as u32).to_le_bytes());
            }
            value.ssz_append(buf);
        }
    }
}

impl<T: ssz::Decode> ssz::Decode for Optional<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        if bytes.is_empty() {
            return Ok(Self::none());
        }
        let value = if T::is_ssz_fixed_len() {
            T::from_ssz_bytes(bytes)?
        } else {
            // A present variable-size value has exactly one canonical list offset.
            if bytes.get(..4) != Some(&[4, 0, 0, 0]) {
                return Err(ssz::DecodeError::BytesInvalid(
                    "optional must contain exactly one value".into(),
                ));
            }
            T::from_ssz_bytes(&bytes[4..])?
        };
        Ok(Self::some(value))
    }
}

/// Engine API v2 REST-SSZ forkchoice update response.
///
/// The REST response is a container of two variable fields, unlike the legacy fixed payload ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkchoiceUpdateResponse {
    /// Restricted payload status; `ACCEPTED` is invalid here.
    pub payload_status: PayloadStatus,
    /// Opaque server-assigned payload identifier.
    pub payload_id: Optional<PayloadId>,
}

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
        if matches!(response.payload_status.status, PayloadStatusKind::Accepted) {
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
        if matches!(payload_status.status, PayloadStatusKind::Accepted) {
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
