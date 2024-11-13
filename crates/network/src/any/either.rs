use crate::{UnknownTxEnvelope, UnknownTypedTransaction};
use alloy_consensus::{Transaction as TransactionTrait, TxEnvelope, TypedTransaction};
use alloy_eips::{
    eip2718::{Decodable2718, Encodable2718},
    eip7702::SignedAuthorization,
};
use alloy_primitives::{Bytes, ChainId, B256, U256};
use alloy_rpc_types_eth::{AccessList, TransactionRequest};
use alloy_serde::WithOtherFields;

/// Unsigned transaction type for a catch-all network.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[doc(alias = "AnyTypedTx")]
pub enum AnyTypedTransaction {
    /// An Ethereum transaction.
    Ethereum(TypedTransaction),
    /// A transaction with unknown type.
    Unknown(UnknownTypedTransaction),
}

impl From<UnknownTypedTransaction> for AnyTypedTransaction {
    fn from(value: UnknownTypedTransaction) -> Self {
        Self::Unknown(value)
    }
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
            AnyTxEnvelope::Unknown(UnknownTxEnvelope { inner, .. }) => inner.into(),
        }
    }
}

impl From<AnyTypedTransaction> for WithOtherFields<TransactionRequest> {
    fn from(value: AnyTypedTransaction) -> Self {
        match value {
            AnyTypedTransaction::Ethereum(tx) => Self::new(tx.into()),
            AnyTypedTransaction::Unknown(UnknownTypedTransaction { ty, mut fields, .. }) => {
                fields.insert("type".to_string(), serde_json::Value::Number(ty.0.into()));
                Self { inner: Default::default(), other: fields }
            }
        }
    }
}

impl From<AnyTxEnvelope> for WithOtherFields<TransactionRequest> {
    fn from(value: AnyTxEnvelope) -> Self {
        AnyTypedTransaction::from(value).into()
    }
}

impl TransactionTrait for AnyTypedTransaction {
    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            Self::Ethereum(inner) => inner.chain_id(),
            Self::Unknown(inner) => inner.chain_id(),
        }
    }

    #[inline]
    fn nonce(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.nonce(),
            Self::Unknown(inner) => inner.nonce(),
        }
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.gas_limit(),
            Self::Unknown(inner) => inner.gas_limit(),
        }
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.gas_price(),
            Self::Unknown(inner) => inner.gas_price(),
        }
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_gas(),
            Self::Unknown(inner) => inner.max_fee_per_gas(),
        }
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_priority_fee_per_gas(),
            Self::Unknown(inner) => inner.max_priority_fee_per_gas(),
        }
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_blob_gas(),
            Self::Unknown(inner) => inner.max_fee_per_blob_gas(),
        }
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.max_priority_fee_per_gas().or_else(|| self.gas_price()).unwrap_or_default()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        match self {
            Self::Ethereum(inner) => inner.effective_gas_price(base_fee),
            Self::Unknown(inner) => inner.effective_gas_price(base_fee),
        }
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        match self {
            Self::Ethereum(inner) => inner.is_dynamic_fee(),
            Self::Unknown(inner) => inner.is_dynamic_fee(),
        }
    }

    fn kind(&self) -> alloy_primitives::TxKind {
        match self {
            Self::Ethereum(inner) => inner.kind(),
            Self::Unknown(inner) => inner.kind(),
        }
    }

    #[inline]
    fn value(&self) -> U256 {
        match self {
            Self::Ethereum(inner) => inner.value(),
            Self::Unknown(inner) => inner.value(),
        }
    }

    #[inline]
    fn input(&self) -> &Bytes {
        match self {
            Self::Ethereum(inner) => inner.input(),
            Self::Unknown(inner) => inner.input(),
        }
    }

    #[inline]
    fn ty(&self) -> u8 {
        match self {
            Self::Ethereum(inner) => inner.ty(),
            Self::Unknown(inner) => inner.ty(),
        }
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        match self {
            Self::Ethereum(inner) => inner.access_list(),
            Self::Unknown(inner) => inner.access_list(),
        }
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            Self::Ethereum(inner) => inner.blob_versioned_hashes(),
            Self::Unknown(inner) => inner.blob_versioned_hashes(),
        }
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            Self::Ethereum(inner) => inner.authorization_list(),
            Self::Unknown(inner) => inner.authorization_list(),
        }
    }
}

