//! Payload types.

use crate::{ExecutionPayloadSidecar, PayloadError};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use alloy_consensus::{
    constants::MAXIMUM_EXTRA_DATA_SIZE, Blob, Block, BlockBody, BlockHeader, Bytes48, Header,
    HeaderInfo, Transaction, EMPTY_OMMER_ROOT_HASH,
};
use alloy_eips::{
    calc_next_block_base_fee,
    eip1559::BaseFeeParams,
    eip2718::{Decodable2718, Encodable2718},
    eip4844::BlobTransactionSidecar,
    eip4895::{Withdrawal, Withdrawals},
    eip7594::{BlobTransactionSidecarEip7594, CELLS_PER_EXT_BLOB},
    eip7685::Requests,
    eip7840::BlobParams,
    BlockNumHash,
};
use alloy_primitives::{bytes::BufMut, Address, Bloom, Bytes, Sealable, B256, B64, U256};
use core::iter::{FromIterator, IntoIterator};

/// The execution payload body response that allows for `null` values.
pub type ExecutionPayloadBodiesV1 = Vec<Option<ExecutionPayloadBodyV1>>;

/// And 8-byte identifier for an execution payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct PayloadId(pub B64);

// === impl PayloadId ===

impl PayloadId {
    /// Creates a new payload id from the given identifier.
    pub fn new(id: [u8; 8]) -> Self {
        Self(B64::from(id))
    }
}

impl core::fmt::Display for PayloadId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<B64> for PayloadId {
    fn from(value: B64) -> Self {
        Self(value)
    }
}

/// This represents the `executionPayload` field in the return value of `engine_getPayloadV2`,
/// specified as:
///
/// - `executionPayload`: `ExecutionPayloadV1` | `ExecutionPayloadV2` where:
///   - `ExecutionPayloadV1` **MUST** be returned if the payload `timestamp` is lower than the
///     Shanghai timestamp
///   - `ExecutionPayloadV2` **MUST** be returned if the payload `timestamp` is greater or equal to
///     the Shanghai timestamp
///
/// See:
/// <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/shanghai.md#response>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum ExecutionPayloadFieldV2 {
    /// V1 payload
    V1(ExecutionPayloadV1),
    /// V2 payload
    V2(ExecutionPayloadV2),
}

impl ExecutionPayloadFieldV2 {
    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadFieldV2`].
    ///
    /// See also:
    ///  - [`ExecutionPayloadV1::from_block_unchecked`].
    ///  - [`ExecutionPayloadV2::from_block_unchecked`].
    ///
    /// If the block body contains withdrawals this returns [`ExecutionPayloadFieldV2::V2`].
    ///
    /// Note: This re-calculates the block hash.
    pub fn from_block_slow<T, H>(block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader + Sealable,
    {
        Self::from_block_unchecked(block.hash_slow(), block)
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadFieldV2`] using the given block
    /// hash.
    ///
    /// See also:
    ///  - [`ExecutionPayloadV1::from_block_unchecked`].
    ///  - [`ExecutionPayloadV2::from_block_unchecked`].
    ///
    /// If the block body contains withdrawals this returns [`ExecutionPayloadFieldV2::V2`].
    pub fn from_block_unchecked<T, H>(block_hash: B256, block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader,
    {
        if block.body.withdrawals.is_some() {
            Self::V2(ExecutionPayloadV2::from_block_unchecked(block_hash, block))
        } else {
            Self::V1(ExecutionPayloadV1::from_block_unchecked(block_hash, block))
        }
    }

    /// Returns the inner [ExecutionPayloadV1]
    pub fn into_v1_payload(self) -> ExecutionPayloadV1 {
        match self {
            Self::V1(payload) => payload,
            Self::V2(payload) => payload.payload_inner,
        }
    }

    /// Converts this payload variant into the corresponding [ExecutionPayload]
    pub fn into_payload(self) -> ExecutionPayload {
        match self {
            Self::V1(payload) => ExecutionPayload::V1(payload),
            Self::V2(payload) => ExecutionPayload::V2(payload),
        }
    }
}

/// This is the input to `engine_newPayloadV2`, which may or may not have a withdrawals field.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", deny_unknown_fields))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadInputV2 {
    /// The V1 execution payload
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub execution_payload: ExecutionPayloadV1,
    /// The payload withdrawals
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub withdrawals: Option<Vec<Withdrawal>>,
}

impl ExecutionPayloadInputV2 {
    /// Converts [`ExecutionPayloadInputV2`] to [`ExecutionPayload`]
    pub fn into_payload(self) -> ExecutionPayload {
        match self.withdrawals {
            Some(withdrawals) => ExecutionPayload::V2(ExecutionPayloadV2 {
                payload_inner: self.execution_payload,
                withdrawals,
            }),
            None => ExecutionPayload::V1(self.execution_payload),
        }
    }
}

impl From<ExecutionPayloadInputV2> for ExecutionPayload {
    fn from(input: ExecutionPayloadInputV2) -> Self {
        input.into_payload()
    }
}

/// This structure maps for the return value of `engine_getPayload` of the beacon chain spec, for
/// V2.
///
/// See also:
/// <https://github.com/ethereum/execution-apis/blob/main/src/engine/shanghai.md#engine_getpayloadv2>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadEnvelopeV2 {
    /// Execution payload, which could be either V1 or V2
    ///
    /// V1 (_NO_ withdrawals) MUST be returned if the payload timestamp is lower than the Shanghai
    /// timestamp
    ///
    /// V2 (_WITH_ withdrawals) MUST be returned if the payload timestamp is greater or equal to
    /// the Shanghai timestamp
    pub execution_payload: ExecutionPayloadFieldV2,
    /// The expected value to be received by the feeRecipient in wei
    pub block_value: U256,
}

impl ExecutionPayloadEnvelopeV2 {
    /// Returns the [ExecutionPayload] for the `engine_getPayloadV1` endpoint
    pub fn into_v1_payload(self) -> ExecutionPayloadV1 {
        self.execution_payload.into_v1_payload()
    }
}

/// This structure maps for the return value of `engine_getPayload` of the beacon chain spec, for
/// V3.
///
/// See also:
/// <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#response-2>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadEnvelopeV3 {
    /// Execution payload V3
    pub execution_payload: ExecutionPayloadV3,
    /// The expected value to be received by the feeRecipient in wei
    pub block_value: U256,
    /// The blobs, commitments, and proofs associated with the executed payload.
    pub blobs_bundle: BlobsBundleV1,
    /// Introduced in V3, this represents a suggestion from the execution layer if the payload
    /// should be used instead of an externally provided one.
    pub should_override_builder: bool,
}

/// This structure maps for the return value of `engine_getPayload` of the beacon chain spec, for
/// V4.
///
/// See also:
/// <https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md#engine_getpayloadv4>
#[derive(Clone, Debug, PartialEq, Eq, derive_more::Deref, derive_more::DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadEnvelopeV4 {
    /// Inner [`ExecutionPayloadEnvelopeV3`].
    #[deref]
    #[deref_mut]
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub envelope_inner: ExecutionPayloadEnvelopeV3,

    /// A list of opaque [EIP-7685][eip7685] requests.
    ///
    /// [eip7685]: https://eips.ethereum.org/EIPS/eip-7685
    pub execution_requests: Requests,
}

/// This structure maps for the return value of `engine_getPayload` of the beacon chain spec, for
/// V5.
///
/// See also:
/// <https://github.com/ethereum/execution-apis/blob/a091e7c3b6a5748a8843a1a9130d5fbfc3191a2c/src/engine/osaka.md#engine_getpayloadv5>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadEnvelopeV5 {
    /// Execution payload V3
    pub execution_payload: ExecutionPayloadV3,
    /// The expected value to be received by the feeRecipient in wei
    pub block_value: U256,
    /// The blobs, commitments, and EIP-7594 style cell proofs associated with the executed
    /// payload. See also: <https://github.com/ethereum/execution-apis/blob/a091e7c3b6a5748a8843a1a9130d5fbfc3191a2c/src/engine/osaka.md#BlobsBundleV2>.
    pub blobs_bundle: BlobsBundleV2,
    /// Introduced in V3, this represents a suggestion from the execution layer if the payload
    /// should be used instead of an externally provided one.
    pub should_override_builder: bool,
    /// A list of opaque [EIP-7685][eip7685] requests.
    ///
    /// [eip7685]: https://eips.ethereum.org/EIPS/eip-7685
    pub execution_requests: Requests,
}

/// This structure maps on the ExecutionPayload structure of the beacon chain spec.
///
/// See also: <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/paris.md#executionpayloadv1>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadV1 {
    /// The parent hash of the block.
    pub parent_hash: B256,
    /// The fee recipient of the block.
    pub fee_recipient: Address,
    /// The state root of the block.
    pub state_root: B256,
    /// The receipts root of the block.
    pub receipts_root: B256,
    /// The logs bloom of the block.
    pub logs_bloom: Bloom,
    /// The previous randao of the block.
    pub prev_randao: B256,
    /// The block number.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub block_number: u64,
    /// The gas limit of the block.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas_limit: u64,
    /// The gas used of the block.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas_used: u64,
    /// The timestamp of the block.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub timestamp: u64,
    /// The extra data of the block.
    pub extra_data: Bytes,
    /// The base fee per gas of the block.
    pub base_fee_per_gas: U256,
    /// The block hash of the block.
    pub block_hash: B256,
    /// The transactions of the block.
    pub transactions: Vec<Bytes>,
}

impl ExecutionPayloadV1 {
    /// Returns the block number and hash as a [`BlockNumHash`].
    pub const fn block_num_hash(&self) -> BlockNumHash {
        BlockNumHash::new(self.block_number, self.block_hash)
    }

    /// Converts [`ExecutionPayloadV1`] to [`Block`]
    pub fn try_into_block<T: Decodable2718>(self) -> Result<Block<T>, PayloadError> {
        self.try_into_block_with(|tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Converts [`ExecutionPayloadV1`] to [`Block`] with the given closure.
    pub fn try_into_block_with<T, F, E>(self, f: F) -> Result<Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.into_block_raw()?.try_map_transactions(f).map_err(Into::into)
    }

    /// Converts [`ExecutionPayloadV1`] to [`Block`] with raw [`Bytes`] transactions.
    ///
    /// This is similar to [`Self::try_into_block_with`] but returns the transactions as raw bytes
    /// without any conversion.
    pub fn into_block_raw(self) -> Result<Block<Bytes>, PayloadError> {
        if self.extra_data.len() > MAXIMUM_EXTRA_DATA_SIZE {
            return Err(PayloadError::ExtraData(self.extra_data));
        }

        // Calculate the transactions root using encoded bytes
        let transactions_root = alloy_consensus::proofs::ordered_trie_root_with_encoder(
            &self.transactions,
            |item, buf| buf.put_slice(item),
        );

        let header = Header {
            parent_hash: self.parent_hash,
            beneficiary: self.fee_recipient,
            state_root: self.state_root,
            transactions_root,
            receipts_root: self.receipts_root,
            withdrawals_root: None,
            logs_bloom: self.logs_bloom,
            number: self.block_number,
            gas_limit: self.gas_limit,
            gas_used: self.gas_used,
            timestamp: self.timestamp,
            mix_hash: self.prev_randao,
            // WARNING: It's allowed for a base fee in EIP1559 to increase unbounded. We assume that
            // it will fit in an u64. This is not always necessarily true, although it is extremely
            // unlikely not to be the case, a u64 maximum would have 2^64 which equates to 18 ETH
            // per gas.
            base_fee_per_gas: Some(
                self.base_fee_per_gas
                    .try_into()
                    .map_err(|_| PayloadError::BaseFee(self.base_fee_per_gas))?,
            ),
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_hash: None,
            extra_data: self.extra_data,
            // Defaults
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            difficulty: Default::default(),
            nonce: Default::default(),
            block_access_list_hash: Default::default(),
        };

        Ok(Block {
            header,
            body: BlockBody {
                transactions: self.transactions,
                ommers: vec![],
                withdrawals: None,
                block_access_list: None,
            },
        })
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV1`].
    ///
    /// Note: This re-calculates the block hash.
    pub fn from_block_slow<T, H>(block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader + Sealable,
    {
        Self::from_block_unchecked(block.header.hash_slow(), block)
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV1`] using the given block hash.
    pub fn from_block_unchecked<T, H>(block_hash: B256, block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader,
    {
        let transactions =
            block.body.transactions().map(|tx| tx.encoded_2718().into()).collect::<Vec<_>>();
        Self {
            parent_hash: block.parent_hash(),
            fee_recipient: block.beneficiary(),
            state_root: block.state_root(),
            receipts_root: block.receipts_root(),
            logs_bloom: block.logs_bloom(),
            prev_randao: block.mix_hash().unwrap_or_default(),
            block_number: block.number(),
            gas_limit: block.gas_limit(),
            gas_used: block.gas_used(),
            timestamp: block.timestamp(),
            base_fee_per_gas: U256::from(block.base_fee_per_gas().unwrap_or_default()),
            extra_data: block.header.extra_data().clone(),
            block_hash,
            transactions,
        }
    }

    /// Calculate base fee for next block according to the EIP-1559 spec.
    ///
    /// Returns a `None` if no base fee is set, no EIP-1559 support
    pub fn next_block_base_fee(&self, base_fee_params: BaseFeeParams) -> Option<u64> {
        Some(calc_next_block_base_fee(
            self.gas_used,
            self.gas_limit,
            self.base_fee_per_gas.try_into().ok()?,
            base_fee_params,
        ))
    }
}

impl<T: Decodable2718> TryFrom<ExecutionPayloadV1> for Block<T> {
    type Error = PayloadError;

    fn try_from(value: ExecutionPayloadV1) -> Result<Self, Self::Error> {
        value.try_into_block()
    }
}

/// This structure maps on the ExecutionPayloadV2 structure of the beacon chain spec.
///
/// See also: <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/shanghai.md#executionpayloadv2>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadV2 {
    /// Inner V1 payload
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload_inner: ExecutionPayloadV1,

    /// Array of [`Withdrawal`] enabled with V2
    /// See <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/shanghai.md#executionpayloadv2>
    pub withdrawals: Vec<Withdrawal>,
}

impl ExecutionPayloadV2 {
    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV2`].
    ///
    /// See also [`ExecutionPayloadV1::from_block_unchecked`].
    ///
    /// If the block does not have any withdrawals, an empty vector is used.
    ///
    /// Note: This re-calculates the block hash.
    pub fn from_block_slow<T, H>(block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader + Sealable,
    {
        Self::from_block_unchecked(block.header.hash_slow(), block)
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV2`] using the given block hash.
    ///
    /// See also [`ExecutionPayloadV1::from_block_unchecked`].
    ///
    /// If the block does not have any withdrawals, an empty vector is used.
    pub fn from_block_unchecked<T, H>(block_hash: B256, block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader,
    {
        Self {
            withdrawals: block
                .body
                .withdrawals
                .clone()
                .map(Withdrawals::into_inner)
                .unwrap_or_default(),
            payload_inner: ExecutionPayloadV1::from_block_unchecked(block_hash, block),
        }
    }

    /// Returns the timestamp for the execution payload.
    pub const fn timestamp(&self) -> u64 {
        self.payload_inner.timestamp
    }

    /// Converts [`ExecutionPayloadV2`] to [`ExecutionPayloadInputV2`].
    ///
    /// An [`ExecutionPayloadInputV2`] should have a [`Some`] withdrawals field if shanghai is
    /// active, otherwise the withdrawals field should be [`None`], so the `is_shanghai_active`
    /// argument is provided which will either:
    /// - include the withdrawals field as [`Some`] if true
    /// - set the withdrawals field to [`None`] if false
    pub fn into_payload_input_v2(self, is_shanghai_active: bool) -> ExecutionPayloadInputV2 {
        ExecutionPayloadInputV2 {
            execution_payload: self.payload_inner,
            withdrawals: is_shanghai_active.then_some(self.withdrawals),
        }
    }

    /// Converts [`ExecutionPayloadV2`] to [`Block`].
    ///
    /// This performs the same conversion as the underlying V1 payload, but calculates the
    /// withdrawals root and adds withdrawals.
    ///
    /// See also [`ExecutionPayloadV1::try_into_block`].
    pub fn try_into_block<T: Decodable2718>(self) -> Result<Block<T>, PayloadError> {
        self.try_into_block_with(|tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Converts [`ExecutionPayloadV2`] to [`Block`] with a custom transaction mapper.
    ///
    /// See also [`ExecutionPayloadV1::try_into_block_with`].
    pub fn try_into_block_with<T, F, E>(self, f: F) -> Result<Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.into_block_raw()?.try_map_transactions(f).map_err(Into::into)
    }

    /// Converts [`ExecutionPayloadV2`] to [`Block`] with raw [`Bytes`] transactions.
    ///
    /// This is similar to [`Self::try_into_block_with`] but returns the transactions as raw bytes
    /// without any conversion.
    pub fn into_block_raw(self) -> Result<Block<Bytes>, PayloadError> {
        let mut base_sealed_block = self.payload_inner.into_block_raw()?;
        let withdrawals_root =
            alloy_consensus::proofs::calculate_withdrawals_root(&self.withdrawals);
        base_sealed_block.body.withdrawals = Some(self.withdrawals.into());
        base_sealed_block.header.withdrawals_root = Some(withdrawals_root);
        Ok(base_sealed_block)
    }
}

impl<T: Decodable2718> TryFrom<ExecutionPayloadV2> for Block<T> {
    type Error = PayloadError;

    fn try_from(value: ExecutionPayloadV2) -> Result<Self, Self::Error> {
        value.try_into_block()
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for ExecutionPayloadV2 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);

        builder.register_type::<B256>()?;
        builder.register_type::<Address>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<Bloom>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<Bytes>()?;
        builder.register_type::<U256>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<Vec<Bytes>>()?;
        builder.register_type::<Vec<Withdrawal>>()?;

        let mut decoder = builder.build()?;

        Ok(Self {
            payload_inner: ExecutionPayloadV1 {
                parent_hash: decoder.decode_next()?,
                fee_recipient: decoder.decode_next()?,
                state_root: decoder.decode_next()?,
                receipts_root: decoder.decode_next()?,
                logs_bloom: decoder.decode_next()?,
                prev_randao: decoder.decode_next()?,
                block_number: decoder.decode_next()?,
                gas_limit: decoder.decode_next()?,
                gas_used: decoder.decode_next()?,
                timestamp: decoder.decode_next()?,
                extra_data: decoder.decode_next()?,
                base_fee_per_gas: decoder.decode_next()?,
                block_hash: decoder.decode_next()?,
                transactions: decoder.decode_next()?,
            },
            withdrawals: decoder.decode_next()?,
        })
    }
}

#[cfg(feature = "ssz")]
impl ssz::Encode for ExecutionPayloadV2 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let offset = <B256 as ssz::Encode>::ssz_fixed_len() * 5
            + <Address as ssz::Encode>::ssz_fixed_len()
            + <Bloom as ssz::Encode>::ssz_fixed_len()
            + <u64 as ssz::Encode>::ssz_fixed_len() * 4
            + <U256 as ssz::Encode>::ssz_fixed_len()
            + ssz::BYTES_PER_LENGTH_OFFSET * 3;

