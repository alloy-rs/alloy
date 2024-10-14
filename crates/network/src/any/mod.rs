use crate::Network;
use alloy_consensus::{TxEnvelope, TxType, TypedTransaction};
use alloy_eips::{
    eip2718::{Decodable2718, Eip2718Error, Encodable2718},
    eip7702::SignedAuthorization,
};
use alloy_primitives::{Bytes, TxKind, B256, U256};
use alloy_rpc_types_eth::{
    AccessList, AnyTransactionReceipt, Block, Transaction, TransactionRequest,
};
use alloy_serde::{OtherFields, WithOtherFields};
use core::fmt;
use std::sync::OnceLock;

mod builder;

pub use alloy_consensus::{AnyHeader, AnyReceiptEnvelope};

/// A catch-all header type for handling headers on multiple networks.
pub type AnyRpcHeader = alloy_rpc_types_eth::Header<alloy_consensus::AnyHeader>;

/// A catch-all block type for handling blocks on multiple networks.
pub type AnyRpcBlock =
    WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>;

/// Transaction type for a catch-all network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[doc(alias = "AnyTransactionType")]
pub struct AnyTxType(u8);

impl fmt::Display for AnyTxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyTxType({})", self.0)
    }
}

impl TryFrom<u8> for AnyTxType {
    type Error = Eip2718Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<&AnyTxType> for u8 {
    fn from(value: &AnyTxType) -> Self {
        value.0
    }
}

impl From<AnyTxType> for u8 {
    fn from(value: AnyTxType) -> Self {
        value.0
    }
}

impl TryFrom<AnyTxType> for TxType {
    type Error = Eip2718Error;

    fn try_from(value: AnyTxType) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl From<TxType> for AnyTxType {
    fn from(value: TxType) -> Self {
        Self(value as u8)
    }
}

/// Memoization for deserialization of [`AnyTxEnvelope`] and [`AnyTypedTransaction`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(unnameable_types)]
pub struct DeserMemo {
    input: OnceLock<Bytes>,
    access_list: OnceLock<AccessList>,
    blob_versioned_hashes: OnceLock<Vec<B256>>,
    authorization_list: OnceLock<Vec<SignedAuthorization>>,
}

/// Transaction envelope for a catch-all network.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[doc(alias = "AnyTransactionEnvelope")]
pub enum AnyTxEnvelope {
    /// An Ethereum transaction.
    Ethereum(TxEnvelope),
    /// A transaction with unknown type.
    Other {
        /// Transaction type.
        #[serde(rename = "type")]
        ty: AnyTxType,
        /// Additional fields.
        fields: std::collections::BTreeMap<String, serde_json::Value>,

        /// Memoization for deserialization.
        #[serde(skip, default)]
        memo: DeserMemo,
    },
}

impl Encodable2718 for AnyTxEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self {
            Self::Ethereum(t) => t.type_flag(),
            Self::Other { ty, .. } => Some(ty.into()),
        }
    }

    fn encode_2718_len(&self) -> usize {
        match self {
            Self::Ethereum(t) => t.encode_2718_len(),
            Self::Other { .. } => 1,
        }
    }

    fn encode_2718(&self, out: &mut dyn alloy_primitives::bytes::BufMut) {
        match self {
            Self::Ethereum(t) => t.encode_2718(out),
            Self::Other { ty, .. } => {
                out.put_u8(ty.into());
            }
        }
    }
}

impl Decodable2718 for AnyTxEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> alloy_eips::eip2718::Eip2718Result<Self> {
        TxEnvelope::typed_decode(ty, buf).map(Self::Ethereum)
    }

    fn fallback_decode(buf: &mut &[u8]) -> alloy_eips::eip2718::Eip2718Result<Self> {
        TxEnvelope::fallback_decode(buf).map(Self::Ethereum)
    }
}

