use core::fmt;
use std::sync::OnceLock;

use alloy_consensus::{TxType, Typed2718};
use alloy_eips::{eip2718::Eip2718Error, eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, B256, U128, U256, U64, U8};
use alloy_serde::OtherFields;

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

impl serde::Serialize for AnyTxType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        U8::from(self.0).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for AnyTxType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
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

impl Typed2718 for AnyTxType {
    fn ty(&self) -> u8 {
        self.0
    }
}

/// Memoization for deserialization of [`UnknownTxEnvelope`],
/// [`UnknownTypedTransaction`], [`AnyTxEnvelope`](crate::AnyTxEnvelope),
/// and [`AnyTypedTransaction`](crate::AnyTypedTransaction).
/// Setting these manually is discouraged, however the fields are left public
/// for power users :)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeserMemo {
    /// Memoized decoded input data.
    pub input: OnceLock<Bytes>,
    /// Memoized decoded access list.
    pub access_list: OnceLock<AccessList>,
    /// Memoized decoded blob versioned hashes.
    pub blob_versioned_hashes: OnceLock<Vec<B256>>,
    /// Memoized decoded authorization list.
    pub authorization_list: OnceLock<Vec<SignedAuthorization>>,
}

/// A typed transaction of an unknown Network
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[doc(alias = "UnknownTypedTx")]
pub struct UnknownTypedTransaction {
    #[serde(rename = "type")]
    /// Transaction type.
    pub ty: AnyTxType,

    /// Additional fields.
    #[serde(flatten)]
    pub fields: OtherFields,

    /// Memoization for deserialization.
    #[serde(skip, default)]
    pub memo: DeserMemo,
}

impl alloy_consensus::Transaction for UnknownTypedTransaction {
    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.fields.get_deserialized::<U64>("chainId").and_then(Result::ok).map(|v| v.to())
    }

    #[inline]
    fn nonce(&self) -> u64 {
        self.fields.get_deserialized::<U64>("nonce").and_then(Result::ok).unwrap_or_default().to()
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        self.fields.get_deserialized::<U64>("gas").and_then(Result::ok).unwrap_or_default().to()
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        self.fields.get_deserialized::<U128>("gasPrice").and_then(Result::ok).map(|v| v.to())
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        self.fields
            .get_deserialized::<U128>("maxFeePerGas")
            .and_then(Result::ok)
            .unwrap_or_default()
            .to()
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.fields
            .get_deserialized::<U128>("maxPriorityFeePerGas")
            .and_then(Result::ok)
            .map(|v| v.to())
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.fields
            .get_deserialized::<U128>("maxFeePerBlobGas")
            .and_then(Result::ok)
            .map(|v| v.to())
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.max_priority_fee_per_gas().or(self.gas_price()).unwrap_or_default()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        if let Some(gas_price) = self.gas_price() {
            return gas_price;
        }

        let max_fee = self.max_fee_per_gas();
        if max_fee == 0 {
            return 0;
        }

        let Some(max_prio_fee) = self.max_priority_fee_per_gas() else {
            return max_fee;
        };

        alloy_eips::eip1559::calc_effective_gas_price(max_fee, max_prio_fee, base_fee)
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        self.fields.get_deserialized::<U128>("maxFeePerGas").is_some()
            || self.fields.get_deserialized::<U128>("maxFeePerBlobGas").is_some()
    }

    #[inline]
    fn kind(&self) -> TxKind {
        self.fields
            .get("to")
            .or(Some(&serde_json::Value::Null))
            .and_then(|v| {
                if v.is_null() {
                    Some(TxKind::Create)
                } else {
                    v.as_str().and_then(|v| v.parse::<Address>().ok().map(Into::into))
                }
            })
            .unwrap_or_default()
    }

    #[inline]
    fn is_create(&self) -> bool {
        self.fields.get("to").is_none_or(|v| v.is_null())
    }

    #[inline]
    fn value(&self) -> U256 {
        self.fields.get_deserialized("value").and_then(Result::ok).unwrap_or_default()
    }

    #[inline]
    fn input(&self) -> &Bytes {
        self.memo.input.get_or_init(|| {
            self.fields.get_deserialized("input").and_then(Result::ok).unwrap_or_default()
        })
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        if self.fields.contains_key("accessList") {
            Some(self.memo.access_list.get_or_init(|| {
                self.fields.get_deserialized("accessList").and_then(Result::ok).unwrap_or_default()
            }))
        } else {
            None
        }
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        if self.fields.contains_key("blobVersionedHashes") {
            Some(self.memo.blob_versioned_hashes.get_or_init(|| {
                self.fields
                    .get_deserialized("blobVersionedHashes")
                    .and_then(Result::ok)
                    .unwrap_or_default()
            }))
        } else {
            None
        }
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        if self.fields.contains_key("authorizationList") {
            Some(self.memo.authorization_list.get_or_init(|| {
                self.fields
                    .get_deserialized("authorizationList")
                    .and_then(Result::ok)
                    .unwrap_or_default()
            }))
        } else {
            None
        }
    }
}

impl Typed2718 for UnknownTxEnvelope {
    fn ty(&self) -> u8 {
        self.inner.ty.0
    }
}

impl Typed2718 for UnknownTypedTransaction {
    fn ty(&self) -> u8 {
        self.ty.0
    }
}

/// A transaction envelope from an unknown network.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[doc(alias = "UnknownTransactionEnvelope")]
pub struct UnknownTxEnvelope {
    /// Transaction hash.
    pub hash: B256,

    /// Transaction type.
    #[serde(flatten)]
    pub inner: UnknownTypedTransaction,
}

impl AsRef<UnknownTypedTransaction> for UnknownTxEnvelope {
    fn as_ref(&self) -> &UnknownTypedTransaction {
        &self.inner
    }
}

impl alloy_consensus::Transaction for UnknownTxEnvelope {
    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.inner.chain_id()
    }

    #[inline]
    fn nonce(&self) -> u64 {
        self.inner.nonce()
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        self.inner.gas_limit()
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        self.inner.gas_price()
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        self.inner.max_fee_per_gas()
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.inner.max_priority_fee_per_gas()
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.inner.max_fee_per_blob_gas()
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.inner.priority_fee_or_price()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        self.inner.effective_gas_price(base_fee)
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        self.inner.is_dynamic_fee()
    }

    #[inline]
    fn kind(&self) -> TxKind {
        self.inner.kind()
    }

    #[inline]
    fn is_create(&self) -> bool {
        self.inner.is_create()
    }

    #[inline]
    fn value(&self) -> U256 {
        self.inner.value()
    }

    #[inline]
    fn input(&self) -> &Bytes {
        self.inner.input()
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        self.inner.access_list()
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.inner.blob_versioned_hashes()
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.inner.authorization_list()
    }
}