        let mut encoder = ssz::SszEncoder::container(buf, offset);

        encoder.append(&self.payload_inner.parent_hash);
        encoder.append(&self.payload_inner.fee_recipient);
        encoder.append(&self.payload_inner.state_root);
        encoder.append(&self.payload_inner.receipts_root);
        encoder.append(&self.payload_inner.logs_bloom);
        encoder.append(&self.payload_inner.prev_randao);
        encoder.append(&self.payload_inner.block_number);
        encoder.append(&self.payload_inner.gas_limit);
        encoder.append(&self.payload_inner.gas_used);
        encoder.append(&self.payload_inner.timestamp);
        encoder.append(&self.payload_inner.extra_data);
        encoder.append(&self.payload_inner.base_fee_per_gas);
        encoder.append(&self.payload_inner.block_hash);
        encoder.append(&self.payload_inner.transactions);
        encoder.append(&self.withdrawals);

        encoder.finalize();
    }

    fn ssz_bytes_len(&self) -> usize {
        <ExecutionPayloadV1 as ssz::Encode>::ssz_bytes_len(&self.payload_inner)
            + ssz::BYTES_PER_LENGTH_OFFSET
            + self.withdrawals.ssz_bytes_len()
    }
}

/// This structure maps on the ExecutionPayloadV3 structure of the beacon chain spec.
///
/// See also: <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/shanghai.md#executionpayloadv2>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadV3 {
    /// Inner V2 payload
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload_inner: ExecutionPayloadV2,

    /// Array of hex [`u64`] representing blob gas used, enabled with V3
    /// See <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#ExecutionPayloadV3>
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub blob_gas_used: u64,
    /// Array of hex[`u64`] representing excess blob gas, enabled with V3
    /// See <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#ExecutionPayloadV3>
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub excess_blob_gas: u64,
}

impl ExecutionPayloadV3 {
    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV3`].
    ///
    /// See also [`ExecutionPayloadV2::from_block_unchecked`].
    ///
    /// Note: This re-calculates the block hash.
    pub fn from_block_slow<T, H>(block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader + Sealable,
    {
        Self::from_block_unchecked(block.hash_slow(), block)
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV3`] using the given block hash.
    ///
    /// See also [`ExecutionPayloadV2::from_block_unchecked`].
    pub fn from_block_unchecked<T, H>(block_hash: B256, block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader,
    {
        Self {
            blob_gas_used: block.blob_gas_used().unwrap_or_default(),
            excess_blob_gas: block.excess_blob_gas().unwrap_or_default(),
            payload_inner: ExecutionPayloadV2::from_block_unchecked(block_hash, block),
        }
    }

    /// Returns the withdrawals for the payload.
    pub const fn withdrawals(&self) -> &Vec<Withdrawal> {
        &self.payload_inner.withdrawals
    }

    /// Returns the timestamp for the payload.
    pub const fn timestamp(&self) -> u64 {
        self.payload_inner.payload_inner.timestamp
    }

    /// Converts [`ExecutionPayloadV3`] to [`Block`].
    ///
    /// This performs the same conversion as the underlying V2 payload, but inserts the blob gas
    /// used and excess blob gas.
    ///
    /// See also [`ExecutionPayloadV2::try_into_block`].
    pub fn try_into_block<T: Decodable2718>(self) -> Result<Block<T>, PayloadError> {
        self.try_into_block_with(|tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Converts [`ExecutionPayloadV3`] to [`Block`] with a custom transaction mapper.
    ///
    /// See also [`ExecutionPayloadV2::try_into_block_with`].
    pub fn try_into_block_with<T, F, E>(self, f: F) -> Result<Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.into_block_raw()?.try_map_transactions(f).map_err(Into::into)
    }

    /// Converts [`ExecutionPayloadV3`] to [`Block`] with raw [`Bytes`] transactions.
    ///
    /// This is similar to [`Self::try_into_block_with`] but returns the transactions as raw bytes
    /// without any conversion.
    pub fn into_block_raw(self) -> Result<Block<Bytes>, PayloadError> {
        let mut base_block = self.payload_inner.into_block_raw()?;

        base_block.header.blob_gas_used = Some(self.blob_gas_used);
        base_block.header.excess_blob_gas = Some(self.excess_blob_gas);

        Ok(base_block)
    }
}

impl<T: Decodable2718> TryFrom<ExecutionPayloadV3> for Block<T> {
    type Error = PayloadError;

    fn try_from(value: ExecutionPayloadV3) -> Result<Self, Self::Error> {
        value.try_into_block()
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for ExecutionPayloadV3 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut builder = ssz::SszDecoderBuilder::new(bytes);

        builder.register_type::<B256>()?;
        builder.register_type::<Address>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<Bloom>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<Bytes>()?;
        builder.register_type::<U256>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<Vec<Bytes>>()?;
        builder.register_type::<Vec<Withdrawal>>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;

        let mut decoder = builder.build()?;

        Ok(Self {
            payload_inner: ExecutionPayloadV2 {
                payload_inner: ExecutionPayloadV1 {
                    parent_hash: decoder.decode_next()?,
                    fee_recipient: decoder.decode_next()?,
                    state_root: decoder.decode_next()?,
                    receipts_root: decoder.decode_next()?,
                    logs_bloom: decoder.decode_next()?,
                    prev_randao: decoder.decode_next()?,
                    block_number: decoder.decode_next()?,
                    gas_limit: decoder.decode_next()?,
                    gas_used: decoder.decode_next()?,
                    timestamp: decoder.decode_next()?,
                    extra_data: decoder.decode_next()?,
                    base_fee_per_gas: decoder.decode_next()?,
                    block_hash: decoder.decode_next()?,
                    transactions: decoder.decode_next()?,
                },
                withdrawals: decoder.decode_next()?,
            },
            blob_gas_used: decoder.decode_next()?,
            excess_blob_gas: decoder.decode_next()?,
        })
    }
}

#[cfg(feature = "ssz")]
impl ssz::Encode for ExecutionPayloadV3 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let offset = <B256 as ssz::Encode>::ssz_fixed_len() * 5
            + <Address as ssz::Encode>::ssz_fixed_len()
            + <Bloom as ssz::Encode>::ssz_fixed_len()
            + <u64 as ssz::Encode>::ssz_fixed_len() * 6
            + <U256 as ssz::Encode>::ssz_fixed_len()
            + ssz::BYTES_PER_LENGTH_OFFSET * 3;

        let mut encoder = ssz::SszEncoder::container(buf, offset);

        encoder.append(&self.payload_inner.payload_inner.parent_hash);
        encoder.append(&self.payload_inner.payload_inner.fee_recipient);
        encoder.append(&self.payload_inner.payload_inner.state_root);
        encoder.append(&self.payload_inner.payload_inner.receipts_root);
        encoder.append(&self.payload_inner.payload_inner.logs_bloom);
        encoder.append(&self.payload_inner.payload_inner.prev_randao);
        encoder.append(&self.payload_inner.payload_inner.block_number);
        encoder.append(&self.payload_inner.payload_inner.gas_limit);
        encoder.append(&self.payload_inner.payload_inner.gas_used);
        encoder.append(&self.payload_inner.payload_inner.timestamp);
        encoder.append(&self.payload_inner.payload_inner.extra_data);
        encoder.append(&self.payload_inner.payload_inner.base_fee_per_gas);
        encoder.append(&self.payload_inner.payload_inner.block_hash);
        encoder.append(&self.payload_inner.payload_inner.transactions);
        encoder.append(&self.payload_inner.withdrawals);
        encoder.append(&self.blob_gas_used);
        encoder.append(&self.excess_blob_gas);

        encoder.finalize();
    }

    fn ssz_bytes_len(&self) -> usize {
        <ExecutionPayloadV2 as ssz::Encode>::ssz_bytes_len(&self.payload_inner)
            + <u64 as ssz::Encode>::ssz_fixed_len() * 2
    }
}

/// New payload structure for V4. This is required for EIP-7928
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadV4 {
    /// Inner V3 payload
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload_inner: ExecutionPayloadV3,
    /// RLP-encoded block access list as defined in EIP-7928
    pub block_access_list: Bytes,
}

impl ExecutionPayloadV4 {
    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV4`].
    ///
    /// See also [`ExecutionPayloadV3::from_block_unchecked`].
    ///
    /// Note: This re-calculates the block hash.
    pub fn from_block_slow<T, H>(block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader + Sealable,
    {
        Self::from_block_unchecked(block.hash_slow(), block)
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayloadV4`] using the given block hash.
    ///
    /// See also [`ExecutionPayloadV3::from_block_unchecked`].
    pub fn from_block_unchecked<T, H>(block_hash: B256, block: &Block<T, H>) -> Self
    where
        T: Encodable2718,
        H: BlockHeader,
    {
        Self {
            block_access_list: alloy_rlp::encode(block.body.block_access_list.clone().unwrap())
                .into(),
            payload_inner: ExecutionPayloadV3::from_block_unchecked(block_hash, block),
        }
    }

    /// Returns the withdrawals for the payload.
    pub const fn withdrawals(&self) -> &Vec<Withdrawal> {
        &self.payload_inner.payload_inner.withdrawals
    }

    /// Returns the timestamp for the payload.
    pub const fn timestamp(&self) -> u64 {
        self.payload_inner.payload_inner.payload_inner.timestamp
    }

    /// Converts [`ExecutionPayloadV4`] to [`Block`].
    ///
    /// This performs the same conversion as the underlying V3 payload, but inserts the block access
    /// list.
    ///
    /// See also [`ExecutionPayloadV3::try_into_block`].
    pub fn try_into_block<T: Decodable2718>(self) -> Result<Block<T>, PayloadError> {
        self.try_into_block_with(|tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Converts [`ExecutionPayloadV4`] to [`Block`] with a custom transaction mapper.
    ///
    /// See also [`ExecutionPayloadV3::try_into_block_with`].
    pub fn try_into_block_with<T, F, E>(self, f: F) -> Result<Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.into_block_raw()?.try_map_transactions(f).map_err(Into::into)
    }

    /// Converts [`ExecutionPayloadV4`] to [`Block`] with raw [`Bytes`] transactions.
    ///
    /// This is similar to [`Self::try_into_block_with`] but returns the transactions as raw bytes
    /// without any conversion.
    pub fn into_block_raw(self) -> Result<Block<Bytes>, PayloadError> {
        let mut base_block = self.payload_inner.into_block_raw()?;

        base_block.body.block_access_list =
            Some(alloy_rlp::decode_exact(self.block_access_list.as_ref())?);

        Ok(base_block)
    }
}