impl alloy_consensus::Transaction for AnyTxEnvelope {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        match self {
            Self::Ethereum(inner) => inner.chain_id(),
            Self::Other { fields, .. } => fields.get("chainId").and_then(|v| v.as_u64()),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.nonce(),
            Self::Other { fields, .. } => {
                fields.get("nonce").and_then(|v| v.as_u64()).expect("missing nonce in tx response")
            }
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.gas_limit(),
            Self::Other { fields, .. } => {
                fields.get("gas").and_then(|v| v.as_u64()).expect("missing gas in tx response")
            }
        }
    }

    fn gas_price(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.gas_price(),
            Self::Other { fields, .. } => {
                fields.get("gasPrice").and_then(|v| v.as_u64()).map(|v| v as u128)
            }
        }
    }

    fn max_fee_per_gas(&self) -> u128 {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_gas(),
            Self::Other { fields, .. } => fields
                .get("maxFeePerGas")
                .and_then(|v| v.as_u64())
                .expect("missing maxFeePerGas in tx response")
                as u128,
        }
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_priority_fee_per_gas(),
            Self::Other { fields, .. } => {
                fields.get("maxPriorityFeePerGas").and_then(|v| v.as_u64()).map(|v| v as u128)
            }
        }
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_blob_gas(),
            Self::Other { fields, .. } => {
                fields.get("maxFeePerBlobGas").and_then(|v| v.as_u64()).map(|v| v as u128)
            }
        }
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.max_priority_fee_per_gas()
            .or_else(|| self.gas_price())
            .expect("missing maxPriorityFeePerGas or gasPrice in tx response")
    }

    fn kind(&self) -> alloy_primitives::TxKind {
        match self {
            Self::Ethereum(inner) => inner.kind(),
            Self::Other { fields, .. } => fields
                .get("to")
                .or(Some(&serde_json::Value::Null))
                .map(|v| {
                    if v.is_null() {
                        TxKind::Create
                    } else {
                        TxKind::Call(
                            v.as_str()
                                .expect("to field is not a string")
                                .parse()
                                .expect("to field is not a valid address"),
                        )
                    }
                })
                .expect("missing to in tx response"),
        }
    }

    fn value(&self) -> U256 {
        match self {
            Self::Ethereum(inner) => inner.value(),
            Self::Other { fields, .. } => fields
                .get("value")
                .and_then(|v| v.as_str())
                .map(|v| v.parse().expect("invalid value"))
                .expect("missing value in tx response"),
        }
    }

    fn input(&self) -> &Bytes {
        match self {
            Self::Ethereum(inner) => inner.input(),
            Self::Other { fields, memo, .. } => memo.input.get_or_init(|| {
                fields
                    .get("input")
                    .and_then(|value| serde_json::from_value(value.clone()).ok())
                    .unwrap_or_default()
            }),
        }
    }

    fn ty(&self) -> u8 {
        match self {
            Self::Ethereum(inner) => inner.ty(),
            Self::Other { ty, .. } => ty.0,
        }
    }

    fn access_list(&self) -> Option<&AccessList> {
        match self {
            Self::Ethereum(inner) => inner.access_list(),
            Self::Other { fields, memo, .. } => {
                if fields.contains_key("accessList") {
                    Some(memo.access_list.get_or_init(|| {
                        fields
                            .get("accessList")
                            .and_then(|value| serde_json::from_value(value.clone()).ok())
                            .unwrap_or_default()
                    }))
                } else {
                    None
                }
            }
        }
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            Self::Ethereum(inner) => inner.blob_versioned_hashes(),
            Self::Other { fields, memo, .. } => {
                if fields.contains_key("blobVersionedHashes") {
                    Some(memo.blob_versioned_hashes.get_or_init(|| {
                        fields
                            .get("blobVersionedHashes")
                            .and_then(|value| serde_json::from_value(value.clone()).ok())
                            .unwrap_or_default()
                    }))
                } else {
                    None
                }
            }
        }
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            Self::Ethereum(inner) => inner.authorization_list(),
            Self::Other { fields, memo, .. } => {
                if fields.contains_key("authorizationList") {
                    Some(memo.authorization_list.get_or_init(|| {
                        fields
                            .get("authorizationList")
                            .and_then(|value| serde_json::from_value(value.clone()).ok())
                            .unwrap_or_default()
                    }))
                } else {
                    None
                }
            }
        }
    }
}

