use crate::Network;
use alloy_consensus::{TxEnvelope, TxType, TypedTransaction};
use alloy_eips::eip2718::{Decodable2718, Eip2718Error, Encodable2718};
use alloy_rpc_types_eth::{AnyTransactionReceipt, Transaction, TransactionRequest};
use alloy_serde::{OtherFields, WithOtherFields};
use core::fmt;

mod builder;

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

/// Transaction envelope for a catch-all network.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    },
}

impl Encodable2718 for AnyTxEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self {
            AnyTxEnvelope::Ethereum(t) => t.type_flag(),
            AnyTxEnvelope::Other { ty, .. } => Some(ty.into()),
        }
    }

    fn encode_2718_len(&self) -> usize {
        match self {
            AnyTxEnvelope::Ethereum(t) => t.encode_2718_len(),
            AnyTxEnvelope::Other { .. } => 1,
        }
    }

    fn encode_2718(&self, out: &mut dyn alloy_primitives::bytes::BufMut) {
        match self {
            AnyTxEnvelope::Ethereum(t) => t.encode_2718(out),
            AnyTxEnvelope::Other { ty, .. } => {
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

/// Unsigned transaction type for a catch-all network.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
            AnyTxEnvelope::Other { ty, fields } => Self::Other { ty, fields },
        }
    }
}

impl From<AnyTypedTransaction> for WithOtherFields<TransactionRequest> {
    fn from(value: AnyTypedTransaction) -> Self {
        match value {
            AnyTypedTransaction::Ethereum(tx) => WithOtherFields::new(tx.into()),
            AnyTypedTransaction::Other { ty, mut fields } => {
                fields.insert("type".to_string(), serde_json::Value::Number(ty.0.into()));
                WithOtherFields { inner: Default::default(), other: OtherFields::new(fields) }
            }
        }
    }
}

impl From<AnyTxEnvelope> for WithOtherFields<TransactionRequest> {
    fn from(value: AnyTxEnvelope) -> Self {
        AnyTypedTransaction::from(value).into()
    }
}

/// Types for a catch-all network.
///
/// Essentially just returns the regular Ethereum types + a catch all field.
/// This [`Network`] should be used only when the network is not known at
/// compile time.
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

    type TransactionResponse = WithOtherFields<Transaction>;

    type ReceiptResponse = AnyTransactionReceipt;

    type HeaderResponse = alloy_rpc_types_eth::AnyNetworkHeader;

    type BlockResponse = alloy_rpc_types_eth::AnyNetworkBlock;
}