/// Transaction envelope for a catch-all network.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[doc(alias = "AnyTransactionEnvelope")]
pub enum AnyTxEnvelope {
    /// An Ethereum transaction.
    Ethereum(TxEnvelope),
    /// A transaction with unknown type.
    Unknown(UnknownTxEnvelope),
}

impl Encodable2718 for AnyTxEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self {
            Self::Ethereum(t) => t.type_flag(),
            Self::Unknown(inner) => Some(inner.ty()),
        }
    }

    fn encode_2718_len(&self) -> usize {
        match self {
            Self::Ethereum(t) => t.encode_2718_len(),
            Self::Unknown(_) => 1,
        }
    }

    #[track_caller]
    fn encode_2718(&self, out: &mut dyn alloy_primitives::bytes::BufMut) {
        match self {
            Self::Ethereum(t) => t.encode_2718(out),
            Self::Unknown(inner) => {
                panic!(
                    "Attempted to encode unknown transaction type: {}. This is not a bug in alloy. To encode or decode unknown transaction types, use a custom Transaction type and a custom Network implementation. See https://docs.rs/alloy-network/latest/alloy_network/ for network documentation.",
                    inner.as_ref().ty
                )
            }
        }
    }

    fn trie_hash(&self) -> B256 {
        match self {
            Self::Ethereum(tx) => tx.trie_hash(),
            Self::Unknown(inner) => inner.hash,
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

impl TransactionTrait for AnyTxEnvelope {
    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            Self::Ethereum(inner) => inner.chain_id(),
            Self::Unknown(inner) => inner.chain_id(),
        }
    }

    #[inline]
    fn nonce(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.nonce(),
            Self::Unknown(inner) => inner.nonce(),
        }
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        match self {
            Self::Ethereum(inner) => inner.gas_limit(),
            Self::Unknown(inner) => inner.gas_limit(),
        }
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.gas_price(),
            Self::Unknown(inner) => inner.gas_price(),
        }
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_gas(),
            Self::Unknown(inner) => inner.max_fee_per_gas(),
        }
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_priority_fee_per_gas(),
            Self::Unknown(inner) => inner.max_priority_fee_per_gas(),
        }
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            Self::Ethereum(inner) => inner.max_fee_per_blob_gas(),
            Self::Unknown(inner) => inner.max_fee_per_blob_gas(),
        }
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.max_priority_fee_per_gas().or_else(|| self.gas_price()).unwrap_or_default()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        match self {
            Self::Ethereum(inner) => inner.effective_gas_price(base_fee),
            Self::Unknown(inner) => inner.effective_gas_price(base_fee),
        }
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        match self {
            Self::Ethereum(inner) => inner.is_dynamic_fee(),
            Self::Unknown(inner) => inner.is_dynamic_fee(),
        }
    }

    fn kind(&self) -> alloy_primitives::TxKind {
        match self {
            Self::Ethereum(inner) => inner.kind(),
            Self::Unknown(inner) => inner.kind(),
        }
    }

    #[inline]
    fn value(&self) -> U256 {
        match self {
            Self::Ethereum(inner) => inner.value(),
            Self::Unknown(inner) => inner.value(),
        }
    }

    #[inline]
    fn input(&self) -> &Bytes {
        match self {
            Self::Ethereum(inner) => inner.input(),
            Self::Unknown(inner) => inner.input(),
        }
    }

    #[inline]
    fn ty(&self) -> u8 {
        match self {
            Self::Ethereum(inner) => inner.ty(),
            Self::Unknown(inner) => inner.ty(),
        }
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        match self {
            Self::Ethereum(inner) => inner.access_list(),
            Self::Unknown(inner) => inner.access_list(),
        }
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            Self::Ethereum(inner) => inner.blob_versioned_hashes(),
            Self::Unknown(inner) => inner.blob_versioned_hashes(),
        }
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            Self::Ethereum(inner) => inner.authorization_list(),
            Self::Unknown(inner) => inner.authorization_list(),
        }
    }
}