impl<T: Decodable2718> TryFrom<ExecutionPayloadV4> for Block<T> {
    type Error = PayloadError;

    fn try_from(value: ExecutionPayloadV4) -> Result<Self, Self::Error> {
        value.try_into_block()
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for ExecutionPayloadV4 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        use alloy_eips::eip4844::builder;

        let mut builder = ssz::SszDecoderBuilder::new(bytes);

        builder.register_type::<B256>()?;
        builder.register_type::<Address>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<Bloom>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<Bytes>()?;
        builder.register_type::<U256>()?;
        builder.register_type::<B256>()?;
        builder.register_type::<Vec<Bytes>>()?;
        builder.register_type::<Vec<Withdrawal>>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<u64>()?;
        builder.register_type::<Bytes>()?;

        let mut decoder = builder.build()?;

        Ok(Self {
            payload_inner: ExecutionPayloadV3 {
                payload_inner: ExecutionPayloadV2 {
                    payload_inner: ExecutionPayloadV1 {
                        parent_hash: decoder.decode_next()?,
                        fee_recipient: decoder.decode_next()?,
                        state_root: decoder.decode_next()?,
                        receipts_root: decoder.decode_next()?,
                        logs_bloom: decoder.decode_next()?,
                        prev_randao: decoder.decode_next()?,
                        block_number: decoder.decode_next()?,
                        gas_limit: decoder.decode_next()?,
                        gas_used: decoder.decode_next()?,
                        timestamp: decoder.decode_next()?,
                        extra_data: decoder.decode_next()?,
                        base_fee_per_gas: decoder.decode_next()?,
                        block_hash: decoder.decode_next()?,
                        transactions: decoder.decode_next()?,
                    },
                    withdrawals: decoder.decode_next()?,
                },
                blob_gas_used: decoder.decode_next()?,
                excess_blob_gas: decoder.decode_next()?,
            },
            block_access_list: decoder.decode_next()?,
        })
    }
}

#[cfg(feature = "ssz")]
impl ssz::Encode for ExecutionPayloadV4 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        let offset = <B256 as ssz::Encode>::ssz_fixed_len() * 5
            + <Address as ssz::Encode>::ssz_fixed_len()
            + <Bloom as ssz::Encode>::ssz_fixed_len()
            + <u64 as ssz::Encode>::ssz_fixed_len() * 6
            + <U256 as ssz::Encode>::ssz_fixed_len()
            + ssz::BYTES_PER_LENGTH_OFFSET * 4;

        let mut encoder = ssz::SszEncoder::container(buf, offset);

        encoder.append(&self.payload_inner.payload_inner.payload_inner.parent_hash);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.fee_recipient);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.state_root);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.receipts_root);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.logs_bloom);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.prev_randao);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.block_number);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.gas_limit);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.gas_used);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.timestamp);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.extra_data);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.base_fee_per_gas);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.block_hash);
        encoder.append(&self.payload_inner.payload_inner.payload_inner.transactions);
        encoder.append(&self.payload_inner.payload_inner.withdrawals);
        encoder.append(&self.payload_inner.blob_gas_used);
        encoder.append(&self.payload_inner.excess_blob_gas);
        encoder.append(&self.block_access_list);
        encoder.finalize();
    }

    fn ssz_bytes_len(&self) -> usize {
        <ExecutionPayloadV3 as ssz::Encode>::ssz_bytes_len(&self.payload_inner)
            + ssz::BYTES_PER_LENGTH_OFFSET
            + self.block_access_list.ssz_bytes_len()
    }
}

/// This includes all bundled blob related data of an executed payload.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct BlobsBundleV1 {
    /// All commitments in the bundle.
    pub commitments: Vec<alloy_consensus::Bytes48>,
    /// All proofs in the bundle.
    pub proofs: Vec<alloy_consensus::Bytes48>,
    /// All blobs in the bundle.
    pub blobs: Vec<alloy_consensus::Blob>,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BlobsBundleV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct BlobsBundleRaw {
            commitments: Vec<alloy_consensus::Bytes48>,
            proofs: Vec<alloy_consensus::Bytes48>,
            blobs: Vec<alloy_consensus::Blob>,
        }
        let raw = BlobsBundleRaw::deserialize(deserializer)?;

        if raw.proofs.len() == raw.commitments.len() && raw.proofs.len() == raw.blobs.len() {
            Ok(Self { commitments: raw.commitments, proofs: raw.proofs, blobs: raw.blobs })
        } else {
            Err(serde::de::Error::invalid_length(
                raw.proofs.len(),
                &format!("{}", raw.commitments.len()).as_str(),
            ))
        }
    }
}

impl BlobsBundleV1 {
    /// Creates a new blob bundle from the given sidecars.
    ///
    /// This folds the sidecar fields into single commit, proof, and blob vectors.
    pub fn new(sidecars: impl IntoIterator<Item = BlobTransactionSidecar>) -> Self {
        let (commitments, proofs, blobs) = sidecars.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut commitments, mut proofs, mut blobs), sidecar| {
                commitments.extend(sidecar.commitments);
                proofs.extend(sidecar.proofs);
                blobs.extend(sidecar.blobs);
                (commitments, proofs, blobs)
            },
        );
        Self { commitments, proofs, blobs }
    }

    /// Returns a new empty blobs bundle.
    ///
    /// This is useful for the opstack engine API that expects an empty bundle as part of the
    /// payload for API compatibility reasons.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Take `len` blob data from the bundle.
    ///
    /// # Panics
    ///
    /// If len is more than the blobs bundle len.
    pub fn take(&mut self, len: usize) -> (Vec<Bytes48>, Vec<Bytes48>, Vec<Blob>) {
        (
            self.commitments.drain(0..len).collect(),
            self.proofs.drain(0..len).collect(),
            self.blobs.drain(0..len).collect(),
        )
    }

    /// Returns the sidecar from the bundle
    ///
    /// # Panics
    ///
    /// If len is more than the blobs bundle len.
    pub fn pop_sidecar(&mut self, len: usize) -> BlobTransactionSidecar {
        let (commitments, proofs, blobs) = self.take(len);
        BlobTransactionSidecar { commitments, proofs, blobs }
    }

    /// Converts this bundle into a single [`BlobTransactionSidecar`].
    ///
    /// Returns an error if the bundle doesn't contain the same number of commitments as blobs and
    /// proofs.
    ///
    /// Returns an empty [`BlobTransactionSidecar`] if the bundle is empty.
    #[cfg(feature = "kzg")]
    pub fn try_into_sidecar(
        self,
    ) -> Result<BlobTransactionSidecar, alloy_consensus::error::ValueError<Self>> {
        if self.commitments.len() != self.proofs.len() || self.commitments.len() != self.blobs.len()
        {
            return Err(alloy_consensus::error::ValueError::new(self, "length mismatch"));
        }

        let Self { commitments, proofs, blobs } = self;
        Ok(BlobTransactionSidecar { blobs, commitments, proofs })
    }
}

impl From<Vec<BlobTransactionSidecar>> for BlobsBundleV1 {
    fn from(sidecars: Vec<BlobTransactionSidecar>) -> Self {
        Self::new(sidecars)
    }
}

impl FromIterator<BlobTransactionSidecar> for BlobsBundleV1 {
    fn from_iter<T: IntoIterator<Item = BlobTransactionSidecar>>(iter: T) -> Self {
        Self::new(iter)
    }
}

#[cfg(feature = "kzg")]
impl TryFrom<BlobsBundleV1> for BlobTransactionSidecar {
    type Error = alloy_consensus::error::ValueError<BlobsBundleV1>;

    fn try_from(value: BlobsBundleV1) -> Result<Self, Self::Error> {
        value.try_into_sidecar()
    }
}

/// This includes all bundled blob related data of an executed payload.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct BlobsBundleV2 {
    /// All commitments in the bundle.
    pub commitments: Vec<alloy_consensus::Bytes48>,
    /// All cell proofs in the bundle.
    pub proofs: Vec<alloy_consensus::Bytes48>,
    /// All blobs in the bundle.
    pub blobs: Vec<alloy_consensus::Blob>,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BlobsBundleV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct BlobsBundleRaw {
            commitments: Vec<alloy_consensus::Bytes48>,
            proofs: Vec<alloy_consensus::Bytes48>,
            blobs: Vec<alloy_consensus::Blob>,
        }
        let raw = BlobsBundleRaw::deserialize(deserializer)?;

        if raw.proofs.len() == raw.blobs.len() * CELLS_PER_EXT_BLOB
            && raw.commitments.len() == raw.blobs.len()
        {
            Ok(Self { commitments: raw.commitments, proofs: raw.proofs, blobs: raw.blobs })
        } else {
            Err(serde::de::Error::invalid_length(
                raw.proofs.len(),
                &format!("{}", raw.commitments.len() * CELLS_PER_EXT_BLOB).as_str(),
            ))
        }
    }
}

#[cfg(feature = "ssz")]
impl ssz::Decode for BlobsBundleV2 {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        #[derive(ssz_derive::Decode)]
        struct BlobsBundleRaw {
            commitments: Vec<alloy_consensus::Bytes48>,
            proofs: Vec<alloy_consensus::Bytes48>,
            blobs: Vec<alloy_consensus::Blob>,
        }

        let raw = BlobsBundleRaw::from_ssz_bytes(bytes)?;

        if raw.proofs.len() == raw.blobs.len() * CELLS_PER_EXT_BLOB
            && raw.commitments.len() == raw.blobs.len()
        {
            Ok(Self { commitments: raw.commitments, proofs: raw.proofs, blobs: raw.blobs })
        } else {
            Err(ssz::DecodeError::BytesInvalid(
                format!(
                    "Invalid BlobsBundleV2: expected {} proofs and {} commitments for {} blobs, got {} proofs and {} commitments",
                    raw.blobs.len() * CELLS_PER_EXT_BLOB,
                    raw.blobs.len(),
                    raw.blobs.len(),
                    raw.proofs.len(),
                    raw.commitments.len()
                )
            ))
        }
    }
}

impl BlobsBundleV2 {
    /// Creates a new blob bundle from the given sidecars.
    ///
    /// This folds the sidecar fields into single commit, proof, and blob vectors.
    pub fn new(sidecars: impl IntoIterator<Item = BlobTransactionSidecarEip7594>) -> Self {
        let (commitments, proofs, blobs) = sidecars.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut commitments, mut proofs, mut blobs), sidecar| {
                commitments.extend(sidecar.commitments);
                proofs.extend(sidecar.cell_proofs);
                blobs.extend(sidecar.blobs);
                (commitments, proofs, blobs)
            },
        );
        Self { commitments, proofs, blobs }
    }

    /// Returns a new empty blobs bundle.
    ///
    /// This is useful for the opstack engine API that expects an empty bundle as part of the
    /// payload for API compatibility reasons.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Take `len` blob data from the bundle.
    ///
    /// Note this will take `len * CELLS_PER_EXT_BLOB` proofs.
    ///
    /// # Panics
    ///
    /// If len is more than the blobs bundle len.
    pub fn take(&mut self, len: usize) -> (Vec<Bytes48>, Vec<Bytes48>, Vec<Blob>) {
        (
            self.commitments.drain(0..len).collect(),
            self.proofs.drain(0..len * CELLS_PER_EXT_BLOB).collect(),
            self.blobs.drain(0..len).collect(),
        )
    }

    /// Returns the sidecar from the bundle
    ///
    /// # Panics
    ///
    /// If len is more than the blobs bundle len.
    pub fn pop_sidecar(&mut self, len: usize) -> BlobTransactionSidecarEip7594 {
        let (commitments, cell_proofs, blobs) = self.take(len);
        BlobTransactionSidecarEip7594 { commitments, cell_proofs, blobs }
    }

    /// Converts this bundle into a single [`BlobTransactionSidecarEip7594`].
    ///
    /// Returns an error if the bundle doesn't contain the correct number of cell proofs
    /// (expected blobs.len() * CELLS_PER_EXT_BLOB) or if the commitments length doesn't
    /// match the blobs length.
    ///
    /// Returns an empty [`BlobTransactionSidecarEip7594`] if the bundle is empty.
    #[cfg(feature = "kzg")]
    pub fn try_into_sidecar(
        self,
    ) -> Result<BlobTransactionSidecarEip7594, alloy_consensus::error::ValueError<Self>> {
        let expected_cell_proofs_len = self.blobs.len() * CELLS_PER_EXT_BLOB;
        if self.proofs.len() != expected_cell_proofs_len {
            let msg = format!(
                "cell proofs length mismatch, expected {expected_cell_proofs_len}, has {}",
                self.proofs.len()
            );
            return Err(alloy_consensus::error::ValueError::new(self, msg));
        }

        if self.commitments.len() != self.blobs.len() {
            let msg = format!(
                "commitments length ({}) mismatch, expected blob length ({})",
                self.commitments.len(),
                self.blobs.len()
            );
            return Err(alloy_consensus::error::ValueError::new(self, msg));
        }

        let Self { commitments, proofs, blobs } = self;
        Ok(BlobTransactionSidecarEip7594 { blobs, commitments, cell_proofs: proofs })
    }
}

impl From<Vec<BlobTransactionSidecarEip7594>> for BlobsBundleV2 {
    fn from(sidecars: Vec<BlobTransactionSidecarEip7594>) -> Self {
        Self::new(sidecars)
    }
}

impl FromIterator<BlobTransactionSidecarEip7594> for BlobsBundleV2 {
    fn from_iter<T: IntoIterator<Item = BlobTransactionSidecarEip7594>>(iter: T) -> Self {
        Self::new(iter)
    }
}

#[cfg(feature = "kzg")]
impl TryFrom<BlobsBundleV2> for BlobTransactionSidecarEip7594 {
    type Error = alloy_consensus::error::ValueError<BlobsBundleV2>;

    fn try_from(value: BlobsBundleV2) -> Result<Self, Self::Error> {
        value.try_into_sidecar()
    }
}

/// An execution payload, which can be either [ExecutionPayloadV1], [ExecutionPayloadV2], or
/// [ExecutionPayloadV3].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum ExecutionPayload {
    /// V1 payload
    V1(ExecutionPayloadV1),
    /// V2 payload
    V2(ExecutionPayloadV2),
    /// V3 payload
    V3(ExecutionPayloadV3),
    /// V4 payload
    V4(ExecutionPayloadV4),
}

