use core::fmt;
use std::sync::OnceLock;

use super::delegate_transaction;
use alloy_consensus::{TxType, Typed2718};
use alloy_eips::{eip2718::Eip2718Error, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, B256, U128, U256, U64, U8};
use alloy_rpc_types_eth::AccessList;
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

/// Memoized deserialization of an unknown transaction's fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct DeserMemo {
    input: OnceLock<Bytes>,
    access_list: OnceLock<AccessList>,
    blob_versioned_hashes: OnceLock<Vec<B256>>,
    authorization_list: OnceLock<Vec<SignedAuthorization>>,
}

/// A typed transaction of an unknown Network
///
/// Equality compares the transaction type and fields, ignoring the deserialization cache.
/// Use [`Self::fields_mut`] to change fields and invalidate their cached values.
#[derive(Debug, Clone, Eq, serde::Serialize, serde::Deserialize)]
#[doc(alias = "UnknownTypedTx")]
pub struct UnknownTypedTransaction {
    /// Transaction type.
    #[serde(rename = "type")]
    pub ty: AnyTxType,

    /// Additional fields.
    #[serde(flatten)]
    fields: OtherFields,

    /// Memoization for deserialization.
    #[serde(skip, default)]
    memo: DeserMemo,
}

impl UnknownTypedTransaction {
    /// Creates an unknown transaction with the given type and fields.
    pub fn new(ty: AnyTxType, fields: OtherFields) -> Self {
        Self { ty, fields, memo: DeserMemo::default() }
    }

    /// Returns the transaction's additional fields.
    pub const fn fields(&self) -> &OtherFields {
        &self.fields
    }

    /// Returns mutable additional fields, invalidating all cached deserialization.
    pub fn fields_mut(&mut self) -> &mut OtherFields {
        self.memo = DeserMemo::default();
        &mut self.fields
    }

    /// Consumes the transaction, returning its additional fields.
    pub fn into_fields(self) -> OtherFields {
        self.fields
    }
}

impl PartialEq for UnknownTypedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.fields == other.fields
    }
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
        self.fields.contains_key("maxFeePerGas") || self.fields.contains_key("maxFeePerBlobGas")
    }

    #[inline]
    fn kind(&self) -> TxKind {
        self.fields
            .get("to")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<Address>().ok())
            .map_or(TxKind::Create, TxKind::Call)
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
    delegate_transaction!(self => &self.inner);
}

#[cfg(test)]
mod tests {
    use alloy_consensus::Transaction;

    use crate::{AnyRpcTransaction, AnyTxEnvelope};

    use super::*;

    #[test]
    fn equality_ignores_deserialization_cache() {
        let tx: UnknownTypedTransaction = serde_json::from_value(serde_json::json!({
            "type": "0x7e",
            "input": "0x1234",
            "accessList": [],
            "blobVersionedHashes": [],
            "authorizationList": [],
        }))
        .unwrap();
        let cold = tx.clone();

        assert_eq!(tx.input().as_ref(), &[0x12, 0x34]);
        assert_eq!(tx, cold);
        assert!(tx.access_list().is_some());
        assert_eq!(tx, cold);
        assert!(tx.blob_versioned_hashes().is_some());
        assert_eq!(tx, cold);
        assert!(tx.authorization_list().is_some());
        assert_eq!(tx, cold);

        let decoded: UnknownTypedTransaction =
            serde_json::from_str(&serde_json::to_string(&tx).unwrap()).unwrap();
        assert_eq!(tx, decoded);
        assert_eq!(
            crate::AnyTypedTransaction::Unknown(tx.clone()),
            crate::AnyTypedTransaction::Unknown(cold.clone()),
        );
        assert_eq!(
            AnyTxEnvelope::Unknown(UnknownTxEnvelope { hash: B256::ZERO, inner: tx }),
            AnyTxEnvelope::Unknown(UnknownTxEnvelope { hash: B256::ZERO, inner: cold }),
        );
    }

    #[test]
    fn equality_compares_type_and_fields() {
        let tx = UnknownTypedTransaction::new(AnyTxType(0x7e), Default::default());
        let mut different_type = tx.clone();
        different_type.ty = AnyTxType(0x7f);
        assert_ne!(tx, different_type);

        let mut different_fields = tx.clone();
        different_fields.fields_mut().insert("nonce".into(), serde_json::json!("0x1"));
        assert_ne!(tx, different_fields);
    }

