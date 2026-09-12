use super::*;

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
///
/// Fork-specific attributes keep later-fork fields out of the SSZ body; the legacy type is a
/// permissive superset.
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
///
/// Fork-specific attributes keep later-fork fields out of the SSZ body; the legacy type is a
/// permissive superset.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesShanghai {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals to include in the payload.
    #[ssz(with = "bounds")]
    pub withdrawals: Vec<Withdrawal>,
}

/// Cancun payload attributes.
///
/// Fork-specific attributes keep later-fork fields out of the SSZ body; the legacy type is a
/// permissive superset.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesCancun {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals to include in the payload.
    #[ssz(with = "bounds")]
    pub withdrawals: Vec<Withdrawal>,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
}

/// Prague uses the Cancun payload-attributes schema.
pub type PayloadAttributesPrague = PayloadAttributesCancun;

/// Osaka uses the Cancun payload-attributes schema.
pub type PayloadAttributesOsaka = PayloadAttributesCancun;

/// Amsterdam payload attributes.
///
/// Fork-specific attributes keep the Amsterdam-only fields in their defined SSZ position.
#[derive(Clone, Debug, Default, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct PayloadAttributesAmsterdam {
    /// Payload timestamp.
    pub timestamp: u64,
    /// Previous RANDAO value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals to include in the payload.
    #[ssz(with = "bounds")]
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
    #[allow(clippy::needless_update)]
    fn from(value: PayloadAttributesAmsterdam) -> Self {
        Self {
            timestamp: value.timestamp,
            prev_randao: value.prev_randao,
            suggested_fee_recipient: value.suggested_fee_recipient,
            withdrawals: Some(value.withdrawals),
            parent_beacon_block_root: Some(value.parent_beacon_block_root),
            slot_number: Some(value.slot_number),
            target_gas_limit: Some(value.target_gas_limit),
            ..Default::default()
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
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadParis,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Shanghai.
///
/// This follows the legacy `engine_getPayloadV2` payload-build response shape: execution payload
/// plus block value only. `should_override_builder` starts at Cancun.
/// The concrete V2 payload prevents the legacy V1/V2 untagged field from accepting a Paris payload.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadShanghai {
    /// Execution payload V2.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadShanghai,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
}

/// Engine API v2 REST-SSZ payload-build response for Cancun.
///
/// This is wire-compatible with the legacy `engine_getPayloadV3` response envelope.
pub type BuiltPayloadCancun = crate::ExecutionPayloadEnvelopeV3;

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Prague.
///
/// Unlike the legacy [`crate::ExecutionPayloadEnvelopeV4`],
/// `execution_requests` precedes `should_override_builder` in the normative SSZ field order.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadPrague {
    /// Execution payload V3.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadPrague,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
    /// The blobs, commitments, and proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV1,
    /// A list of opaque EIP-7685 requests.
    #[ssz(with = "bounds")]
    pub execution_requests: Requests,
    /// A suggestion from the execution layer whether this payload should be used instead of an
    /// externally provided one.
    pub should_override_builder: bool,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Osaka.
///
/// It is separate from legacy V5 because REST-SSZ places `execution_requests` before the builder
/// override flag.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadOsaka {
    /// Execution payload V3.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadOsaka,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
    /// The blobs, commitments, and EIP-7594 cell proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV2,
    /// A list of opaque EIP-7685 requests.
    #[ssz(with = "bounds")]
    pub execution_requests: Requests,
    /// A suggestion from the execution layer whether this payload should be used instead of an
    /// externally provided one.
    pub should_override_builder: bool,
}

/// This structure maps to the Engine API v2 REST-SSZ payload-build response for Amsterdam.
///
/// It is separate from legacy V6 because REST-SSZ places `execution_requests` before the builder
/// override flag.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct BuiltPayloadAmsterdam {
    /// Execution payload V4.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadAmsterdam,
    /// The expected value to be received by the fee recipient in wei.
    pub block_value: U256,
    /// The blobs, commitments, and EIP-7594 cell proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV2,
    /// A list of opaque EIP-7685 requests.
    #[ssz(with = "bounds")]
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

/// REST-SSZ payload-submission request containers.
///
/// These are distinct from the legacy Engine JSON-RPC get-payload envelopes: submission requests
/// are fork-specific request bodies, while the legacy envelope types mostly model get-payload
/// responses and sometimes carry response-only fields such as block value, blob bundles, builder
/// override hints, or a different field order.
///
/// Paris payload-submission request.
///
/// The single-field container is required by REST-SSZ; the legacy endpoint submitted a bare
/// payload.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeParis {
    /// Submitted execution payload.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadParis,
}

/// Shanghai payload-submission request.
///
/// The single-field container is required by REST-SSZ and fixes the payload fork at decode time.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeShanghai {
    /// Submitted execution payload.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadShanghai,
}

/// Cancun payload-submission request.
///
/// Cancun adds the parent beacon block root to the REST request envelope.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeCancun {
    /// Submitted execution payload.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadCancun,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
}

/// Prague payload-submission request.
///
/// Prague adds execution requests to the REST request envelope.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopePrague {
    /// Submitted execution payload.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadPrague,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// EIP-7685 execution requests.
    #[ssz(with = "bounds")]
    pub execution_requests: Requests,
}

/// Osaka payload-submission request.
///
/// Osaka keeps the REST envelope shape while selecting the Osaka payload schema.
pub type ExecutionPayloadEnvelopeOsaka = ExecutionPayloadEnvelopePrague;

/// Amsterdam payload-submission request.
///
/// Amsterdam selects the V4 payload while retaining the Cancun and Prague envelope fields.
#[derive(Clone, Debug, PartialEq, Eq, ssz_derive::Encode, ssz_derive::Decode)]
pub struct ExecutionPayloadEnvelopeAmsterdam {
    /// Submitted execution payload.
    #[ssz(with = "bounds")]
    pub payload: ExecutionPayloadAmsterdam,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// EIP-7685 execution requests.
    #[ssz(with = "bounds")]
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
///
/// REST-SSZ uses an `Optional` field inside one container; legacy FCU used separate RPC
/// parameters and a legacy `Option` encoding.
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
pub type ForkchoiceUpdatePrague = ForkchoiceUpdateCancun;

/// Osaka forkchoice-update request.
pub type ForkchoiceUpdateOsaka = ForkchoiceUpdateCancun;

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
