//! Payload support for the beacon API.
//!
//! Internal helper module to deserialize/serialize the payload attributes for the beacon API, which
//! uses snake case and quoted decimals.
//!
//! This is necessary because we don't want to allow a mixture of both formats, hence `serde`
//! aliases are not an option.
//!
//! See also <https://github.com/ethereum/consensus-specs/blob/master/specs/deneb/beacon-chain.md#executionpayload>

use crate::{withdrawals::BeaconWithdrawal, BlsPublicKey};
use alloy_eips::eip4895::Withdrawal;
use alloy_primitives::{Address, Bloom, Bytes, B256, U256};
use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3,
    ExecutionPayloadV4,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, DeserializeAs, DisplayFromStr, SerializeAs};
use std::borrow::Cow;

/// Response object of GET `/eth/v1/builder/header/{slot}/{parent_hash}/{pubkey}`
///
/// See also <https://ethereum.github.io/builder-specs/#/Builder/getHeader>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetExecutionPayloadHeaderResponse {
    /// The version of the response.
    pub version: String,
    /// The data associated with the execution payload header.
    pub data: ExecutionPayloadHeaderData,
}

/// Data structure representing the header data of an execution payload.
///
/// This structure is used to hold the core elements of an execution payload header,
/// including the message and signature components.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPayloadHeaderData {
    /// The message of the execution payload header.
    pub message: ExecutionPayloadHeaderMessage,
    /// The signature of the execution payload header.
    pub signature: Bytes,
}

/// Message structure within the header of an execution payload.
///
/// This structure contains detailed information about the execution payload,
/// including the header, value, and public key associated with the payload.
#[serde_as]
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPayloadHeaderMessage {
    /// The header of the execution payload.
    pub header: ExecutionPayloadHeader,
    /// The value of the execution payload, represented as a `U256`.
    #[serde_as(as = "DisplayFromStr")]
    pub value: U256,
    /// The public key associated with the execution payload.
    pub pubkey: BlsPublicKey,
}

/// Data structure representing the signed blinded block submitted to the builder, binding the
/// proposer to the block. with its signature.
///
/// See <https://ethereum.github.io/builder-specs/#/Builder/submitBlindedBlockV2>.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockData {
    /// The message of the signed beacon block
    pub message: BeaconBlockMessage,
    /// The signature of the beacon block
    pub signature: Bytes,
}

/// Block Body Message
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockMessage {
    /// Slot number
    pub slot: String,
    /// Proposer index  
    pub proposer_index: String,
    /// Parent root
    pub parent_root: String,
    /// State root
    pub state_root: String,
    /// Block body
    pub body: BeaconBlockBody,
}

/// Execution payload body
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockBody {
    /// Execution payload
    #[serde(
        serialize_with = "beacon_payload::serialize",
        deserialize_with = "beacon_payload::deserialize"
    )]
    pub execution_payload: ExecutionPayload,
}

impl BeaconBlockData {
    /// Get the execution payload
    pub const fn execution_payload(&self) -> &ExecutionPayload {
        &self.message.body.execution_payload
    }
}