impl ExecutionPayload {
    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayload`] and also returns the
    /// [`ExecutionPayloadSidecar`] extracted from the block.
    ///
    /// See also [`ExecutionPayloadV3::from_block_unchecked`].
    /// See also [`ExecutionPayloadSidecar::from_block`].
    ///
    /// Note: This re-calculates the block hash.
    pub fn from_block_slow<T, H>(block: &Block<T, H>) -> (Self, ExecutionPayloadSidecar)
    where
        T: Encodable2718 + Transaction,
        H: BlockHeader + Sealable,
    {
        Self::from_block_unchecked(block.hash_slow(), block)
    }

    /// Converts [`alloy_consensus::Block`] to [`ExecutionPayload`] and also returns the
    /// [`ExecutionPayloadSidecar`] extracted from the block.
    ///
    /// See also [`ExecutionPayloadV3::from_block_unchecked`].
    /// See also [`ExecutionPayloadSidecar::from_block`].
    pub fn from_block_unchecked<T, H>(
        block_hash: B256,
        block: &Block<T, H>,
    ) -> (Self, ExecutionPayloadSidecar)
    where
        T: Encodable2718 + Transaction,
        H: BlockHeader,
    {
        let sidecar = ExecutionPayloadSidecar::from_block(block);

        let execution_payload = if block.header.block_access_list_hash().is_some() {
            // block with block access list: V4
            Self::V4(ExecutionPayloadV4::from_block_unchecked(block_hash, block))
        } else if block.header.parent_beacon_block_root().is_some() {
            // block with parent beacon block root: V3
            Self::V3(ExecutionPayloadV3::from_block_unchecked(block_hash, block))
        } else if block.body.withdrawals.is_some() {
            // block with withdrawals: V2
            Self::V2(ExecutionPayloadV2::from_block_unchecked(block_hash, block))
        } else {
            // otherwise V1
            Self::V1(ExecutionPayloadV1::from_block_unchecked(block_hash, block))
        };

        (execution_payload, sidecar)
    }

    /// Tries to create a new unsealed block from the given payload and payload sidecar.
    ///
    /// Performs additional validation of `extra_data` and `base_fee_per_gas` fields.
    ///
    /// # Note
    ///
    /// The log bloom is assumed to be validated during serialization.
    ///
    /// See <https://github.com/ethereum/go-ethereum/blob/79a478bb6176425c2400e949890e668a3d9a3d05/core/beacon/types.go#L145>
    pub fn try_into_block_with_sidecar<T: Decodable2718>(
        self,
        sidecar: &ExecutionPayloadSidecar,
    ) -> Result<Block<T>, PayloadError> {
        self.try_into_block_with_sidecar_with(sidecar, |tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Converts [`ExecutionPayload`] to [`Block`] with sidecar and a custom transaction mapper.
    ///
    /// The log bloom is assumed to be validated during serialization.
    ///
    /// See <https://github.com/ethereum/go-ethereum/blob/79a478bb6176425c2400e949890e668a3d9a3d05/core/beacon/types.go#L145>
    pub fn try_into_block_with_sidecar_with<T, F, E>(
        self,
        sidecar: &ExecutionPayloadSidecar,
        f: F,
    ) -> Result<Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.into_block_with_sidecar_raw(sidecar)?.try_map_transactions(f).map_err(Into::into)
    }

    /// Converts [`ExecutionPayload`] to [`Block`] with raw [`Bytes`] transactions and sidecar.
    ///
    /// This is similar to [`Self::try_into_block_with_sidecar_with`] but returns the transactions
    /// as raw bytes without any conversion.
    pub fn into_block_with_sidecar_raw(
        self,
        sidecar: &ExecutionPayloadSidecar,
    ) -> Result<Block<Bytes>, PayloadError> {
        let mut base_block = self.into_block_raw()?;
        base_block.header.parent_beacon_block_root = sidecar.parent_beacon_block_root();
        base_block.header.requests_hash = sidecar.requests_hash();
        Ok(base_block)
    }

    /// Converts [`ExecutionPayloadV1`] to [`Block`].
    ///
    /// Caution: This does not set fields that are not part of the payload and only part of the
    /// [`ExecutionPayloadSidecar`]:
    /// - parent_beacon_block_root
    /// - requests_hash
    ///
    /// See also: [`ExecutionPayload::try_into_block_with_sidecar`]
    pub fn try_into_block<T: Decodable2718>(self) -> Result<Block<T>, PayloadError> {
        self.try_into_block_with(|tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Converts [`ExecutionPayload`] to [`Block`] with a custom transaction mapper.
    ///
    /// Caution: This does not set fields that are not part of the payload and only part of the
    /// [`ExecutionPayloadSidecar`]:
    /// - parent_beacon_block_root
    /// - requests_hash
    ///
    /// See also: [`ExecutionPayload::try_into_block_with_sidecar`]
    pub fn try_into_block_with<T, F, E>(self, f: F) -> Result<Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.into_block_raw()?.try_map_transactions(f).map_err(Into::into)
    }

    /// Converts [`ExecutionPayload`] to [`Block`] with raw [`Bytes`] transactions.
    ///
    /// This is similar to [`Self::try_into_block_with`] but returns the transactions as raw bytes
    /// without any conversion.
    pub fn into_block_raw(self) -> Result<Block<Bytes>, PayloadError> {
        match self {
            Self::V1(payload) => payload.into_block_raw(),
            Self::V2(payload) => payload.into_block_raw(),
            Self::V3(payload) => payload.into_block_raw(),
            Self::V4(payload) => payload.into_block_raw(),
        }
    }

    /// Returns a reference to the V1 payload.
    pub const fn as_v1(&self) -> &ExecutionPayloadV1 {
        match self {
            Self::V1(payload) => payload,
            Self::V2(payload) => &payload.payload_inner,
            Self::V3(payload) => &payload.payload_inner.payload_inner,
            Self::V4(payload) => &payload.payload_inner.payload_inner.payload_inner,
        }
    }

    /// Returns a mutable reference to the V1 payload.
    pub const fn as_v1_mut(&mut self) -> &mut ExecutionPayloadV1 {
        match self {
            Self::V1(payload) => payload,
            Self::V2(payload) => &mut payload.payload_inner,
            Self::V3(payload) => &mut payload.payload_inner.payload_inner,
            Self::V4(payload) => &mut payload.payload_inner.payload_inner.payload_inner,
        }
    }

    /// Consumes the payload and returns the V1 payload.
    pub fn into_v1(self) -> ExecutionPayloadV1 {
        match self {
            Self::V1(payload) => payload,
            Self::V2(payload) => payload.payload_inner,
            Self::V3(payload) => payload.payload_inner.payload_inner,
            Self::V4(payload) => payload.payload_inner.payload_inner.payload_inner,
        }
    }

    /// Returns a reference to the V2 payload, if any.
    pub const fn as_v2(&self) -> Option<&ExecutionPayloadV2> {
        match self {
            Self::V1(_) => None,
            Self::V2(payload) => Some(payload),
            Self::V3(payload) => Some(&payload.payload_inner),
            Self::V4(payload) => Some(&payload.payload_inner.payload_inner),
        }
    }

    /// Returns a mutable reference to the V2 payload, if any.
    pub const fn as_v2_mut(&mut self) -> Option<&mut ExecutionPayloadV2> {
        match self {
            Self::V1(_) => None,
            Self::V2(payload) => Some(payload),
            Self::V3(payload) => Some(&mut payload.payload_inner),
            Self::V4(payload) => Some(&mut payload.payload_inner.payload_inner),
        }
    }

    /// Returns a reference to the V2 payload, if any.
    pub const fn as_v3(&self) -> Option<&ExecutionPayloadV3> {
        match self {
            Self::V1(_) | Self::V2(_) => None,
            Self::V3(payload) => Some(payload),
            Self::V4(payload) => Some(&payload.payload_inner),
        }
    }

    /// Returns a mutable reference to the V2 payload, if any.
    pub const fn as_v3_mut(&mut self) -> Option<&mut ExecutionPayloadV3> {
        match self {
            Self::V1(_) | Self::V2(_) => None,
            Self::V3(payload) => Some(payload),
            Self::V4(payload) => Some(&mut payload.payload_inner),
        }
    }

    /// Returns the withdrawals for the payload.
    pub const fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        match self.as_v2() {
            Some(payload) => Some(&payload.withdrawals),
            None => None,
        }
    }

    /// Returns the transactions for the payload.
    pub const fn transactions(&self) -> &Vec<Bytes> {
        &self.as_v1().transactions
    }

    /// Returns a mutable reference to the transactions for the payload.
    pub const fn transactions_mut(&mut self) -> &mut Vec<Bytes> {
        &mut self.as_v1_mut().transactions
    }

    /// Extracts essential information into one container type.
    pub fn header_info(&self) -> HeaderInfo {
        HeaderInfo {
            number: self.block_number(),
            beneficiary: self.fee_recipient(),
            timestamp: self.timestamp(),
            gas_limit: self.gas_limit(),
            base_fee_per_gas: Some(self.saturated_base_fee_per_gas()),
            excess_blob_gas: self.excess_blob_gas(),
            blob_gas_used: self.blob_gas_used(),
            difficulty: U256::ZERO,
            mix_hash: Some(self.prev_randao()),
        }
    }

    /// Returns the gas limit for the payload.
    ///
    /// Note: this returns the u64 saturated base fee, but it is specified as [`U256`].
    pub fn saturated_base_fee_per_gas(&self) -> u64 {
        self.as_v1().base_fee_per_gas.saturating_to()
    }

    /// Returns the blob gas used for the payload.
    pub fn blob_gas_used(&self) -> Option<u64> {
        self.as_v3().map(|payload| payload.blob_gas_used)
    }

    /// Returns the excess blob gas for the payload.
    pub fn excess_blob_gas(&self) -> Option<u64> {
        self.as_v3().map(|payload| payload.excess_blob_gas)
    }

    /// Returns the gas limit for the payload.
    pub const fn gas_limit(&self) -> u64 {
        self.as_v1().gas_limit
    }

    /// Returns the fee recipient.
    pub const fn fee_recipient(&self) -> Address {
        self.as_v1().fee_recipient
    }

    /// Returns the timestamp for the payload.
    pub const fn timestamp(&self) -> u64 {
        self.as_v1().timestamp
    }

    /// Returns the parent hash for the payload.
    pub const fn parent_hash(&self) -> B256 {
        self.as_v1().parent_hash
    }

    /// Returns the block hash for the payload.
    pub const fn block_hash(&self) -> B256 {
        self.as_v1().block_hash
    }

    /// Returns the block number for this payload.
    pub const fn block_number(&self) -> u64 {
        self.as_v1().block_number
    }

    /// Returns the block number for this payload.
    pub const fn block_num_hash(&self) -> BlockNumHash {
        self.as_v1().block_num_hash()
    }

    /// Returns the prev randao for this payload.
    pub const fn prev_randao(&self) -> B256 {
        self.as_v1().prev_randao
    }

    /// Returns the blob fee for _this_ block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None
    pub fn blob_fee(&self, blob_params: BlobParams) -> Option<u128> {
        Some(blob_params.calc_blob_fee(self.excess_blob_gas()?))
    }

    /// Returns the blob fee for the next block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None.
    ///
    /// See also [Self::next_block_excess_blob_gas]
    pub fn next_block_blob_fee(&self, blob_params: BlobParams) -> Option<u128> {
        Some(blob_params.calc_blob_fee(self.next_block_excess_blob_gas(blob_params)?))
    }

    /// Calculate base fee for next block according to the EIP-1559 spec.
    ///
    /// Returns a `None` if no base fee is set, no EIP-1559 support
    pub fn next_block_base_fee(&self, base_fee_params: BaseFeeParams) -> Option<u64> {
        self.as_v1().next_block_base_fee(base_fee_params)
    }

    /// Calculate excess blob gas for the next block according to the EIP-4844
    /// spec.
    ///
    /// Returns a `None` if no excess blob gas is set, no EIP-4844 support
    pub fn next_block_excess_blob_gas(&self, blob_params: BlobParams) -> Option<u64> {
        Some(blob_params.next_block_excess_blob_gas(self.excess_blob_gas()?, self.blob_gas_used()?))
    }

    /// Convenience function for [`Self::next_block_excess_blob_gas`] with an optional
    /// [`BlobParams`] argument.
    ///
    /// Returns `None` if the `blob_params` are `None`.
    pub fn maybe_next_block_excess_blob_gas(&self, blob_params: Option<BlobParams>) -> Option<u64> {
        self.next_block_excess_blob_gas(blob_params?)
    }
}

impl From<ExecutionPayloadV1> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV1) -> Self {
        Self::V1(payload)
    }
}

impl From<ExecutionPayloadV2> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV2) -> Self {
        Self::V2(payload)
    }
}

impl From<ExecutionPayloadFieldV2> for ExecutionPayload {
    fn from(payload: ExecutionPayloadFieldV2) -> Self {
        payload.into_payload()
    }
}

impl From<ExecutionPayloadV3> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV3) -> Self {
        Self::V3(payload)
    }
}

impl From<ExecutionPayloadV4> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV4) -> Self {
        Self::V4(payload)
    }
}

impl<T: Decodable2718> TryFrom<ExecutionPayload> for Block<T> {
    type Error = PayloadError;

    fn try_from(value: ExecutionPayload) -> Result<Self, Self::Error> {
        value.try_into_block()
    }
}

// Deserializes untagged ExecutionPayload depending on the available fields
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ExecutionPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use alloy_primitives::U64;

        struct ExecutionPayloadVisitor;

        impl<'de> serde::de::Visitor<'de> for ExecutionPayloadVisitor {
            type Value = ExecutionPayload;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a valid ExecutionPayload object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // this currently rejects unknown fields
                #[cfg_attr(feature = "serde", derive(serde::Deserialize))]
                #[cfg_attr(feature = "serde", serde(field_identifier, rename_all = "camelCase"))]
                enum Fields {
                    ParentHash,
                    FeeRecipient,
                    StateRoot,
                    ReceiptsRoot,
                    LogsBloom,
                    PrevRandao,
                    BlockNumber,
                    GasLimit,
                    GasUsed,
                    Timestamp,
                    ExtraData,
                    BaseFeePerGas,
                    BlockHash,
                    Transactions,
                    // V2
                    Withdrawals,
                    // V3
                    BlobGasUsed,
                    ExcessBlobGas,
                    // V4
                    BlockAccessList,
                }

                let mut parent_hash = None;
                let mut fee_recipient = None;
                let mut state_root = None;
                let mut receipts_root = None;
                let mut logs_bloom = None;
                let mut prev_randao = None;
                let mut block_number = None;
                let mut gas_limit = None;
                let mut gas_used = None;
                let mut timestamp = None;
                let mut extra_data = None;
                let mut base_fee_per_gas = None;
                let mut block_hash = None;
                let mut transactions = None;
                let mut withdrawals = None;
                let mut blob_gas_used = None;
                let mut excess_blob_gas = None;
                let mut block_access_list = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::ParentHash => parent_hash = Some(map.next_value()?),
                        Fields::FeeRecipient => fee_recipient = Some(map.next_value()?),
                        Fields::StateRoot => state_root = Some(map.next_value()?),
                        Fields::ReceiptsRoot => receipts_root = Some(map.next_value()?),
                        Fields::LogsBloom => logs_bloom = Some(map.next_value()?),
                        Fields::PrevRandao => prev_randao = Some(map.next_value()?),
                        Fields::BlockNumber => {
                            let raw = map.next_value::<U64>()?;
                            block_number = Some(raw.to());
                        }
                        Fields::GasLimit => {
                            let raw = map.next_value::<U64>()?;
                            gas_limit = Some(raw.to());
                        }
                        Fields::GasUsed => {
                            let raw = map.next_value::<U64>()?;
                            gas_used = Some(raw.to());
                        }
                        Fields::Timestamp => {
                            let raw = map.next_value::<U64>()?;
                            timestamp = Some(raw.to());
                        }
                        Fields::ExtraData => extra_data = Some(map.next_value()?),
                        Fields::BaseFeePerGas => base_fee_per_gas = Some(map.next_value()?),
                        Fields::BlockHash => block_hash = Some(map.next_value()?),
                        Fields::Transactions => transactions = Some(map.next_value()?),
                        Fields::Withdrawals => withdrawals = Some(map.next_value()?),
                        Fields::BlobGasUsed => {
                            let raw = map.next_value::<U64>()?;
                            blob_gas_used = Some(raw.to());
                        }
                        Fields::ExcessBlobGas => {
                            let raw = map.next_value::<U64>()?;
                            excess_blob_gas = Some(raw.to());
                        }
                        Fields::BlockAccessList => block_access_list = Some(map.next_value()?),
                    }
                }

                let parent_hash =
                    parent_hash.ok_or_else(|| serde::de::Error::missing_field("parentHash"))?;
                let fee_recipient =
                    fee_recipient.ok_or_else(|| serde::de::Error::missing_field("feeRecipient"))?;
                let state_root =
                    state_root.ok_or_else(|| serde::de::Error::missing_field("stateRoot"))?;
                let receipts_root =
                    receipts_root.ok_or_else(|| serde::de::Error::missing_field("receiptsRoot"))?;
                let logs_bloom =
                    logs_bloom.ok_or_else(|| serde::de::Error::missing_field("logsBloom"))?;
                let prev_randao =
                    prev_randao.ok_or_else(|| serde::de::Error::missing_field("prevRandao"))?;
                let block_number =
                    block_number.ok_or_else(|| serde::de::Error::missing_field("blockNumber"))?;
                let gas_limit =
                    gas_limit.ok_or_else(|| serde::de::Error::missing_field("gasLimit"))?;
                let gas_used =
                    gas_used.ok_or_else(|| serde::de::Error::missing_field("gasUsed"))?;
                let timestamp =
                    timestamp.ok_or_else(|| serde::de::Error::missing_field("timestamp"))?;
                let extra_data =
                    extra_data.ok_or_else(|| serde::de::Error::missing_field("extraData"))?;
                let base_fee_per_gas = base_fee_per_gas
                    .ok_or_else(|| serde::de::Error::missing_field("baseFeePerGas"))?;
                let block_hash =
                    block_hash.ok_or_else(|| serde::de::Error::missing_field("blockHash"))?;
                let transactions =
                    transactions.ok_or_else(|| serde::de::Error::missing_field("transactions"))?;

                let v1 = ExecutionPayloadV1 {
                    parent_hash,
                    fee_recipient,
                    state_root,
                    receipts_root,
                    logs_bloom,
                    prev_randao,
                    block_number,
                    gas_limit,
                    gas_used,
                    timestamp,
                    extra_data,
                    base_fee_per_gas,
                    block_hash,
                    transactions,
                };

                let Some(withdrawals) = withdrawals else {
                    return if blob_gas_used.is_none() && excess_blob_gas.is_none() {
                        Ok(ExecutionPayload::V1(v1))
                    } else {
                        Err(serde::de::Error::custom("invalid enum variant"))
                    };
                };

                if let (Some(blob_gas_used), Some(excess_blob_gas)) =
                    (blob_gas_used, excess_blob_gas)
                {
                    return Ok(ExecutionPayload::V3(ExecutionPayloadV3 {
                        payload_inner: ExecutionPayloadV2 { payload_inner: v1, withdrawals },
                        blob_gas_used,
                        excess_blob_gas,
                    }));
                }
                if let (Some(blob_gas_used), Some(excess_blob_gas), Some(block_access_list)) =
                    (blob_gas_used, excess_blob_gas, block_access_list)
                {
                    return Ok(ExecutionPayload::V4(ExecutionPayloadV4 {
                        payload_inner: ExecutionPayloadV3 {
                            payload_inner: ExecutionPayloadV2 { payload_inner: v1, withdrawals },
                            blob_gas_used,
                            excess_blob_gas,
                        },
                        block_access_list,
                    }));
                }

                // reject incomplete V3 payloads even if they could construct a valid V2
                if blob_gas_used.is_some() || excess_blob_gas.is_some() {
                    return Err(serde::de::Error::custom("invalid enum variant"));
                }

                Ok(ExecutionPayload::V2(ExecutionPayloadV2 { payload_inner: v1, withdrawals }))
            }
        }

        const FIELDS: &[&str] = &[
            "parentHash",
            "feeRecipient",
            "stateRoot",
            "receiptsRoot",
            "logsBloom",
            "prevRandao",
            "blockNumber",
            "gasLimit",
            "gasUsed",
            "timestamp",
            "extraData",
            "baseFeePerGas",
            "blockHash",
            "transactions",
            "withdrawals",
            "blobGasUsed",
            "excessBlobGas",
            "blockAccessList",
        ];
        deserializer.deserialize_struct("ExecutionPayload", FIELDS, ExecutionPayloadVisitor)
    }
}

/// This structure contains a body of an execution payload.
///
/// See also: <https://github.com/ethereum/execution-apis/blob/6452a6b194d7db269bf1dbd087a267251d3cc7f8/src/engine/shanghai.md#executionpayloadbodyv1>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadBodyV1 {
    /// Enveloped encoded transactions.
    pub transactions: Vec<Bytes>,
    /// All withdrawals in the block.
    ///
    /// Will always be `None` if pre shanghai.
    pub withdrawals: Option<Vec<Withdrawal>>,
}

impl ExecutionPayloadBodyV1 {
    /// Creates an [`ExecutionPayloadBodyV1`] from the given withdrawals and transactions
    pub fn new<'a, T>(
        withdrawals: Option<Withdrawals>,
        transactions: impl IntoIterator<Item = &'a T>,
    ) -> Self
    where
        T: Encodable2718 + 'a,
    {
        Self {
            transactions: transactions.into_iter().map(|tx| tx.encoded_2718().into()).collect(),
            withdrawals: withdrawals.map(Withdrawals::into_inner),
        }
    }

    /// Converts a [`alloy_consensus::Block`] into an execution payload body.
    pub fn from_block<T: Encodable2718, H>(block: Block<T, H>) -> Self {
        Self::new(block.body.withdrawals.clone(), block.body.transactions())
    }
}

impl<T: Encodable2718, H> From<Block<T, H>> for ExecutionPayloadBodyV1 {
    fn from(value: Block<T, H>) -> Self {
        Self::from_block(value)
    }
}

/// This structure contains the attributes required to initiate a payload build process in the
/// context of an `engine_forkchoiceUpdated` call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct PayloadAttributes {
    /// Value for the `timestamp` field of the new payload
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub timestamp: u64,
    /// Value for the `prevRandao` field of the new payload
    pub prev_randao: B256,
    /// Suggested value for the `feeRecipient` field of the new payload
    pub suggested_fee_recipient: Address,
    /// Array of [`Withdrawal`] enabled with V2
    /// See <https://github.com/ethereum/execution-apis/blob/6452a6b194d7db269bf1dbd087a267251d3cc7f8/src/engine/shanghai.md#payloadattributesv2>
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Root of the parent beacon block enabled with V3.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#payloadattributesv3>
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub parent_beacon_block_root: Option<B256>,
}

/// This structure contains the result of processing a payload or fork choice update.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct PayloadStatus {
    /// The status of the payload.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub status: PayloadStatusEnum,
    /// Hash of the most recent valid block in the branch defined by payload and its ancestors
    pub latest_valid_hash: Option<B256>,
}

impl PayloadStatus {
    /// Initializes a new payload status.
    pub const fn new(status: PayloadStatusEnum, latest_valid_hash: Option<B256>) -> Self {
        Self { status, latest_valid_hash }
    }

    /// Creates a new payload status from the given status.
    pub const fn from_status(status: PayloadStatusEnum) -> Self {
        Self { status, latest_valid_hash: None }
    }

    /// Sets the latest valid hash.
    pub const fn with_latest_valid_hash(mut self, latest_valid_hash: B256) -> Self {
        self.latest_valid_hash = Some(latest_valid_hash);
        self
    }

    /// Sets the latest valid hash if it's not None.
    pub const fn maybe_latest_valid_hash(mut self, latest_valid_hash: Option<B256>) -> Self {
        self.latest_valid_hash = latest_valid_hash;
        self
    }

    /// Returns true if the payload status is syncing.
    pub const fn is_syncing(&self) -> bool {
        self.status.is_syncing()
    }

    /// Returns true if the payload status is valid.
    pub const fn is_valid(&self) -> bool {
        self.status.is_valid()
    }

    /// Returns true if the payload status is invalid.
    pub const fn is_invalid(&self) -> bool {
        self.status.is_invalid()
    }
}

impl core::fmt::Display for PayloadStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PayloadStatus {{ status: {}, latestValidHash: {:?} }}",
            self.status, self.latest_valid_hash
        )
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PayloadStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("status", self.status.as_str())?;
        map.serialize_entry("latestValidHash", &self.latest_valid_hash)?;
        map.serialize_entry("validationError", &self.status.validation_error())?;
        map.end()
    }
}

impl From<PayloadError> for PayloadStatusEnum {
    fn from(error: PayloadError) -> Self {
        Self::Invalid { validation_error: error.to_string() }
    }
}

/// Represents the status response of a payload.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PayloadStatusEnum {
    /// VALID is returned by the engine API in the following calls:
    ///   - newPayload:       if the payload was already known or was just validated and executed
    ///   - forkchoiceUpdate: if the chain accepted the reorg (might ignore if it's stale)
    Valid,

    /// INVALID is returned by the engine API in the following calls:
    ///   - newPayload:       if the payload failed to execute on top of the local chain
    ///   - forkchoiceUpdate: if the new head is unknown, pre-merge, or reorg to it fails
    Invalid {
        /// The error message for the invalid payload.
        #[cfg_attr(feature = "serde", serde(rename = "validationError"))]
        validation_error: String,
    },

    /// SYNCING is returned by the engine API in the following calls:
    ///   - newPayload:       if the payload was accepted on top of an active sync
    ///   - forkchoiceUpdate: if the new head was seen before, but not part of the chain
    Syncing,

    /// ACCEPTED is returned by the engine API in the following calls:
    ///   - newPayload: if the payload was accepted, but not processed (side chain)
    Accepted,
}

impl PayloadStatusEnum {
    /// Returns the string representation of the payload status.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Valid => "VALID",
            Self::Invalid { .. } => "INVALID",
            Self::Syncing => "SYNCING",
            Self::Accepted => "ACCEPTED",
        }
    }

    /// Returns the validation error if the payload status is invalid.
    pub fn validation_error(&self) -> Option<&str> {
        match self {
            Self::Invalid { validation_error } => Some(validation_error),
            _ => None,
        }
    }

    /// Returns true if the payload status is syncing.
    pub const fn is_syncing(&self) -> bool {
        matches!(self, Self::Syncing)
    }

    /// Returns true if the payload status is valid.
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// Returns true if the payload status is invalid.
    pub const fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid { .. })
    }
}

impl core::fmt::Display for PayloadStatusEnum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Invalid { validation_error } => {
                f.write_str(self.as_str())?;
                f.write_str(": ")?;
                f.write_str(validation_error.as_str())
            }
            _ => f.write_str(self.as_str()),
        }
    }
}

/// Struct aggregating [`ExecutionPayload`] and [`ExecutionPayloadSidecar`] and encapsulating
/// complete payload supplied for execution.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionData {
    /// Execution payload.
    pub payload: ExecutionPayload,
    /// Additional fork-specific fields.
    pub sidecar: ExecutionPayloadSidecar,
}

impl ExecutionData {
    /// Creates new instance of [`ExecutionData`].
    pub const fn new(payload: ExecutionPayload, sidecar: ExecutionPayloadSidecar) -> Self {
        Self { payload, sidecar }
    }

    /// Conversion from [`alloy_consensus::Block`]. Also returns the [`ExecutionPayloadSidecar`]
    /// extracted from the block.
    ///
    /// For the [`ExecutionPayloadSidecar`] this is expected to use just the requests hash, because
    /// the [`Requests`] are not part of the block/header. See also
    /// [`RequestsOrHash`](alloy_eips::eip7685::RequestsOrHash).
    ///
    /// See also [`ExecutionPayload::from_block_unchecked`].
    pub fn from_block_unchecked<T, H>(block_hash: B256, block: &Block<T, H>) -> Self
    where
        T: Encodable2718 + Transaction,
        H: BlockHeader,
    {
        let (payload, sidecar) = ExecutionPayload::from_block_unchecked(block_hash, block);
        Self::new(payload, sidecar)
    }

    /// Returns the parent hash of the block.
    pub const fn parent_hash(&self) -> B256 {
        self.payload.parent_hash()
    }

    /// Returns the hash of the block.
    pub const fn block_hash(&self) -> B256 {
        self.payload.block_hash()
    }

    /// Returns the number of the block.
    pub const fn block_number(&self) -> u64 {
        self.payload.block_number()
    }

    /// Returns the parent beacon block root, if any.
    pub fn parent_beacon_block_root(&self) -> Option<B256> {
        self.sidecar.parent_beacon_block_root()
    }

    /// Return the withdrawals for the payload or attributes.
    pub const fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        self.payload.withdrawals()
    }

    /// Tries to create a new unsealed block from the given payload and payload sidecar.
    ///
    /// Performs additional validation of `extra_data` and `base_fee_per_gas` fields.
    ///
    /// # Note
    ///
    /// The log bloom is assumed to be validated during serialization.
    ///
    /// See <https://github.com/ethereum/go-ethereum/blob/79a478bb6176425c2400e949890e668a3d9a3d05/core/beacon/types.go#L145>
    pub fn try_into_block<T: Decodable2718>(
        self,
    ) -> Result<alloy_consensus::Block<T>, PayloadError> {
        self.try_into_block_with(|tx| {
            T::decode_2718_exact(tx.as_ref())
                .map_err(alloy_rlp::Error::from)
                .map_err(PayloadError::from)
        })
    }

    /// Tries to create a new unsealed block from the given payload and payload sidecar with a
    /// custom transaction mapper.
    ///
    /// Performs additional validation of `extra_data` and `base_fee_per_gas` fields.
    ///
    /// # Note
    ///
    /// The log bloom is assumed to be validated during serialization.
    ///
    /// See <https://github.com/ethereum/go-ethereum/blob/79a478bb6176425c2400e949890e668a3d9a3d05/core/beacon/types.go#L145>
    pub fn try_into_block_with<T, F, E>(
        self,
        f: F,
    ) -> Result<alloy_consensus::Block<T>, PayloadError>
    where
        F: FnMut(Bytes) -> Result<T, E>,
        E: Into<PayloadError>,
    {
        self.payload.try_into_block_with_sidecar_with(&self.sidecar, f)
    }

    /// Converts [`ExecutionData`] to [`Block`] with raw [`Bytes`] transactions.
    ///
    /// This is similar to [`Self::try_into_block_with`] but returns the transactions as raw bytes
    /// without any conversion.
    pub fn into_block_raw(self) -> Result<Block<Bytes>, PayloadError> {
        let mut base_block = self.payload.into_block_raw()?;
        base_block.header.parent_beacon_block_root = self.sidecar.parent_beacon_block_root();
        base_block.header.requests_hash = self.sidecar.requests_hash();
        Ok(base_block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CancunPayloadFields, PayloadValidationError};
    use alloc::vec;
    use alloy_consensus::TxEnvelope;
    use alloy_primitives::{b256, hex};
    use similar_asserts::assert_eq;

    #[test]
    #[cfg(feature = "kzg")]
    fn convert_empty_bundle() {
        let bundle = BlobsBundleV1::default();
        let _sidecar = bundle.try_into_sidecar().unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_blobsbundlev1_empty() {
        let blobs_bundle_v1 = BlobsBundleV1::empty();

        let serialized = serde_json::to_string(&blobs_bundle_v1).unwrap();
        let deserialized: BlobsBundleV1 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, blobs_bundle_v1);
    }

    #[test]
    #[cfg(feature = "serde")]
    #[cfg(not(debug_assertions))]
    fn serde_blobsbundlev1_not_empty_pass() {
        let blobs_bundle_v1 = BlobsBundleV1 {
            proofs: vec![Bytes48::default()],
            commitments: vec![Bytes48::default()],
            blobs: vec![Blob::default()],
        };

        let serialized = serde_json::to_string(&blobs_bundle_v1).unwrap();
        let deserialized: BlobsBundleV1 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, blobs_bundle_v1);
    }

    #[test]
    #[cfg(feature = "serde")]
    #[cfg(not(debug_assertions))]
    fn serde_blobsbundlev1_not_empty_fail() {
        let blobs_bundle_v1 = BlobsBundleV1 {
            proofs: vec![Bytes48::default(), Bytes48::default()],
            commitments: vec![Bytes48::default()],
            blobs: vec![Blob::default()],
        };

        let serialized = serde_json::to_string(&blobs_bundle_v1).unwrap();
        let deserialized: Result<BlobsBundleV1, serde_json::Error> =
            serde_json::from_str(&serialized);
        assert!(deserialized.is_err(), "invalid length 2, expected commitments.len()");
    }

    #[test]
    #[cfg(feature = "serde")]
    #[cfg(not(debug_assertions))]
    fn serde_blobsbundlev2_not_empty_pass() {
        let commitments = vec![Bytes48::default()];

        let blobs_bundle_v2 = BlobsBundleV2 {
            proofs: vec![Bytes48::default(); commitments.len() * CELLS_PER_EXT_BLOB],
            commitments,
            blobs: vec![Blob::default()],
        };

        let serialized = serde_json::to_string(&blobs_bundle_v2).unwrap();
        let deserialized: BlobsBundleV2 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, blobs_bundle_v2);
    }

    #[test]
    #[cfg(feature = "serde")]
    #[cfg(not(debug_assertions))]
    fn serde_blobsbundlev2_not_empty_fail() {
        let blobs_bundle_v2 = BlobsBundleV2 {
            proofs: vec![Bytes48::default()],
            commitments: vec![Bytes48::default()],
            blobs: vec![],
        };

        let serialized = serde_json::to_string(&blobs_bundle_v2).unwrap();
        let deserialized: Result<BlobsBundleV2, serde_json::Error> =
            serde_json::from_str(&serialized);
        assert!(deserialized.is_err());
    }

    #[test]
    #[cfg(feature = "ssz")]
    #[cfg(not(debug_assertions))]
    fn ssz_blobsbundlev2_roundtrip() {
        let commitments = vec![Bytes48::default(), Bytes48::default()];
        let num_blobs = commitments.len();

        let blobs_bundle_v2 = BlobsBundleV2 {
            commitments,
            proofs: vec![Bytes48::default(); num_blobs * CELLS_PER_EXT_BLOB],
            blobs: vec![Blob::default(); num_blobs],
        };

        let encoded = ssz::Encode::as_ssz_bytes(&blobs_bundle_v2);
        let decoded: BlobsBundleV2 = ssz::Decode::from_ssz_bytes(&encoded).unwrap();

        assert_eq!(decoded, blobs_bundle_v2);
    }

    #[test]
    #[cfg(feature = "ssz")]
    #[cfg(not(debug_assertions))]
    fn ssz_blobsbundlev2_invalid_proofs_length() {
        let commitments = vec![Bytes48::default()];

        let blobs_bundle_v2 = BlobsBundleV2 {
            commitments,
            proofs: vec![Bytes48::default(); 2],
            blobs: vec![Blob::default()],
        };

        let encoded = ssz::Encode::as_ssz_bytes(&blobs_bundle_v2);

        // Attempt to decode - should fail due to mismatched proofs length
        let result: Result<BlobsBundleV2, _> = ssz::Decode::from_ssz_bytes(&encoded);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "ssz")]
    #[cfg(not(debug_assertions))]
    fn ssz_blobsbundlev2_mismatched_commitments_blobs() {
        let blobs_bundle_v2 = BlobsBundleV2 {
            commitments: vec![Bytes48::default(), Bytes48::default()],
            proofs: vec![Bytes48::default(); CELLS_PER_EXT_BLOB],
            blobs: vec![Blob::default()],
        };

        let encoded = ssz::Encode::as_ssz_bytes(&blobs_bundle_v2);

        // Attempt to decode - should fail due to wrong number of commitments
        let result: Result<BlobsBundleV2, _> = ssz::Decode::from_ssz_bytes(&encoded);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "ssz")]
    fn ssz_blobsbundlev2_empty() {
        let blobs_bundle_v2 = BlobsBundleV2 { commitments: vec![], proofs: vec![], blobs: vec![] };

        let encoded = ssz::Encode::as_ssz_bytes(&blobs_bundle_v2);

        // Decode from SSZ - empty bundle should be valid
        let decoded: BlobsBundleV2 = ssz::Decode::from_ssz_bytes(&encoded).unwrap();
        assert_eq!(decoded, blobs_bundle_v2);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_payload_status() {
        let s = r#"{"status":"SYNCING","latestValidHash":null,"validationError":null}"#;
        let status: PayloadStatus = serde_json::from_str(s).unwrap();
        assert_eq!(status.status, PayloadStatusEnum::Syncing);
        assert!(status.latest_valid_hash.is_none());
        assert!(status.status.validation_error().is_none());
        assert_eq!(serde_json::to_string(&status).unwrap(), s);

        let full = s;
        let s = r#"{"status":"SYNCING","latestValidHash":null}"#;
        let status: PayloadStatus = serde_json::from_str(s).unwrap();
        assert_eq!(status.status, PayloadStatusEnum::Syncing);
        assert!(status.latest_valid_hash.is_none());
        assert!(status.status.validation_error().is_none());
        assert_eq!(serde_json::to_string(&status).unwrap(), full);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_payload_status_error_deserialize() {
        let s = r#"{"status":"INVALID","latestValidHash":null,"validationError":"Failed to decode block"}"#;
        let q = PayloadStatus {
            latest_valid_hash: None,
            status: PayloadStatusEnum::Invalid {
                validation_error: "Failed to decode block".to_string(),
            },
        };
        assert_eq!(q, serde_json::from_str(s).unwrap());

        let s = r#"{"status":"INVALID","latestValidHash":null,"validationError":"links to previously rejected block"}"#;
        let q = PayloadStatus {
            latest_valid_hash: None,
            status: PayloadStatusEnum::Invalid {
                validation_error: PayloadValidationError::LinksToRejectedPayload.to_string(),
            },
        };
        assert_eq!(q, serde_json::from_str(s).unwrap());

        let s = r#"{"status":"INVALID","latestValidHash":null,"validationError":"invalid block number"}"#;
        let q = PayloadStatus {
            latest_valid_hash: None,
            status: PayloadStatusEnum::Invalid {
                validation_error: PayloadValidationError::InvalidBlockNumber.to_string(),
            },
        };
        assert_eq!(q, serde_json::from_str(s).unwrap());

        let s = r#"{"status":"INVALID","latestValidHash":null,"validationError":
        "invalid merkle root: (remote: 0x3f77fb29ce67436532fee970e1add8f5cc80e8878c79b967af53b1fd92a0cab7 local: 0x603b9628dabdaadb442a3bb3d7e0360efc110e1948472909230909f1690fed17)"}"#;
        let q = PayloadStatus {
            latest_valid_hash: None,
            status: PayloadStatusEnum::Invalid {
                validation_error: PayloadValidationError::InvalidStateRoot {
                    remote: "0x3f77fb29ce67436532fee970e1add8f5cc80e8878c79b967af53b1fd92a0cab7"
                        .parse()
                        .unwrap(),
                    local: "0x603b9628dabdaadb442a3bb3d7e0360efc110e1948472909230909f1690fed17"
                        .parse()
                        .unwrap(),
                }
                .to_string(),
            },
        };
        assert_eq!(q, serde_json::from_str(s).unwrap());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip_legacy_txs_payload_v1() {
        // pulled from hive tests
        let s = r#"{"parentHash":"0x67ead97eb79b47a1638659942384143f36ed44275d4182799875ab5a87324055","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","receiptsRoot":"0x4e3c608a9f2e129fccb91a1dae7472e78013b8e654bccc8d224ce3d63ae17006","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0x44bb4b98c59dbb726f96ffceb5ee028dcbe35b9bba4f9ffd56aeebf8d1e4db62","blockNumber":"0x1","gasLimit":"0x2fefd8","gasUsed":"0xa860","timestamp":"0x1235","extraData":"0x8b726574682f76302e312e30","baseFeePerGas":"0x342770c0","blockHash":"0x5655011482546f16b2312ef18e9fad03d6a52b1be95401aea884b222477f9e64","transactions":["0xf865808506fc23ac00830124f8940000000000000000000000000000000000000316018032a044b25a8b9b247d01586b3d59c71728ff49c9b84928d9e7fa3377ead3b5570b5da03ceac696601ff7ee6f5fe8864e2998db9babdf5eeba1a0cd5b4d44b3fcbd181b"]}"#;
        let payload: ExecutionPayloadV1 = serde_json::from_str(s).unwrap();
        assert_eq!(serde_json::to_string(&payload).unwrap(), s);

        let any_payload: ExecutionPayload = serde_json::from_str(s).unwrap();
        assert_eq!(any_payload, payload.into());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip_legacy_txs_payload_v3() {
        // pulled from hive tests - modified with 4844 fields
        let s = r#"{"parentHash":"0x67ead97eb79b47a1638659942384143f36ed44275d4182799875ab5a87324055","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","receiptsRoot":"0x4e3c608a9f2e129fccb91a1dae7472e78013b8e654bccc8d224ce3d63ae17006","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0x44bb4b98c59dbb726f96ffceb5ee028dcbe35b9bba4f9ffd56aeebf8d1e4db62","blockNumber":"0x1","gasLimit":"0x2fefd8","gasUsed":"0xa860","timestamp":"0x1235","extraData":"0x8b726574682f76302e312e30","baseFeePerGas":"0x342770c0","blockHash":"0x5655011482546f16b2312ef18e9fad03d6a52b1be95401aea884b222477f9e64","transactions":["0xf865808506fc23ac00830124f8940000000000000000000000000000000000000316018032a044b25a8b9b247d01586b3d59c71728ff49c9b84928d9e7fa3377ead3b5570b5da03ceac696601ff7ee6f5fe8864e2998db9babdf5eeba1a0cd5b4d44b3fcbd181b"],"withdrawals":[],"blobGasUsed":"0xb10b","excessBlobGas":"0xb10b"}"#;
        let payload: ExecutionPayloadV3 = serde_json::from_str(s).unwrap();
        assert_eq!(serde_json::to_string(&payload).unwrap(), s);

        let any_payload: ExecutionPayload = serde_json::from_str(s).unwrap();
        assert_eq!(any_payload, payload.into());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip_enveloped_txs_payload_v1() {
        // pulled from hive tests
        let s = r#"{"parentHash":"0x67ead97eb79b47a1638659942384143f36ed44275d4182799875ab5a87324055","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x76a03cbcb7adce07fd284c61e4fa31e5e786175cefac54a29e46ec8efa28ea41","receiptsRoot":"0x4e3c608a9f2e129fccb91a1dae7472e78013b8e654bccc8d224ce3d63ae17006","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0x028111cb7d25918386a69656b3d17b2febe95fd0f11572c1a55c14f99fdfe3df","blockNumber":"0x1","gasLimit":"0x2fefd8","gasUsed":"0xa860","timestamp":"0x1235","extraData":"0x8b726574682f76302e312e30","baseFeePerGas":"0x342770c0","blockHash":"0xa6f40ed042e61e88e76125dede8fff8026751ea14454b68fb534cea99f2b2a77","transactions":["0xf865808506fc23ac00830124f8940000000000000000000000000000000000000316018032a044b25a8b9b247d01586b3d59c71728ff49c9b84928d9e7fa3377ead3b5570b5da03ceac696601ff7ee6f5fe8864e2998db9babdf5eeba1a0cd5b4d44b3fcbd181b"]}"#;
        let payload: ExecutionPayloadV1 = serde_json::from_str(s).unwrap();
        assert_eq!(serde_json::to_string(&payload).unwrap(), s);

        let any_payload: ExecutionPayload = serde_json::from_str(s).unwrap();
        assert_eq!(any_payload, payload.into());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip_enveloped_txs_payload_v3() {
        // pulled from hive tests - modified with 4844 fields
        let s = r#"{"parentHash":"0x67ead97eb79b47a1638659942384143f36ed44275d4182799875ab5a87324055","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x76a03cbcb7adce07fd284c61e4fa31e5e786175cefac54a29e46ec8efa28ea41","receiptsRoot":"0x4e3c608a9f2e129fccb91a1dae7472e78013b8e654bccc8d224ce3d63ae17006","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0x028111cb7d25918386a69656b3d17b2febe95fd0f11572c1a55c14f99fdfe3df","blockNumber":"0x1","gasLimit":"0x2fefd8","gasUsed":"0xa860","timestamp":"0x1235","extraData":"0x8b726574682f76302e312e30","baseFeePerGas":"0x342770c0","blockHash":"0xa6f40ed042e61e88e76125dede8fff8026751ea14454b68fb534cea99f2b2a77","transactions":["0xf865808506fc23ac00830124f8940000000000000000000000000000000000000316018032a044b25a8b9b247d01586b3d59c71728ff49c9b84928d9e7fa3377ead3b5570b5da03ceac696601ff7ee6f5fe8864e2998db9babdf5eeba1a0cd5b4d44b3fcbd181b"],"withdrawals":[],"blobGasUsed":"0xb10b","excessBlobGas":"0xb10b"}"#;
        let payload: ExecutionPayloadV3 = serde_json::from_str(s).unwrap();
        assert_eq!(serde_json::to_string(&payload).unwrap(), s);

        let any_payload: ExecutionPayload = serde_json::from_str(s).unwrap();
        assert_eq!(any_payload, payload.into());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip_execution_payload_envelope_v3() {
        // pulled from a geth response getPayloadV3 in hive tests
        let response = r#"{"executionPayload":{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"withdrawals":[],"blobGasUsed":"0x0","excessBlobGas":"0x0"},"blockValue":"0x0","blobsBundle":{"commitments":[],"proofs":[],"blobs":[]},"shouldOverrideBuilder":false}"#;
        let envelope: ExecutionPayloadEnvelopeV3 = serde_json::from_str(response).unwrap();
        assert_eq!(serde_json::to_string(&envelope).unwrap(), response);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_payload_input_enum_v3() {
        let response_v3 = r#"{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"withdrawals":[],"blobGasUsed":"0x0","excessBlobGas":"0x0"}"#;

        let payload: ExecutionPayload = serde_json::from_str(response_v3).unwrap();
        assert!(payload.as_v3().is_some());
        assert_eq!(serde_json::to_string(&payload).unwrap(), response_v3);

        let payload_v3: ExecutionPayloadV3 = serde_json::from_str(response_v3).unwrap();
        assert_eq!(payload.as_v3().unwrap(), &payload_v3);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_payload_input_enum_v2() {
        let response_v2 = r#"{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"withdrawals":[]}"#;

        let payload: ExecutionPayload = serde_json::from_str(response_v2).unwrap();
        assert!(payload.as_v3().is_none());
        assert!(payload.as_v2().is_some());
        assert_eq!(serde_json::to_string(&payload).unwrap(), response_v2);

        let payload_v2: ExecutionPayloadV2 = serde_json::from_str(response_v2).unwrap();
        assert_eq!(payload.as_v2().unwrap(), &payload_v2);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_payload_input_enum_faulty_v2() {
        // incomplete V3 payload should be rejected even if it has all V2 fields
        let response_faulty = r#"{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"withdrawals":[], "blobGasUsed": "0x0"}"#;

        let payload: Result<ExecutionPayload, serde_json::Error> =
            serde_json::from_str(response_faulty);
        assert!(payload.is_err());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_payload_input_enum_faulty_v1() {
        // incomplete V3 payload should be rejected even if it has all V1 fields
        let response_faulty = r#"{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"blobGasUsed": "0x0"}"#;

        let payload: Result<ExecutionPayload, serde_json::Error> =
            serde_json::from_str(response_faulty);
        assert!(payload.is_err());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_faulty_roundtrip_payload_input_v3() {
        // The deserialization behavior of ExecutionPayload structs is faulty.
        // They should not be implicitly deserializable to an earlier version,
        // as this breaks round-trip behavior
        let response_v3 = r#"{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"withdrawals":[],"blobGasUsed":"0x0","excessBlobGas":"0x0"}"#;

        let payload_v2: ExecutionPayloadV2 = serde_json::from_str(response_v3).unwrap();
        assert_ne!(response_v3, serde_json::to_string(&payload_v2).unwrap());

        let payload_v1: ExecutionPayloadV1 = serde_json::from_str(response_v3).unwrap();
        assert_ne!(response_v3, serde_json::to_string(&payload_v1).unwrap());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_faulty_roundtrip_payload_input_v2() {
        // The deserialization behavior of ExecutionPayload structs is faulty.
        // They should not be implicitly deserializable to an earlier version,
        // as this breaks round-trip behavior
        let response_v2 = r#"{"parentHash":"0xe927a1448525fb5d32cb50ee1408461a945ba6c39bd5cf5621407d500ecc8de9","feeRecipient":"0x0000000000000000000000000000000000000000","stateRoot":"0x10f8a0830000e8edef6d00cc727ff833f064b1950afd591ae41357f97e543119","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xe0d8b4521a7da1582a713244ffb6a86aa1726932087386e2dc7973f43fc6cb24","blockNumber":"0x1","gasLimit":"0x2ffbd2","gasUsed":"0x0","timestamp":"0x1235","extraData":"0xd883010d00846765746888676f312e32312e30856c696e7578","baseFeePerGas":"0x342770c0","blockHash":"0x44d0fa5f2f73a938ebb96a2a21679eb8dea3e7b7dd8fd9f35aa756dda8bf0a8a","transactions":[],"withdrawals":[]}"#;

        let payload: ExecutionPayloadV1 = serde_json::from_str(response_v2).unwrap();
        assert_ne!(response_v2, serde_json::to_string(&payload).unwrap());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_deserialize_execution_payload_input_v2() {
        let response = r#"
{
  "baseFeePerGas": "0x173b30b3",
  "blockHash": "0x99d486755fd046ad0bbb60457bac93d4856aa42fa00629cc7e4a28b65b5f8164",
  "blockNumber": "0xb",
  "extraData": "0xd883010d01846765746888676f312e32302e33856c696e7578",
  "feeRecipient": "0x0000000000000000000000000000000000000000",
  "gasLimit": "0x405829",
  "gasUsed": "0x3f0ca0",
  "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "parentHash": "0xfe34aaa2b869c66a727783ee5ad3e3983b6ef22baf24a1e502add94e7bcac67a",
  "prevRandao": "0x74132c32fe3ab9a470a8352544514d21b6969e7749f97742b53c18a1b22b396c",
  "receiptsRoot": "0x6a5c41dc55a1bd3e74e7f6accc799efb08b00c36c15265058433fcea6323e95f",
  "stateRoot": "0xde3b357f5f099e4c33d0343c9e9d204d663d7bd9c65020a38e5d0b2a9ace78a2",
  "timestamp": "0x6507d6b4",
  "transactions": [
    "0xf86d0a8458b20efd825208946177843db3138ae69679a54b95cf345ed759450d8806f3e8d87878800080820a95a0f8bddb1dcc4558b532ff747760a6f547dd275afdbe7bdecc90680e71de105757a014f34ba38c180913c0543b0ac2eccfb77cc3f801a535008dc50e533fbe435f53",
    "0xf86d0b8458b20efd82520894687704db07e902e9a8b3754031d168d46e3d586e8806f3e8d87878800080820a95a0e3108f710902be662d5c978af16109961ffaf2ac4f88522407d40949a9574276a0205719ed21889b42ab5c1026d40b759a507c12d92db0d100fa69e1ac79137caa",
    "0xf86d0c8458b20efd8252089415e6a5a2e131dd5467fa1ff3acd104f45ee5940b8806f3e8d87878800080820a96a0af556ba9cda1d686239e08c24e169dece7afa7b85e0948eaa8d457c0561277fca029da03d3af0978322e54ac7e8e654da23934e0dd839804cb0430f8aaafd732dc",
    "0xf8521784565adcb7830186a0808080820a96a0ec782872a673a9fe4eff028a5bdb30d6b8b7711f58a187bf55d3aec9757cb18ea001796d373da76f2b0aeda72183cce0ad070a4f03aa3e6fee4c757a9444245206",
    "0xf8521284565adcb7830186a0808080820a95a08a0ea89028eff02596b385a10e0bd6ae098f3b281be2c95a9feb1685065d7384a06239d48a72e4be767bd12f317dd54202f5623a33e71e25a87cb25dd781aa2fc8",
    "0xf8521384565adcb7830186a0808080820a95a0784dbd311a82f822184a46f1677a428cbe3a2b88a798fb8ad1370cdbc06429e8a07a7f6a0efd428e3d822d1de9a050b8a883938b632185c254944dd3e40180eb79"
  ],
  "withdrawals": []
}
        "#;
        let payload: ExecutionPayloadInputV2 = serde_json::from_str(response).unwrap();
        assert_eq!(payload.withdrawals, Some(vec![]));

        let response = r#"
{
  "baseFeePerGas": "0x173b30b3",
  "blockHash": "0x99d486755fd046ad0bbb60457bac93d4856aa42fa00629cc7e4a28b65b5f8164",
  "blockNumber": "0xb",
  "extraData": "0xd883010d01846765746888676f312e32302e33856c696e7578",
  "feeRecipient": "0x0000000000000000000000000000000000000000",
  "gasLimit": "0x405829",
  "gasUsed": "0x3f0ca0",
  "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "parentHash": "0xfe34aaa2b869c66a727783ee5ad3e3983b6ef22baf24a1e502add94e7bcac67a",
  "prevRandao": "0x74132c32fe3ab9a470a8352544514d21b6969e7749f97742b53c18a1b22b396c",
  "receiptsRoot": "0x6a5c41dc55a1bd3e74e7f6accc799efb08b00c36c15265058433fcea6323e95f",
  "stateRoot": "0xde3b357f5f099e4c33d0343c9e9d204d663d7bd9c65020a38e5d0b2a9ace78a2",
  "timestamp": "0x6507d6b4",
  "transactions": [
    "0xf86d0a8458b20efd825208946177843db3138ae69679a54b95cf345ed759450d8806f3e8d87878800080820a95a0f8bddb1dcc4558b532ff747760a6f547dd275afdbe7bdecc90680e71de105757a014f34ba38c180913c0543b0ac2eccfb77cc3f801a535008dc50e533fbe435f53",
    "0xf86d0b8458b20efd82520894687704db07e902e9a8b3754031d168d46e3d586e8806f3e8d87878800080820a95a0e3108f710902be662d5c978af16109961ffaf2ac4f88522407d40949a9574276a0205719ed21889b42ab5c1026d40b759a507c12d92db0d100fa69e1ac79137caa",
    "0xf86d0c8458b20efd8252089415e6a5a2e131dd5467fa1ff3acd104f45ee5940b8806f3e8d87878800080820a96a0af556ba9cda1d686239e08c24e169dece7afa7b85e0948eaa8d457c0561277fca029da03d3af0978322e54ac7e8e654da23934e0dd839804cb0430f8aaafd732dc",
    "0xf8521784565adcb7830186a0808080820a96a0ec782872a673a9fe4eff028a5bdb30d6b8b7711f58a187bf55d3aec9757cb18ea001796d373da76f2b0aeda72183cce0ad070a4f03aa3e6fee4c757a9444245206",
    "0xf8521284565adcb7830186a0808080820a95a08a0ea89028eff02596b385a10e0bd6ae098f3b281be2c95a9feb1685065d7384a06239d48a72e4be767bd12f317dd54202f5623a33e71e25a87cb25dd781aa2fc8",
    "0xf8521384565adcb7830186a0808080820a95a0784dbd311a82f822184a46f1677a428cbe3a2b88a798fb8ad1370cdbc06429e8a07a7f6a0efd428e3d822d1de9a050b8a883938b632185c254944dd3e40180eb79"
  ]
}
        "#;
        let payload: ExecutionPayloadInputV2 = serde_json::from_str(response).unwrap();
        assert_eq!(payload.withdrawals, None);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_deserialize_v2_input_with_blob_fields() {
        let input = r#"
{
    "parentHash": "0xaaa4c5b574f37e1537c78931d1bca24a4d17d4f29f1ee97e1cd48b704909de1f",
    "feeRecipient": "0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba",
    "stateRoot": "0x308ee9c5c6fab5e3d08763a3b5fe0be8ada891fa5010a49a3390e018dd436810",
    "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "blockNumber": "0xf",
    "gasLimit": "0x16345785d8a0000",
    "gasUsed": "0x0",
    "timestamp": "0x3a97",
    "extraData": "0x",
    "baseFeePerGas": "0x7",
    "blockHash": "0x38bb6ba645c7e6bd970f9c7d492fafe1e04d85349054cb48d16c9d2c3e3cd0bf",
    "transactions": [],
    "withdrawals": [],
    "excessBlobGas": "0x0",
    "blobGasUsed": "0x0"
}
        "#;

        // ensure that deserializing this (it includes blob fields) fails
        let payload_res: Result<ExecutionPayloadInputV2, serde_json::Error> =
            serde_json::from_str(input);
        assert!(payload_res.is_err());
    }

    // <https://github.com/paradigmxyz/reth/issues/6036>
    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_op_base_payload() {
        let payload = r#"{"parentHash":"0x24e8df372a61cdcdb1a163b52aaa1785e0c869d28c3b742ac09e826bbb524723","feeRecipient":"0x4200000000000000000000000000000000000011","stateRoot":"0x9a5db45897f1ff1e620a6c14b0a6f1b3bcdbed59f2adc516a34c9a9d6baafa71","receiptsRoot":"0x8af6f74835d47835deb5628ca941d00e0c9fd75585f26dabdcb280ec7122e6af","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0xf37b24eeff594848072a05f74c8600001706c83e489a9132e55bf43a236e42ec","blockNumber":"0xe3d5d8","gasLimit":"0x17d7840","gasUsed":"0xb705","timestamp":"0x65a118c0","extraData":"0x","baseFeePerGas":"0x7a0ff32","blockHash":"0xf5c147b2d60a519b72434f0a8e082e18599021294dd9085d7597b0ffa638f1c0","withdrawals":[],"transactions":["0x7ef90159a05ba0034ffdcb246703298224564720b66964a6a69d0d7e9ffd970c546f7c048094deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b90104015d8eb900000000000000000000000000000000000000000000000000000000009e1c4a0000000000000000000000000000000000000000000000000000000065a11748000000000000000000000000000000000000000000000000000000000000000a4b479e5fa8d52dd20a8a66e468b56e993bdbffcccf729223aabff06299ab36db000000000000000000000000000000000000000000000000000000000000000400000000000000000000000073b4168cc87f35cc239200a20eb841cded23493b000000000000000000000000000000000000000000000000000000000000083400000000000000000000000000000000000000000000000000000000000f4240"]}"#;
        let _payload = serde_json::from_str::<ExecutionPayloadInputV2>(payload).unwrap();
    }

    #[test]
    fn roundtrip_payload_to_block() {
        let first_transaction_raw = Bytes::from_static(&hex!("02f9017a8501a1f0ff438211cc85012a05f2008512a05f2000830249f094d5409474fd5a725eab2ac9a8b26ca6fb51af37ef80b901040cc7326300000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000001bdd2ed4b616c800000000000000000000000000001e9ee781dd4b97bdef92e5d1785f73a1f931daa20000000000000000000000007a40026a3b9a41754a95eec8c92c6b99886f440c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000009ae80eb647dd09968488fa1d7e412bf8558a0b7a0000000000000000000000000f9815537d361cb02befd9918c95c97d4d8a4a2bc001a0ba8f1928bb0efc3fcd01524a2039a9a2588fa567cd9a7cc18217e05c615e9d69a0544bfd11425ac7748e76b3795b57a5563e2b0eff47b5428744c62ff19ccfc305")[..]);
        let second_transaction_raw = Bytes::from_static(&hex!("03f901388501a1f0ff430c843b9aca00843b9aca0082520894e7249813d8ccf6fa95a2203f46a64166073d58878080c005f8c6a00195f6dff17753fc89b60eac6477026a805116962c9e412de8015c0484e661c1a001aae314061d4f5bbf158f15d9417a238f9589783f58762cd39d05966b3ba2fba0013f5be9b12e7da06f0dd11a7bdc4e0db8ef33832acc23b183bd0a2c1408a757a0019d9ac55ea1a615d92965e04d960cb3be7bff121a381424f1f22865bd582e09a001def04412e76df26fefe7b0ed5e10580918ae4f355b074c0cfe5d0259157869a0011c11a415db57e43db07aef0de9280b591d65ca0cce36c7002507f8191e5d4a80a0c89b59970b119187d97ad70539f1624bbede92648e2dc007890f9658a88756c5a06fb2e3d4ce2c438c0856c2de34948b7032b1aadc4642a9666228ea8cdc7786b7")[..]);

        let new_payload = ExecutionPayloadV3 {
            payload_inner: ExecutionPayloadV2 {
                payload_inner: ExecutionPayloadV1 {
                    base_fee_per_gas:  U256::from(7u64),
                    block_number: 0xa946u64,
                    block_hash: hex!("a5ddd3f286f429458a39cafc13ffe89295a7efa8eb363cf89a1a4887dbcf272b").into(),
                    logs_bloom: hex!("00200004000000000000000080000000000200000000000000000000000000000000200000000000000000000000000000000000800000000200000000000000000000000000000000000008000000200000000000000000000001000000000000000000000000000000800000000000000000000100000000000030000000000000000040000000000000000000000000000000000800080080404000000000000008000000000008200000000000200000000000000000000000000000000000000002000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000100000000000000000000").into(),
                    extra_data: hex!("d883010d03846765746888676f312e32312e31856c696e7578").into(),
                    gas_limit: 0x1c9c380,
                    gas_used: 0x1f4a9,
                    timestamp: 0x651f35b8,
                    fee_recipient: hex!("f97e180c050e5ab072211ad2c213eb5aee4df134").into(),
                    parent_hash: hex!("d829192799c73ef28a7332313b3c03af1f2d5da2c36f8ecfafe7a83a3bfb8d1e").into(),
                    prev_randao: hex!("753888cc4adfbeb9e24e01c84233f9d204f4a9e1273f0e29b43c4c148b2b8b7e").into(),
                    receipts_root: hex!("4cbc48e87389399a0ea0b382b1c46962c4b8e398014bf0cc610f9c672bee3155").into(),
                    state_root: hex!("017d7fa2b5adb480f5e05b2c95cb4186e12062eed893fc8822798eed134329d1").into(),
                    transactions: vec![first_transaction_raw, second_transaction_raw],
                },
                withdrawals: vec![],
            },
            blob_gas_used: 0xc0000,
            excess_blob_gas: 0x580000,
        };

        let mut block: Block<TxEnvelope> = new_payload.clone().try_into_block().unwrap();

        // this newPayload came with a parent beacon block root, we need to manually insert it
        // before hashing
        let parent_beacon_block_root =
            b256!("531cd53b8e68deef0ea65edfa3cda927a846c307b0907657af34bc3f313b5871");
        block.header.parent_beacon_block_root = Some(parent_beacon_block_root);

        let converted_payload = ExecutionPayloadV3::from_block_unchecked(block.hash_slow(), &block);

        // ensure the payloads are the same
        assert_eq!(new_payload, converted_payload);
    }

    #[test]
    fn payload_to_block_rejects_network_encoded_tx() {
        let first_transaction_raw = Bytes::from_static(&hex!("b9017e02f9017a8501a1f0ff438211cc85012a05f2008512a05f2000830249f094d5409474fd5a725eab2ac9a8b26ca6fb51af37ef80b901040cc7326300000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000001bdd2ed4b616c800000000000000000000000000001e9ee781dd4b97bdef92e5d1785f73a1f931daa20000000000000000000000007a40026a3b9a41754a95eec8c92c6b99886f440c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000009ae80eb647dd09968488fa1d7e412bf8558a0b7a0000000000000000000000000f9815537d361cb02befd9918c95c97d4d8a4a2bc001a0ba8f1928bb0efc3fcd01524a2039a9a2588fa567cd9a7cc18217e05c615e9d69a0544bfd11425ac7748e76b3795b57a5563e2b0eff47b5428744c62ff19ccfc305")[..]);
        let second_transaction_raw = Bytes::from_static(&hex!("b9013c03f901388501a1f0ff430c843b9aca00843b9aca0082520894e7249813d8ccf6fa95a2203f46a64166073d58878080c005f8c6a00195f6dff17753fc89b60eac6477026a805116962c9e412de8015c0484e661c1a001aae314061d4f5bbf158f15d9417a238f9589783f58762cd39d05966b3ba2fba0013f5be9b12e7da06f0dd11a7bdc4e0db8ef33832acc23b183bd0a2c1408a757a0019d9ac55ea1a615d92965e04d960cb3be7bff121a381424f1f22865bd582e09a001def04412e76df26fefe7b0ed5e10580918ae4f355b074c0cfe5d0259157869a0011c11a415db57e43db07aef0de9280b591d65ca0cce36c7002507f8191e5d4a80a0c89b59970b119187d97ad70539f1624bbede92648e2dc007890f9658a88756c5a06fb2e3d4ce2c438c0856c2de34948b7032b1aadc4642a9666228ea8cdc7786b7")[..]);

        let new_payload = ExecutionPayloadV3 {
            payload_inner: ExecutionPayloadV2 {
                payload_inner: ExecutionPayloadV1 {
                    base_fee_per_gas:  U256::from(7u64),
                    block_number: 0xa946u64,
                    block_hash: hex!("a5ddd3f286f429458a39cafc13ffe89295a7efa8eb363cf89a1a4887dbcf272b").into(),
                    logs_bloom: hex!("00200004000000000000000080000000000200000000000000000000000000000000200000000000000000000000000000000000800000000200000000000000000000000000000000000008000000200000000000000000000001000000000000000000000000000000800000000000000000000100000000000030000000000000000040000000000000000000000000000000000800080080404000000000000008000000000008200000000000200000000000000000000000000000000000000002000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000100000000000000000000").into(),
                    extra_data: hex!("d883010d03846765746888676f312e32312e31856c696e7578").into(),
                    gas_limit: 0x1c9c380,
                    gas_used: 0x1f4a9,
                    timestamp: 0x651f35b8,
                    fee_recipient: hex!("f97e180c050e5ab072211ad2c213eb5aee4df134").into(),
                    parent_hash: hex!("d829192799c73ef28a7332313b3c03af1f2d5da2c36f8ecfafe7a83a3bfb8d1e").into(),
                    prev_randao: hex!("753888cc4adfbeb9e24e01c84233f9d204f4a9e1273f0e29b43c4c148b2b8b7e").into(),
                    receipts_root: hex!("4cbc48e87389399a0ea0b382b1c46962c4b8e398014bf0cc610f9c672bee3155").into(),
                    state_root: hex!("017d7fa2b5adb480f5e05b2c95cb4186e12062eed893fc8822798eed134329d1").into(),
                    transactions: vec![first_transaction_raw, second_transaction_raw],
                },
                withdrawals: vec![],
            },
            blob_gas_used: 0xc0000,
            excess_blob_gas: 0x580000,
        };

        let _block = new_payload
            .try_into_block::<TxEnvelope>()
            .expect_err("execution payload conversion requires typed txs without a rlp header");
    }

    #[test]
    fn devnet_invalid_block_hash_repro() {
        let deser_block = r#"
        {
            "parentHash": "0xae8315ee86002e6269a17dd1e9516a6cf13223e9d4544d0c32daff826fb31acc",
            "feeRecipient": "0xf97e180c050e5ab072211ad2c213eb5aee4df134",
            "stateRoot": "0x03787f1579efbaa4a8234e72465eb4e29ef7e62f61242d6454661932e1a282a1",
            "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
            "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "prevRandao": "0x918e86b497dc15de7d606457c36ca583e24d9b0a110a814de46e33d5bb824a66",
            "blockNumber": "0x6a784",
            "gasLimit": "0x1c9c380",
            "gasUsed": "0x0",
            "timestamp": "0x65bc1d60",
            "extraData": "0x9a726574682f76302e312e302d616c7068612e31362f6c696e7578",
            "baseFeePerGas": "0x8",
            "blobGasUsed": "0x0",
            "excessBlobGas": "0x0",
            "blockHash": "0x340c157eca9fd206b87c17f0ecbe8d411219de7188a0a240b635c88a96fe91c5",
            "transactions": [],
            "withdrawals": [
                {
                    "index": "0x5ab202",
                    "validatorIndex": "0xb1b",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab203",
                    "validatorIndex": "0xb1c",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x15892"
                },
                {
                    "index": "0x5ab204",
                    "validatorIndex": "0xb1d",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab205",
                    "validatorIndex": "0xb1e",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab206",
                    "validatorIndex": "0xb1f",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab207",
                    "validatorIndex": "0xb20",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab208",
                    "validatorIndex": "0xb21",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x15892"
                },
                {
                    "index": "0x5ab209",
                    "validatorIndex": "0xb22",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab20a",
                    "validatorIndex": "0xb23",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab20b",
                    "validatorIndex": "0xb24",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x17db2"
                },
                {
                    "index": "0x5ab20c",
                    "validatorIndex": "0xb25",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab20d",
                    "validatorIndex": "0xb26",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                },
                {
                    "index": "0x5ab20e",
                    "validatorIndex": "0xa91",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x15892"
                },
                {
                    "index": "0x5ab20f",
                    "validatorIndex": "0xa92",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x1c05d"
                },
                {
                    "index": "0x5ab210",
                    "validatorIndex": "0xa93",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x15892"
                },
                {
                    "index": "0x5ab211",
                    "validatorIndex": "0xa94",
                    "address": "0x388ea662ef2c223ec0b047d41bf3c0f362142ad5",
                    "amount": "0x19b3d"
                }
            ]
        }
        "#;

        // deserialize payload
        let payload: ExecutionPayload =
            serde_json::from_str::<ExecutionPayloadV3>(deser_block).unwrap().into();

        // NOTE: the actual block hash here is incorrect, it is a result of a bug, this was the
        // fix:
        // <https://github.com/paradigmxyz/reth/pull/6328>
        let block_hash_with_blob_fee_fields =
            b256!("a7cdd5f9e54147b53a15833a8c45dffccbaed534d7fdc23458f45102a4bf71b0");

        let versioned_hashes = vec![];
        let parent_beacon_block_root =
            b256!("1162de8a0f4d20d86b9ad6e0a2575ab60f00a433dc70d9318c8abc9041fddf54");

        // set up cancun payload fields
        let cancun_fields = CancunPayloadFields { parent_beacon_block_root, versioned_hashes };

        // convert into block
        let block = payload
            .try_into_block_with_sidecar::<TxEnvelope>(&ExecutionPayloadSidecar::v3(cancun_fields))
            .unwrap();

        // Ensure the actual hash is calculated if we set the fields to what they should be
        assert_eq!(block_hash_with_blob_fee_fields, block.header.hash_slow());
    }

    #[test]
    fn test_payload_to_block_with_sidecar_raw() {
        use std::path::PathBuf;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/payload");
        let dir = std::fs::read_dir(path).expect("Unable to read payload folder");

        for entry in dir {
            let entry = entry.expect("Unable to read entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let contents = std::fs::read_to_string(&path).expect("Unable to read file");
            let value: serde_json::Value = serde_json::from_str(&contents)
                .unwrap_or_else(|e| panic!("Failed to parse JSON from {path:?}: {e}"));

            // Extract the newPayload object
            let new_payload = &value["newPayload"];
            let payload_value = &new_payload["payload"];
            let sidecar_value = &new_payload["sidecar"];

            let payload: ExecutionPayload = serde_json::from_value(payload_value.clone())
                .unwrap_or_else(|e| panic!("Failed to deserialize payload from {path:?}: {e}"));

            // Deserialize the sidecar
            let sidecar: ExecutionPayloadSidecar = serde_json::from_value(sidecar_value.clone())
                .unwrap_or_else(|e| panic!("Failed to deserialize sidecar from {path:?}: {e}"));

            // Convert to block with raw transactions
            let block = payload.clone().into_block_with_sidecar_raw(&sidecar).unwrap_or_else(|e| {
                panic!("Failed to convert payload to block from {path:?}: {e}")
            });

            // Verify the block has raw transactions (Bytes) if there are any
            if let Some(tx_count) = payload_value["transactions"].as_array().map(|a| a.len()) {
                assert_eq!(
                    block.body.transactions.len(),
                    tx_count,
                    "Transaction count mismatch in {:?}",
                    path
                );
            }

            // Verify sidecar fields are applied
            assert_eq!(
                block.header.parent_beacon_block_root,
                sidecar.parent_beacon_block_root(),
                "Parent beacon block root mismatch in {:?}",
                path
            );
            assert_eq!(
                block.header.requests_hash,
                sidecar.requests_hash(),
                "Requests hash mismatch in {:?}",
                path
            );

            // Verify the block hash matches the one in the payload
            let expected_hash = payload_value["blockHash"]
                .as_str()
                .unwrap()
                .parse::<B256>()
                .unwrap_or_else(|e| panic!("Failed to parse block hash from {path:?}: {e}"));
            let actual_hash = block.header.hash_slow();
            assert_eq!(
                actual_hash, expected_hash,
                "Block hash mismatch in {:?}: expected {}, got {}",
                path, expected_hash, actual_hash
            );

            let block =
                payload.try_into_block_with_sidecar::<TxEnvelope>(&sidecar).unwrap_or_else(|e| {
                    panic!("Failed to convert payload to block from {path:?}: {e}")
                });
            let actual_hash = block.header.hash_slow();
            assert_eq!(
                actual_hash, expected_hash,
                "Block hash mismatch in {:?}: expected {}, got {}",
                path, expected_hash, actual_hash
            );
        }
    }
}