/// Unsigned transaction type for a catch-all network.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[doc(alias = "AnyTypedTx")]
pub enum AnyTypedTransaction {
    /// An Ethereum transaction.
    Ethereum(TypedTransaction),
    /// A transaction with unknown type.
    Other {
        #[serde(rename = "type")]
        /// Transaction type.
        ty: AnyTxType,
        /// Additional fields.
        fields: std::collections::BTreeMap<String, serde_json::Value>,

        /// Memoization for deserialization.
        #[serde(skip, default)]
        memo: DeserMemo,
    },
}

impl From<TypedTransaction> for AnyTypedTransaction {
    fn from(value: TypedTransaction) -> Self {
        Self::Ethereum(value)
    }
}

impl From<AnyTxEnvelope> for AnyTypedTransaction {
    fn from(value: AnyTxEnvelope) -> Self {
        match value {
            AnyTxEnvelope::Ethereum(tx) => Self::Ethereum(tx.into()),
            AnyTxEnvelope::Other { ty, fields, memo } => Self::Other { ty, fields, memo },
        }
    }
}

impl From<AnyTypedTransaction> for WithOtherFields<TransactionRequest> {
    fn from(value: AnyTypedTransaction) -> Self {
        match value {
            AnyTypedTransaction::Ethereum(tx) => Self::new(tx.into()),
            AnyTypedTransaction::Other { ty, mut fields, .. } => {
                fields.insert("type".to_string(), serde_json::Value::Number(ty.0.into()));
                Self { inner: Default::default(), other: OtherFields::new(fields) }
            }
        }
    }
}

impl From<AnyTxEnvelope> for WithOtherFields<TransactionRequest> {
    fn from(value: AnyTxEnvelope) -> Self {
        AnyTypedTransaction::from(value).into()
    }
}

impl alloy_consensus::Transaction for AnyTypedTransaction {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        match self {
            Self::Ethereum(inner) => inner.chain_id(),
            Self::Other { fields, .. } => fields.get("chainId").and_then(|v| v.as_u64()),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.nonce(),
            Self::Other { fields, .. } => {
                fields.get("nonce").and_then(|v| v.as_u64()).expect("missing nonce in tx response")
            }
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.gas_limit(),
            Self::Other { fields, .. } => {
                fields.get("gas").and_then(|v| v.as_u64()).expect("missing gas in tx response")
            }
        }
    }

    fn gas_price(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.gas_price(),
            Self::Other { fields, .. } => {
                fields.get("gasPrice").and_then(|v| v.as_u64()).map(|v| v as u128)
            }
        }
    }

    fn max_fee_per_gas(&self) -> u128 {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_gas(),
            Self::Other { fields, .. } => fields
                .get("maxFeePerGas")
                .and_then(|v| v.as_u64())
                .expect("missing maxFeePerGas in tx response")
                as u128,
        }
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_priority_fee_per_gas(),
            Self::Other { fields, .. } => {
                fields.get("maxPriorityFeePerGas").and_then(|v| v.as_u64()).map(|v| v as u128)
            }
        }
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_blob_gas(),
            Self::Other { fields, .. } => {
                fields.get("maxFeePerBlobGas").and_then(|v| v.as_u64()).map(|v| v as u128)
            }
        }
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.max_priority_fee_per_gas()
            .or_else(|| self.gas_price())
            .expect("missing maxPriorityFeePerGas or gasPrice in tx response")
    }

    fn kind(&self) -> alloy_primitives::TxKind {
        match self {
            Self::Ethereum(inner) => inner.kind(),
            Self::Other { fields, .. } => fields
                .get("to")
                .or(Some(&serde_json::Value::Null))
                .map(|v| {
                    if v.is_null() {
                        TxKind::Create
                    } else {
                        TxKind::Call(
                            v.as_str()
                                .expect("to field is not a string")
                                .parse()
                                .expect("to field is not a valid address"),
                        )
                    }
                })
                .expect("missing to in tx response"),
        }
    }

    fn value(&self) -> U256 {
        match self {
            Self::Ethereum(inner) => inner.value(),
            Self::Other { fields, .. } => fields
                .get("value")
                .and_then(|v| v.as_str())
                .map(|v| v.parse().expect("invalid value"))
                .expect("missing value in tx response"),
        }
    }

    fn input(&self) -> &Bytes {
        match self {
            Self::Ethereum(inner) => inner.input(),
            Self::Other { fields, memo, .. } => memo.input.get_or_init(|| {
                fields
                    .get("input")
                    .and_then(|value| serde_json::from_value(value.clone()).ok())
                    .unwrap_or_default()
            }),
        }
    }

    fn ty(&self) -> u8 {
        match self {
            Self::Ethereum(inner) => inner.ty(),
            Self::Other { ty, .. } => ty.0,
        }
    }

    fn access_list(&self) -> Option<&AccessList> {
        match self {
            Self::Ethereum(inner) => inner.access_list(),
            Self::Other { fields, memo, .. } => {
                if fields.contains_key("accessList") {
                    Some(memo.access_list.get_or_init(|| {
                        fields
                            .get("accessList")
                            .and_then(|value| serde_json::from_value(value.clone()).ok())
                            .unwrap_or_default()
                    }))
                } else {
                    None
                }
            }
        }
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            Self::Ethereum(inner) => inner.blob_versioned_hashes(),
            Self::Other { fields, memo, .. } => {
                if fields.contains_key("blobVersionedHashes") {
                    Some(memo.blob_versioned_hashes.get_or_init(|| {
                        fields
                            .get("blobVersionedHashes")
                            .and_then(|value| serde_json::from_value(value.clone()).ok())
                            .unwrap_or_default()
                    }))
                } else {
                    None
                }
            }
        }
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            Self::Ethereum(inner) => inner.authorization_list(),
            Self::Other { fields, memo, .. } => {
                if fields.contains_key("authorizationList") {
                    Some(memo.authorization_list.get_or_init(|| {
                        fields
                            .get("authorizationList")
                            .and_then(|value| serde_json::from_value(value.clone()).ok())
                            .unwrap_or_default()
                    }))
                } else {
                    None
                }
            }
        }
    }
}