/// The header of the execution payload.
#[serde_as]
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPayloadHeader {
    /// The parent hash of the execution payload.
    pub parent_hash: B256,
    /// The fee recipient address of the execution payload.
    pub fee_recipient: Address,
    /// The state root of the execution payload.
    pub state_root: B256,
    /// The receipts root of the execution payload.
    pub receipts_root: B256,
    /// The logs bloom filter of the execution payload.
    pub logs_bloom: Bloom,
    /// The previous Randao value of the execution payload.
    pub prev_randao: B256,
    /// The block number of the execution payload, represented as a `u64`.
    #[serde_as(as = "DisplayFromStr")]
    pub block_number: u64,
    /// The gas limit of the execution payload, represented as a `u64`.
    #[serde_as(as = "DisplayFromStr")]
    pub gas_limit: u64,
    /// The gas used by the execution payload, represented as a `u64`.
    #[serde_as(as = "DisplayFromStr")]
    pub gas_used: u64,
    /// The timestamp of the execution payload, represented as a `u64`.
    #[serde_as(as = "DisplayFromStr")]
    pub timestamp: u64,
    /// The extra data of the execution payload.
    pub extra_data: Bytes,
    /// The base fee per gas of the execution payload, represented as a `U256`.
    #[serde_as(as = "DisplayFromStr")]
    pub base_fee_per_gas: U256,
    /// The block hash of the execution payload.
    pub block_hash: B256,
    /// The transactions root of the execution payload.
    pub transactions_root: B256,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct BeaconPayloadAttributes {
    #[serde_as(as = "DisplayFromStr")]
    timestamp: u64,
    prev_randao: B256,
    suggested_fee_recipient: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Vec<BeaconWithdrawal>>")]
    withdrawals: Option<Vec<Withdrawal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_beacon_block_root: Option<B256>,
}

/// A helper module for serializing and deserializing the payload attributes for the beacon API.
///
/// The beacon API encoded object has equivalent fields to the
/// [PayloadAttributes](alloy_rpc_types_engine::PayloadAttributes) with two differences:
/// 1) `snake_case` identifiers must be used rather than `camelCase`;
/// 2) integers must be encoded as quoted decimals rather than big-endian hex.
pub mod beacon_api_payload_attributes {
    use super::*;
    use alloy_rpc_types_engine::PayloadAttributes;

    /// Serialize the payload attributes for the beacon API.
    pub fn serialize<S>(
        payload_attributes: &PayloadAttributes,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let beacon_api_payload_attributes = BeaconPayloadAttributes {
            timestamp: payload_attributes.timestamp,
            prev_randao: payload_attributes.prev_randao,
            suggested_fee_recipient: payload_attributes.suggested_fee_recipient,
            withdrawals: payload_attributes.withdrawals.clone(),
            parent_beacon_block_root: payload_attributes.parent_beacon_block_root,
        };
        beacon_api_payload_attributes.serialize(serializer)
    }

    /// Deserialize the payload attributes for the beacon API.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<PayloadAttributes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let beacon_api_payload_attributes = BeaconPayloadAttributes::deserialize(deserializer)?;
        Ok(PayloadAttributes {
            timestamp: beacon_api_payload_attributes.timestamp,
            prev_randao: beacon_api_payload_attributes.prev_randao,
            suggested_fee_recipient: beacon_api_payload_attributes.suggested_fee_recipient,
            withdrawals: beacon_api_payload_attributes.withdrawals,
            parent_beacon_block_root: beacon_api_payload_attributes.parent_beacon_block_root,
        })
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct BeaconExecutionPayloadV1<'a> {
    parent_hash: Cow<'a, B256>,
    fee_recipient: Cow<'a, Address>,
    state_root: Cow<'a, B256>,
    receipts_root: Cow<'a, B256>,
    logs_bloom: Cow<'a, Bloom>,
    prev_randao: Cow<'a, B256>,
    #[serde_as(as = "DisplayFromStr")]
    block_number: u64,
    #[serde_as(as = "DisplayFromStr")]
    gas_limit: u64,
    #[serde_as(as = "DisplayFromStr")]
    gas_used: u64,
    #[serde_as(as = "DisplayFromStr")]
    timestamp: u64,
    extra_data: Cow<'a, Bytes>,
    #[serde_as(as = "DisplayFromStr")]
    base_fee_per_gas: U256,
    block_hash: Cow<'a, B256>,
    transactions: Cow<'a, [Bytes]>,
}

impl<'a> From<BeaconExecutionPayloadV1<'a>> for ExecutionPayloadV1 {
    fn from(payload: BeaconExecutionPayloadV1<'a>) -> Self {
        let BeaconExecutionPayloadV1 {
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
        } = payload;
        Self {
            parent_hash: parent_hash.into_owned(),
            fee_recipient: fee_recipient.into_owned(),
            state_root: state_root.into_owned(),
            receipts_root: receipts_root.into_owned(),
            logs_bloom: logs_bloom.into_owned(),
            prev_randao: prev_randao.into_owned(),
            block_number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data: extra_data.into_owned(),
            base_fee_per_gas,
            block_hash: block_hash.into_owned(),
            transactions: transactions.into_owned(),
        }
    }
}

impl<'a> From<&'a ExecutionPayloadV1> for BeaconExecutionPayloadV1<'a> {
    fn from(value: &'a ExecutionPayloadV1) -> Self {
        let ExecutionPayloadV1 {
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
        } = value;

        BeaconExecutionPayloadV1 {
            parent_hash: Cow::Borrowed(parent_hash),
            fee_recipient: Cow::Borrowed(fee_recipient),
            state_root: Cow::Borrowed(state_root),
            receipts_root: Cow::Borrowed(receipts_root),
            logs_bloom: Cow::Borrowed(logs_bloom),
            prev_randao: Cow::Borrowed(prev_randao),
            block_number: *block_number,
            gas_limit: *gas_limit,
            gas_used: *gas_used,
            timestamp: *timestamp,
            extra_data: Cow::Borrowed(extra_data),
            base_fee_per_gas: *base_fee_per_gas,
            block_hash: Cow::Borrowed(block_hash),
            transactions: Cow::Borrowed(transactions),
        }
    }
}

