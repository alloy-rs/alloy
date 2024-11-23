use core::fmt;

use crate::UnknownTxEnvelope;
use alloy_consensus::{Transaction as TransactionTrait, TxEnvelope, TxType};
use alloy_eips::{
    eip2718::{Decodable2718, Eip2718Error, Encodable2718},
    eip2930::AccessList,
    eip7702::SignedAuthorization,
};
use alloy_primitives::{Bytes, ChainId, B256, U256};

/// Transaction type for a catch-all network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(alias = "AnyTransactionType")]
pub struct AnyTxType(pub u8);

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

#[cfg(feature = "serde")]
impl serde::Serialize for AnyTxType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use alloy_primitives::U8;
        U8::from(self.0).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for AnyTxType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use alloy_primitives::U8;
        U8::deserialize(deserializer).map(|t| Self(t.to::<u8>()))
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
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