/// Types for a catch-all network.
///
/// `AnyNetwork`'s associated types allow for many different types of
/// transactions, using catch-all fields. This [`Network`] should be used
/// only when the application needs to support multiple networks via the same
/// codepaths without knowing the networks at compile time.
///
/// ## Rough Edges
///
/// Supporting arbitrary unknown types is hard, and users of this network
/// should be aware of the following:
///
/// - The implementation of [`Decodable2718`] for [`AnyTxEnvelope`] will not work for non-Ethereum
///   transaction types. It will succesfully decode an Ethereum [`TxEnvelope`], but will decode only
///   the type for any unknown transaction type. It will also leave the buffer unconsumed, which
///   will cause further deserialization to produce erroneous results.
/// - The implementation of [`Encodable2718`] for [`AnyTypedTransaction`] will not work for
///   non-Ethereum transaction types. It will encode the type for any unknown transaction type, but
///   will not encode any other fields. This is symmetric with the decoding behavior, but still
///   erroneous.
/// - The [`TransactionRequest`] will build ONLY Ethereum types. It will error when attempting to
///   build any unknown type.
/// - The [`Network::TransactionResponse`] may deserialize unknown metadata fields into the inner
///   [`AnyTxEnvelope`], rather than into the outer [`WithOtherFields`].
#[derive(Clone, Copy, Debug)]
pub struct AnyNetwork {
    _private: (),
}

impl Network for AnyNetwork {
    type TxType = AnyTxType;

    type TxEnvelope = AnyTxEnvelope;

    type UnsignedTx = AnyTypedTransaction;

    type ReceiptEnvelope = alloy_consensus::AnyReceiptEnvelope;

    type Header = alloy_consensus::AnyHeader;

    type TransactionRequest = WithOtherFields<TransactionRequest>;

    type TransactionResponse = WithOtherFields<Transaction<Self::TxEnvelope>>;

    type ReceiptResponse = AnyTransactionReceipt;

    type HeaderResponse = AnyRpcHeader;

    type BlockResponse = AnyRpcBlock;
}