/// A helper serde module to convert from/to the Beacon API which uses quoted decimals rather than
/// big-endian hex.
pub mod beacon_payload_v1 {
    use super::*;

    /// Serialize the payload attributes for the beacon API.
    pub fn serialize<S>(
        payload_attributes: &ExecutionPayloadV1,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BeaconExecutionPayloadV1::from(payload_attributes).serialize(serializer)
    }

    /// Deserialize the payload attributes for the beacon API.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExecutionPayloadV1, D::Error>
    where
        D: Deserializer<'de>,
    {
        BeaconExecutionPayloadV1::deserialize(deserializer).map(Into::into)
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct BeaconExecutionPayloadV2<'a> {
    /// Inner V1 payload
    #[serde(flatten)]
    payload_inner: BeaconExecutionPayloadV1<'a>,
    /// Array of [`Withdrawal`] enabled with V2
    /// See <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/shanghai.md#executionpayloadv2>
    #[serde_as(as = "Vec<BeaconWithdrawal>")]
    withdrawals: Vec<Withdrawal>,
}

impl<'a> From<BeaconExecutionPayloadV2<'a>> for ExecutionPayloadV2 {
    fn from(payload: BeaconExecutionPayloadV2<'a>) -> Self {
        let BeaconExecutionPayloadV2 { payload_inner, withdrawals } = payload;
        Self { payload_inner: payload_inner.into(), withdrawals }
    }
}

impl<'a> From<&'a ExecutionPayloadV2> for BeaconExecutionPayloadV2<'a> {
    fn from(value: &'a ExecutionPayloadV2) -> Self {
        let ExecutionPayloadV2 { payload_inner, withdrawals } = value;
        BeaconExecutionPayloadV2 {
            payload_inner: payload_inner.into(),
            withdrawals: withdrawals.clone(),
        }
    }
}

/// A helper serde module to convert from/to the Beacon API which uses quoted decimals rather than
/// big-endian hex.
pub mod beacon_payload_v2 {
    use super::*;

    /// Serialize the payload attributes for the beacon API.
    pub fn serialize<S>(
        payload_attributes: &ExecutionPayloadV2,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BeaconExecutionPayloadV2::from(payload_attributes).serialize(serializer)
    }

    /// Deserialize the payload attributes for the beacon API.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExecutionPayloadV2, D::Error>
    where
        D: Deserializer<'de>,
    {
        BeaconExecutionPayloadV2::deserialize(deserializer).map(Into::into)
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct BeaconExecutionPayloadV3<'a> {
    /// Inner V2 payload
    #[serde(flatten)]
    payload_inner: BeaconExecutionPayloadV2<'a>,
    #[serde_as(as = "DisplayFromStr")]
    blob_gas_used: u64,
    #[serde_as(as = "DisplayFromStr")]
    excess_blob_gas: u64,
}

impl<'a> From<BeaconExecutionPayloadV3<'a>> for ExecutionPayloadV3 {
    fn from(payload: BeaconExecutionPayloadV3<'a>) -> Self {
        let BeaconExecutionPayloadV3 { payload_inner, blob_gas_used, excess_blob_gas } = payload;
        Self { payload_inner: payload_inner.into(), blob_gas_used, excess_blob_gas }
    }
}

impl<'a> From<&'a ExecutionPayloadV3> for BeaconExecutionPayloadV3<'a> {
    fn from(value: &'a ExecutionPayloadV3) -> Self {
        let ExecutionPayloadV3 { payload_inner, blob_gas_used, excess_blob_gas } = value;
        BeaconExecutionPayloadV3 {
            payload_inner: payload_inner.into(),
            blob_gas_used: *blob_gas_used,
            excess_blob_gas: *excess_blob_gas,
        }
    }
}

