use core::fmt;
use std::sync::OnceLock;

use alloy_consensus::TxType;
use alloy_eips::{eip2718::Eip2718Error, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, TxKind, B256};
use alloy_rpc_types_eth::AccessList;

/// Transaction type for a catch-all network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Memoization for deserialization of [`AnyTxEnvelope`] and
/// [`AnyTypedTransaction`]. Setting these manually is discouraged, however the
/// fields are left public for power users :)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(unnameable_types)]
pub struct DeserMemo {
    pub input: OnceLock<Bytes>,
    pub access_list: OnceLock<AccessList>,
    pub blob_versioned_hashes: OnceLock<Vec<B256>>,
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
    pub fields: std::collections::BTreeMap<String, serde_json::Value>,

    /// Memoization for deserialization.
    #[serde(skip, default)]
    pub memo: DeserMemo,
}

impl UnknownTypedTransaction {
    /// Select a field by key and attempt to deserialize it.
    ///
    /// This method will return `None` if the key is not present in the fields,
    /// or if the transaction is already fully deserialized (i.e. it is an
    /// Ethereum [`TxEnvelope`]). Otherwise, it will attempt to deserialize the
    /// field and return the result wrapped in a `Some`.
    pub fn deser_by_key<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<serde_json::Result<T>> {
        self.fields.get(key).cloned().map(serde_json::from_value)
    }
}

impl alloy_consensus::Transaction for UnknownTypedTransaction {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        self.deser_by_key("chainId").map(Result::ok).flatten()
    }

    fn nonce(&self) -> u64 {
        self.deser_by_key("nonce").map(Result::ok).flatten().unwrap_or_default()
    }

    fn gas_limit(&self) -> u64 {
        self.deser_by_key("gasLimit").map(Result::ok).flatten().unwrap_or_default()
    }

    fn gas_price(&self) -> Option<u128> {
        self.deser_by_key("gasPrice").map(Result::ok).flatten()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.deser_by_key("maxFeePerGas").map(Result::ok).flatten().unwrap_or_default()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.deser_by_key("maxPriorityFeePerGas").map(Result::ok).flatten()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.deser_by_key("maxFeePerBlobGas").map(Result::ok).flatten()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.gas_price().or(self.max_priority_fee_per_gas()).unwrap_or_default()
    }

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

    fn value(&self) -> alloy_primitives::U256 {
        self.deser_by_key("value").map(Result::ok).flatten().unwrap_or_default()
    }

    fn input(&self) -> &Bytes {
        self.memo
            .input
            .get_or_init(|| self.deser_by_key("input").and_then(Result::ok).unwrap_or_default())
    }

    fn ty(&self) -> u8 {
        self.ty.0
    }

    fn access_list(&self) -> Option<&AccessList> {
        if self.fields.contains_key("accessList") {
            Some(self.memo.access_list.get_or_init(|| {
                self.deser_by_key("accessList").and_then(Result::ok).unwrap_or_default()
            }))
        } else {
            None
        }
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        if self.fields.contains_key("blobVersionedHashes") {
            Some(self.memo.blob_versioned_hashes.get_or_init(|| {
                self.deser_by_key("blobVersionedHashes").and_then(Result::ok).unwrap_or_default()
            }))
        } else {
            None
        }
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        if self.fields.contains_key("authorizationList") {
            Some(self.memo.authorization_list.get_or_init(|| {
                self.deser_by_key("authorizationList").and_then(Result::ok).unwrap_or_default()
            }))
        } else {
            None
        }
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
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        self.inner.chain_id()
    }

    fn nonce(&self) -> u64 {
        self.inner.nonce()
    }

    fn gas_limit(&self) -> u64 {
        self.inner.gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        self.inner.gas_price()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.inner.max_fee_per_gas()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.inner.max_priority_fee_per_gas()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.inner.max_fee_per_blob_gas()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.inner.priority_fee_or_price()
    }

    fn kind(&self) -> TxKind {
        self.inner.kind()
    }

    fn value(&self) -> alloy_primitives::U256 {
        self.inner.value()
    }

    fn input(&self) -> &Bytes {
        self.inner.input()
    }

    fn ty(&self) -> u8 {
        self.inner.ty()
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.inner.access_list()
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.inner.blob_versioned_hashes()
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.inner.authorization_list()
    }
}