    #[test]
    fn field_mutation_invalidates_deserialization_cache() {
        let mut tx: UnknownTypedTransaction = serde_json::from_value(serde_json::json!({
            "type": "0x7e",
            "input": "0x1234",
            "accessList": [],
            "blobVersionedHashes": [],
            "authorizationList": [],
        }))
        .unwrap();

        assert_eq!(tx.input().as_ref(), &[0x12, 0x34]);
        assert!(tx.access_list().unwrap().is_empty());
        assert!(tx.blob_versioned_hashes().unwrap().is_empty());
        assert!(tx.authorization_list().unwrap().is_empty());

        let replacement = serde_json::json!({
            "type": "0x7e",
            "input": "0x5678",
            "accessList": [{ "address": Address::ZERO, "storageKeys": [B256::ZERO] }],
            "blobVersionedHashes": [B256::ZERO],
            "authorizationList": [{
                "chainId": "0x1",
                "address": Address::ZERO,
                "nonce": "0x0",
                "yParity": "0x0",
                "r": "0x1",
                "s": "0x1"
            }],
        });
        let expected: UnknownTypedTransaction =
            serde_json::from_value(replacement.clone()).unwrap();
        *tx.fields_mut() = expected.fields().clone();

        assert_eq!(tx.input().as_ref(), &[0x56, 0x78]);
        assert_eq!(tx.access_list().unwrap().len(), 1);
        assert_eq!(tx.blob_versioned_hashes().unwrap().len(), 1);
        assert_eq!(tx.authorization_list().unwrap().len(), 1);
        assert_eq!(tx.access_list(), expected.access_list());
        assert_eq!(tx.blob_versioned_hashes(), expected.blob_versioned_hashes());
        assert_eq!(tx.authorization_list(), expected.authorization_list());
        assert_eq!(tx, expected);
        assert_eq!(serde_json::to_value(&tx).unwrap(), replacement);

        tx.fields_mut().clear();
        assert!(tx.input().is_empty());
        assert!(tx.access_list().is_none());
        assert!(tx.blob_versioned_hashes().is_none());
        assert!(tx.authorization_list().is_none());
    }

    #[test]
    fn test_serde_anytype() {
        let ty = AnyTxType(126);
        assert_eq!(serde_json::to_string(&ty).unwrap(), "\"0x7e\"");
    }

    #[test]
    fn test_serde_op_deposit() {
        let input = r#"{
            "blockHash": "0xef664d656f841b5ad6a2b527b963f1eb48b97d7889d742f6cbff6950388e24cd",
            "blockNumber": "0x73a78fd",
            "depositReceiptVersion": "0x1",
            "from": "0x36bde71c97b33cc4729cf772ae268934f7ab70b2",
            "gas": "0xc27a8",
            "gasPrice": "0x521",
            "hash": "0x0bf1845c5d7a82ec92365d5027f7310793d53004f3c86aa80965c67bf7e7dc80",
            "input": "0xd764ad0b000100000000000000000000000000000000000000000000000000000001cf5400000000000000000000000099c9fc46f92e8a1c0dec1b1747d010903e884be100000000000000000000000042000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a12000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000e40166a07a0000000000000000000000000994206dfe8de6ec6920ff4d779b0d950605fb53000000000000000000000000d533a949740bb3306d119cc777fa900ba034cd52000000000000000000000000ca74f404e0c7bfa35b13b511097df966d5a65597000000000000000000000000ca74f404e0c7bfa35b13b511097df966d5a65597000000000000000000000000000000000000000000000216614199391dbba2ba00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "mint": "0x0",
            "nonce": "0x74060",
            "r": "0x0",
            "s": "0x0",
            "sourceHash": "0x074adb22f2e6ed9bdd31c52eefc1f050e5db56eb85056450bccd79a6649520b3",
            "to": "0x4200000000000000000000000000000000000007",
            "transactionIndex": "0x1",
            "type": "0x7e",
            "v": "0x0",
            "value": "0x0"
        }"#;

        let tx: AnyRpcTransaction = serde_json::from_str(input).unwrap();

        let AnyTxEnvelope::Unknown(inner) = tx.inner.inner.inner().clone() else {
            panic!("expected unknown envelope");
        };

        assert_eq!(inner.inner.ty, AnyTxType(126));
        assert!(inner.inner.fields().contains_key("input"));
        assert!(inner.inner.fields().contains_key("mint"));
        assert!(inner.inner.fields().contains_key("sourceHash"));
        assert_eq!(inner.gas_limit(), 796584);
        assert_eq!(inner.gas_price(), Some(1313));
        assert_eq!(inner.nonce(), 475232);

        let roundrip_tx: AnyRpcTransaction =
            serde_json::from_str(&serde_json::to_string(&tx).unwrap()).unwrap();

        assert_eq!(tx, roundrip_tx);
    }

    /// <https://github.com/alloy-rs/alloy/issues/2842>
    #[test]
    fn gas_price_saturation() {
        let tx_json_with_saturate = serde_json::json!({
            "type": "0x78",
            "nonce": "0x1",
            // A gas price larger than u128::MAX.
            "gasPrice": "0x30783134626639633464372e3333333333333333333333333333333333333333",
            "gas": "0x5208",
            "to": "0x0000000000000000000000000000000000000000",
            "value": "0x0",
            "input": "0x",
            "from": "0x0000000000000000000000000000000000000000"
        });
        let tx: alloy_rpc_types_eth::Transaction<UnknownTypedTransaction> =
            serde_json::from_value(tx_json_with_saturate).unwrap();
        assert!(tx.inner.gas_price().is_none());
        assert_eq!(tx.effective_gas_price, Some(u128::MAX));

        let tx_json_no_saturate = serde_json::json!({
            "type": "0x78",
            "nonce": "0x1",
            // A normal gas price.
            "gasPrice": "0x3b9aca00",
            "gas": "0x5208",
            "to": "0x0000000000000000000000000000000000000000",
            "value": "0x0",
            "input": "0x",
            "from": "0x0000000000000000000000000000000000000000"
        });
        let tx: alloy_rpc_types_eth::Transaction<UnknownTypedTransaction> =
            serde_json::from_value(tx_json_no_saturate).unwrap();
        assert!(tx.inner.gas_price().is_some_and(|g| g == 1_000_000_000));
        assert!(tx.effective_gas_price.is_some_and(|g| g == 1_000_000_000));
    }
}