/// A helper serde module to convert from/to the Beacon API which uses quoted decimals rather than
/// big-endian hex.
pub mod beacon_payload_v3 {
    use super::*;

    /// Serialize the payload attributes for the beacon API.
    pub fn serialize<S>(
        payload_attributes: &ExecutionPayloadV3,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BeaconExecutionPayloadV3::from(payload_attributes).serialize(serializer)
    }

    /// Deserialize the payload attributes for the beacon API.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExecutionPayloadV3, D::Error>
    where
        D: Deserializer<'de>,
    {
        BeaconExecutionPayloadV3::deserialize(deserializer).map(Into::into)
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct BeaconExecutionPayloadV4<'a> {
    /// Inner V3 payload
    #[serde(flatten)]
    payload_inner: BeaconExecutionPayloadV3<'a>,
    /// RLP encoded `block_access_list`
    block_access_list: Cow<'a, Bytes>,
}

impl<'a> From<BeaconExecutionPayloadV4<'a>> for ExecutionPayloadV4 {
    fn from(payload: BeaconExecutionPayloadV4<'a>) -> Self {
        let BeaconExecutionPayloadV4 { payload_inner, block_access_list } = payload;
        Self {
            payload_inner: payload_inner.into(),
            block_access_list: block_access_list.into_owned(),
        }
    }
}

impl<'a> From<&'a ExecutionPayloadV4> for BeaconExecutionPayloadV4<'a> {
    fn from(value: &'a ExecutionPayloadV4) -> Self {
        let ExecutionPayloadV4 { payload_inner, block_access_list } = value;
        BeaconExecutionPayloadV4 {
            payload_inner: payload_inner.into(),
            block_access_list: Cow::Borrowed(block_access_list),
        }
    }
}

/// A helper serde module to convert from/to the Beacon API which uses quoted decimals rather than
/// big-endian hex.
pub mod beacon_payload_v4 {
    use super::*;

    /// Serialize the payload attributes for the beacon API.
    pub fn serialize<S>(
        payload_attributes: &ExecutionPayloadV4,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BeaconExecutionPayloadV4::from(payload_attributes).serialize(serializer)
    }

    /// Deserialize the payload attributes for the beacon API.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExecutionPayloadV4, D::Error>
    where
        D: Deserializer<'de>,
    {
        BeaconExecutionPayloadV4::deserialize(deserializer).map(Into::into)
    }
}

