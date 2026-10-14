use super::*;

/// Fork-specific execution payload body for Paris.
///
/// Paris omits withdrawals entirely; the legacy body keeps them as an optional union field.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadBodyParis {
    /// Enveloped encoded transactions.
    #[ssz(with = "bounds")]
    pub transactions: Vec<Bytes>,
}

/// Fork-specific execution payload body for Shanghai.
///
/// Shanghai makes withdrawals a direct field rather than the legacy optional union.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadBodyShanghai {
    /// Enveloped encoded transactions.
    #[ssz(with = "bounds")]
    pub transactions: Vec<Bytes>,
    /// Withdrawals included in the block.
    #[ssz(with = "bounds")]
    pub withdrawals: Vec<Withdrawal>,
}

/// Cancun uses the Shanghai execution-payload-body schema.
pub type ExecutionPayloadBodyCancun = ExecutionPayloadBodyShanghai;

/// Prague uses the Shanghai execution-payload-body schema.
pub type ExecutionPayloadBodyPrague = ExecutionPayloadBodyShanghai;

/// Osaka uses the Shanghai execution-payload-body schema.
pub type ExecutionPayloadBodyOsaka = ExecutionPayloadBodyShanghai;

/// Fork-specific execution payload body for Amsterdam.
///
/// Amsterdam adds the block access list as a direct field rather than a legacy optional field.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadBodyAmsterdam {
    /// Enveloped encoded transactions.
    #[ssz(with = "bounds")]
    pub transactions: Vec<Bytes>,
    /// Withdrawals included in the block.
    #[ssz(with = "bounds")]
    pub withdrawals: Vec<Withdrawal>,
    /// RLP-encoded EIP-7928 block access list.
    #[ssz(with = "bounds")]
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
    #[ssz(with = "body_hashes")]
    pub block_hashes: Vec<B256>,
}

/// Historical body response entry with explicit availability.
///
/// REST-SSZ uses a boolean availability bit instead of the legacy `Option<body>` union.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BodyEntry<T: ssz::Encode + ssz::Decode> {
    /// Whether the body is available and belongs to the requested fork.
    pub available: bool,
    /// Fork-specific body, ignored when `available` is false.
    pub body: T,
}

impl<T: ssz::Encode + ssz::Decode> BodyEntry<T> {
    /// Creates an available body entry.
    pub const fn available(body: T) -> Self {
        Self { available: true, body }
    }
}

impl<T: ssz::Encode + ssz::Decode + Default> BodyEntry<T> {
    /// Creates an unavailable body entry.
    pub fn unavailable() -> Self {
        Self { available: false, body: T::default() }
    }
}

/// REST-SSZ historical bodies response.
///
/// The response is a one-field SSZ container around the entries list, not a bare list.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BodiesResponse<T: ssz::Encode + ssz::Decode> {
    /// Body entries in request or range order.
    #[ssz(with = "body_entries")]
    pub entries: Vec<BodyEntry<T>>,
}

impl<T: ssz::Encode + ssz::Decode + Default> BodiesResponse<T> {
    /// Creates a response from optional legacy bodies.
    ///
    /// Missing bodies, or bodies that do not convert to the requested fork container, are encoded
    /// as unavailable entries.
    pub fn from_optional_bodies<LegacyBody>(
        bodies: Vec<Option<LegacyBody>>,
        convert: impl Fn(LegacyBody) -> Option<T>,
    ) -> Result<Self, ConversionError> {
        if bodies.len() > MAX_BODIES_REQUEST {
            return Err(ConversionError::TooManyItems {
                field: "bodies",
                max: MAX_BODIES_REQUEST,
                actual: bodies.len(),
            });
        }
        let entries = bodies
            .into_iter()
            .map(|body| match body.and_then(&convert) {
                Some(body) => BodyEntry::available(body),
                None => BodyEntry::unavailable(),
            })
            .collect();

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