/// Represents all possible payload versions.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum BeaconExecutionPayload<'a> {
    /// V1 payload
    V1(BeaconExecutionPayloadV1<'a>),
    /// V2 payload
    V2(BeaconExecutionPayloadV2<'a>),
    /// V3 payload
    V3(BeaconExecutionPayloadV3<'a>),
    /// V4 payload
    V4(BeaconExecutionPayloadV4<'a>),
}

// Deserializes untagged ExecutionPayload by trying each variant in falling order
impl<'de> Deserialize<'de> for BeaconExecutionPayload<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum BeaconExecutionPayloadDesc<'a> {
            V4(BeaconExecutionPayloadV4<'a>),
            V3(BeaconExecutionPayloadV3<'a>),
            V2(BeaconExecutionPayloadV2<'a>),
            V1(BeaconExecutionPayloadV1<'a>),
        }
        match BeaconExecutionPayloadDesc::deserialize(deserializer)? {
            BeaconExecutionPayloadDesc::V4(payload) => Ok(Self::V4(payload)),
            BeaconExecutionPayloadDesc::V3(payload) => Ok(Self::V3(payload)),
            BeaconExecutionPayloadDesc::V2(payload) => Ok(Self::V2(payload)),
            BeaconExecutionPayloadDesc::V1(payload) => Ok(Self::V1(payload)),
        }
    }
}

impl<'a> From<BeaconExecutionPayload<'a>> for ExecutionPayload {
    fn from(payload: BeaconExecutionPayload<'a>) -> Self {
        match payload {
            BeaconExecutionPayload::V1(payload) => Self::V1(ExecutionPayloadV1::from(payload)),
            BeaconExecutionPayload::V2(payload) => Self::V2(ExecutionPayloadV2::from(payload)),
            BeaconExecutionPayload::V3(payload) => Self::V3(ExecutionPayloadV3::from(payload)),
            BeaconExecutionPayload::V4(payload) => Self::V4(ExecutionPayloadV4::from(payload)),
        }
    }
}

impl<'a> From<&'a ExecutionPayload> for BeaconExecutionPayload<'a> {
    fn from(value: &'a ExecutionPayload) -> Self {
        match value {
            ExecutionPayload::V1(payload) => {
                BeaconExecutionPayload::V1(BeaconExecutionPayloadV1::from(payload))
            }
            ExecutionPayload::V2(payload) => {
                BeaconExecutionPayload::V2(BeaconExecutionPayloadV2::from(payload))
            }
            ExecutionPayload::V3(payload) => {
                BeaconExecutionPayload::V3(BeaconExecutionPayloadV3::from(payload))
            }
            ExecutionPayload::V4(payload) => {
                BeaconExecutionPayload::V4(BeaconExecutionPayloadV4::from(payload))
            }
        }
    }
}

impl SerializeAs<ExecutionPayload> for BeaconExecutionPayload<'_> {
    fn serialize_as<S>(source: &ExecutionPayload, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        beacon_payload::serialize(source, serializer)
    }
}

impl<'de> DeserializeAs<'de, ExecutionPayload> for BeaconExecutionPayload<'de> {
    fn deserialize_as<D>(deserializer: D) -> Result<ExecutionPayload, D::Error>
    where
        D: Deserializer<'de>,
    {
        beacon_payload::deserialize(deserializer)
    }
}

/// Module providing serialization and deserialization support for the beacon API payload
/// attributes.
pub mod beacon_payload {
    use super::*;

    /// Serialize the payload attributes for the beacon API.
    pub fn serialize<S>(
        payload_attributes: &ExecutionPayload,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BeaconExecutionPayload::from(payload_attributes).serialize(serializer)
    }

    /// Deserialize the payload attributes for the beacon API.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExecutionPayload, D::Error>
    where
        D: Deserializer<'de>,
    {
        BeaconExecutionPayload::deserialize(deserializer).map(Into::into)
    }
}

/// Helper for deserializing an execution layer [`ExecutionPayload`] payload from the consensus
/// layer format (snake_case)
pub fn execution_payload_from_beacon_str(val: &str) -> Result<ExecutionPayload, serde_json::Error> {
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct E {
        #[serde(deserialize_with = "beacon_payload::deserialize")]
        payload: ExecutionPayload,
    }
    serde_json::from_str::<E>(val).map(|val| val.payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    #[test]
    fn serde_get_payload_header_response() {
        let s = r#"{"version":"bellatrix","data":{"message":{"header":{"parent_hash":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","fee_recipient":"0xabcf8e0d4e9587369b2301d0790347320302cc09","state_root":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","receipts_root":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","logs_bloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prev_randao":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","block_number":"1","gas_limit":"1","gas_used":"1","timestamp":"1","extra_data":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","base_fee_per_gas":"1","block_hash":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","transactions_root":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"},"value":"1","pubkey":"0x93247f2209abcacf57b75a51dafae777f9dd38bc7053d1af526f220a7489a6d3a2753e5f3e8b1cfe39b56f43611df74a"},"signature":"0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"}}"#;
        let resp: GetExecutionPayloadHeaderResponse = serde_json::from_str(s).unwrap();
        let json: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(json, serde_json::to_value(resp).unwrap());
    }

    #[test]
    fn serde_payload_header() {
        let s = r#"{"parent_hash":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","fee_recipient":"0xabcf8e0d4e9587369b2301d0790347320302cc09","state_root":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","receipts_root":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","logs_bloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prev_randao":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","block_number":"1","gas_limit":"1","gas_used":"1","timestamp":"1","extra_data":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","base_fee_per_gas":"1","block_hash":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2","transactions_root":"0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"}"#;
        let header: ExecutionPayloadHeader = serde_json::from_str(s).unwrap();
        let json: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(json, serde_json::to_value(header).unwrap());
    }

    #[test]
    fn test_execution_payload_from_beacon_str() {
        // Test V1 payload
        let v1_payload_str = r#"{
            "parent_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "fee_recipient": "0xabcf8e0d4e9587369b2301d0790347320302cc09",
            "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "receipts_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "logs_bloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "prev_randao": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "block_number": "1",
            "gas_limit": "1000",
            "gas_used": "500",
            "timestamp": "1234567890",
            "extra_data": "0x",
            "base_fee_per_gas": "1000000000",
            "block_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "transactions": ["0x02f878831469668303f51d843b9ac9f9843b9aca0082520894c93269b73096998db66be0441e836d873535cb9c8894a19041886f000080c001a031cc29234036afbf9a1fb9476b463367cb1f957ac0b919b69bbc798436e604aaa018c4e9c3914eb27aadd0b91e10b18655739fcf8c1fc398763a9f1beecb8ddc86"]
        }"#;

        let payload = execution_payload_from_beacon_str(v1_payload_str).unwrap();
        match payload {
            ExecutionPayload::V1(v1) => {
                assert_eq!(v1.block_number, 1);
                assert_eq!(v1.gas_limit, 1000);
                assert_eq!(v1.gas_used, 500);
                assert_eq!(v1.timestamp, 1234567890);
                assert_eq!(v1.base_fee_per_gas, U256::from(1000000000u64));
                assert_eq!(v1.transactions.len(), 1);
            }
            _ => panic!("Expected V1 payload"),
        }

        // Test V2 payload with withdrawals
        let v2_payload_str = r#"{
            "parent_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "fee_recipient": "0xabcf8e0d4e9587369b2301d0790347320302cc09",
            "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "receipts_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "logs_bloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "prev_randao": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "block_number": "2",
            "gas_limit": "2000",
            "gas_used": "1000",
            "timestamp": "1234567891",
            "extra_data": "0x",
            "base_fee_per_gas": "2000000000",
            "block_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "transactions": [],
            "withdrawals": [
                {
                    "index": "0",
                    "validator_index": "0",
                    "address": "0xabcf8e0d4e9587369b2301d0790347320302cc09",
                    "amount": "32000000000"
                }
            ]
        }"#;

        let payload = execution_payload_from_beacon_str(v2_payload_str).unwrap();
        match payload {
            ExecutionPayload::V2(v2) => {
                assert_eq!(v2.payload_inner.block_number, 2);
                assert_eq!(v2.payload_inner.gas_limit, 2000);
                assert_eq!(v2.payload_inner.gas_used, 1000);
                assert_eq!(v2.withdrawals.len(), 1);
                assert_eq!(v2.withdrawals[0].index, 0);
                assert_eq!(v2.withdrawals[0].amount, 32000000000);
            }
            _ => panic!("Expected V2 payload"),
        }

        // Test V3 payload with blob gas fields
        let v3_payload_str = r#"{
            "parent_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "fee_recipient": "0xabcf8e0d4e9587369b2301d0790347320302cc09",
            "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "receipts_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "logs_bloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "prev_randao": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "block_number": "3",
            "gas_limit": "3000",
            "gas_used": "1500",
            "timestamp": "1234567892",
            "extra_data": "0x",
            "base_fee_per_gas": "3000000000",
            "block_hash": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
            "transactions": [],
            "withdrawals": [],
            "blob_gas_used": "131072",
            "excess_blob_gas": "262144"
        }"#;

        let payload = execution_payload_from_beacon_str(v3_payload_str).unwrap();
        match payload {
            ExecutionPayload::V3(v3) => {
                assert_eq!(v3.payload_inner.payload_inner.block_number, 3);
                assert_eq!(v3.payload_inner.payload_inner.gas_limit, 3000);
                assert_eq!(v3.payload_inner.payload_inner.gas_used, 1500);
                assert_eq!(v3.blob_gas_used, 131072);
                assert_eq!(v3.excess_blob_gas, 262144);
            }
            _ => panic!("Expected V3 payload"),
        }

        // Test invalid JSON should return error
        let invalid_json = r#"{ invalid json }"#;
        assert!(execution_payload_from_beacon_str(invalid_json).is_err());
    }

    #[test]
    fn test_extract_payload_from_beacon_block() {
        // Extracted from https://light-mainnet.beaconcha.in/slot/0x6ceadbf2a6adbbd64cbec33fdebbc582f25171cd30ac43f641cbe76ac7313ddf with only 2 transactions
        let beacon_block_json = r#"{
        "message": {
            "slot": "12225729",
            "proposer_index": "496520",
            "parent_root": "0x462f4abf9b6881724e6489085b3bb3931312e31ffb43f7cec3d0ee624dc2b58e",
            "state_root": "0x2c6e3ff0b0f7bc33b30a020e75e69c2bba26fb42a7e234e8275e655170925a71",
            "body": {
                "randao_reveal": "0x825dc181628713b55f40ed3f489be0c60f0513f88eecb25c7aa512ad24b912b3929bdf1930b50af4c18fb8b5f490352218a1c25adc01f7c3aaa50f982d762f589b4f5b6806e1d37e3f70af7afe990d1b1e8e337ac67b53bb7896f2052ecfccc1",
                "eth1_data": {
                    "deposit_root": "0x2ebc563cabdbbacbc56f0de1d2d1c2d5315a4b071fcd8566aabbf0a45161c64e",
                    "deposit_count": "2045305",
                    "block_hash": "0x0958d83550263ff0d9f9a0bc5ea3cd2a136e0933b6f43cbb17f36e4da8d809b1"
                },
                "graffiti": "0x52502d4e502076312e31372e3000000000000000000000000000000000000000",
                "proposer_slashings": [],
                "attester_slashings": [],
                "attestations": [],
                "deposits": [],
                "voluntary_exits": [],
                "sync_aggregate": {
                    "sync_committee_bits": "0x71b7f7596e64ef7f7ef4f938e9f68abfbfe95bff09393315bb93bbec7f7ef27effa4c7f25ba7cbdb87efbbf73fdaebb9efefeb3ef7fff8effafdd7aff5677bfc",
                    "sync_committee_signature": "0xb45afdccf46b3518c295407594d82fcfd7fbff767f1b7bb2e7c9bdc8a0229232d201247b449d4bddf01fc974ce0b57601987fb401bb346062e53981cfb81dd6f9c519d645248a46ceba695c2d9630cfc68b26efc35f6ca14c49af9170581ad90"
                },
                "execution_payload": {
                    "parent_hash": "0x3a798cf01d2c58af71b4d00f6b343c1faa88a4e8350d763d181928205ece05fa",
                    "fee_recipient": "0xdadB0d80178819F2319190D340ce9A924f783711",
                    "state_root": "0xf258006fe790a654326ceb30933e4216cd8cc2087b16f5189c8ac316d22b918f",
                    "receipts_root": "0xf8e75ccce80b590f6ac30b859f308edab28c9d87ed8ad50d257902df4ba05ca5",
                    "logs_bloom": "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffdfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                    "prev_randao": "0x6a4900e9b3958061c0d7fff943a7cd75d9c13f1ba16bd4f9b7dd31b72a82cc11",
                    "block_number": "23003311",
                    "gas_limit": "45043901",
                    "gas_used": "41421505",
                    "timestamp": "1753532771",
                    "extra_data": "0x4275696c6465724e6574202842656176657229",
                    "base_fee_per_gas": "236192093",
                    "block_hash": "0xa46feca5c8c498c9bf9741f3716d935b25a1a7ff2961d5d1e692f1e97f93a2ca",
                    "transactions": [
                        "0x02f901540182e0948505dec6f0ec8505dec6f0ec8307a12094360e051a25ca6decd2f0e91ea4c179a96c0e565e80b8e4ccf22927000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000bc396689893d065f41bc2c6ecbee5e008523344700000000000000000000000000000000000000000000000012651a94c4e78f2200000000000000000000000000000000000000000000034519cd0a9daa3a356e000000000000000000000000cd83055557536eff25fd0eafbc56e74a1b4260b30000000000000000000000000000000000000000000000000000000000000bb80000000000000000000000000000000000000000000000000000000000000000c001a0c45c9362e16382b20cc8f04599743f8cdb52031868251c5e4baa8add569019f1a02558d71f50ed265f7d63dde46e8ae477d58d833c36dfe9b4e7eb8783732cbf63",
                        "0x02f90405018265c38084151e020b8303ca8894a69babef1ca67a37ffaf7a485dfff3382056e78c83db9700b9014478e111f600000000000000000000000039807fc9a64a376b99b1cebde2e79e3826d39aa1000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c42f1c6b50000000000000000000000000000000000000000000000000abd98bd97ba9e63c00000000000000000000000000000000000000000000000033f578d0b9b5b4000000000000000000000000000000000000000000000402d44ba9f99ee186b7d50000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000006884c963ff8000000000000000000000000000000000000000000000000000000001227b00000000000000000000000000000000000000000000000000000000f90251d69439807fc9a64a376b99b1cebde2e79e3826d39aa1c0f85994c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2f842a00cb865ff1951c90111975d77bc75fa8312f25b08bb19b908f6b9c43691ac0cafa075245230289a9f0bf73a6c59aef6651b98b3833a62a3c0bd9ab6b0dec8ed4d8ff8dd9411b815efb8f581194ae79006d24e0d814b7697f6f8c6a00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000001a00000000000000000000000000000000000000000000000000000000000000004a0000000000000000000000000000000000000000000000000000000000000001ba0000000000000000000000000000000000000000000000000000000000000001ca02f2606b2c0d121a5cc1b59088ba7234e9d1c805f41724c938a2661d69532e0e9f8fe94dac17f958d2ee523a2206206994597c13d831ec7f8e7a00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000003a00000000000000000000000000000000000000000000000000000000000000004a0000000000000000000000000000000000000000000000000000000000000000aa0169228ca33ea854d54aa1e506e59ec687f618a41074f5f5de937a0e9c6343e5aa035d7fb7665514f774d2c2df607e197eb8674b6e63d2638472758647a2e67406aa03ad2db55fe5657fe773e3b7111e43f4b662a181a20e875b3b8be52dd9f0e233380a00cc4b1ab039d330bff37a79b961104004056d1fdf24ecf370036b453fdd4f2c7a0197e3c66c1f3fbaf9cb07902e38ebb451fd9f98e5a5a283d21716cfe3dc761fa"
                        ]
                    }
                }
            },
            "signature": "0x8a9cfe747dbb5d6ee1538638b2adfc304c8bcbeb03f489756ca7dc7a12081df892f38b924d19c9f5530c746b86a34beb019070bb7707de5a8efc8bdab8ca5668d7bb0e31c5ffd24913d23c80a6f6f70ba89e280dd46d19d6128ac7f42ffee93e"

        }"#;

        let beacon_block: BeaconBlockData =
            serde_json::from_str(beacon_block_json).expect("Failed to deserialize beacon block");

        let execution_payload = beacon_block.execution_payload();

        match execution_payload {
            ExecutionPayload::V1(v1) => {
                assert_eq!(v1.block_number, 23003311);
                assert_eq!(v1.gas_limit, 45043901);
                assert_eq!(v1.gas_used, 41421505);
                assert_eq!(v1.timestamp, 1753532771);
                assert_eq!(
                    v1.parent_hash.to_string(),
                    "0x3a798cf01d2c58af71b4d00f6b343c1faa88a4e8350d763d181928205ece05fa"
                );
                assert_eq!(
                    v1.fee_recipient.to_string().to_lowercase(),
                    "0xdadb0d80178819f2319190d340ce9a924f783711"
                );
                assert_eq!(
                    v1.block_hash.to_string(),
                    "0xa46feca5c8c498c9bf9741f3716d935b25a1a7ff2961d5d1e692f1e97f93a2ca"
                );

                // Verify 2 transaction were included
                assert_eq!(v1.transactions.len(), 2);
                assert!(v1.transactions[0].to_string().starts_with("0x02f901540182e094"));
            }
            ExecutionPayload::V2(_) => panic!("Expected V1 payload, got V2"),
            ExecutionPayload::V3(_) => panic!("Expected V1 payload, got V3"),
            ExecutionPayload::V4(_) => panic!("Expected V1 payload, got V4"),
        }
    }
}
