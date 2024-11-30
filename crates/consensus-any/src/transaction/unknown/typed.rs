use std::sync::OnceLock;

use crate::AnyTxType;
use alloy_eips::{eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, B256, U128, U256, U64};
use alloy_serde::OtherFields;

/// Memoization for deserialization of [`UnknownTxEnvelope`],
/// [`UnknownTypedTransaction`] [`AnyTxEnvelope`], [`AnyTypedTransaction`].
/// Setting these manually is discouraged, however the fields are left public
/// for power users :)
///
/// [`AnyTxEnvelope`]: crate::AnyTxEnvelope
/// [`AnyTypedTransaction`]: crate::AnyTypedTransaction
/// [`UnknownTxEnvelope`]: crate::UnknownTxEnvelope
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(unnameable_types)]
pub struct DeserMemo {
    pub input: OnceLock<Bytes>,
    pub access_list: OnceLock<AccessList>,
    pub blob_versioned_hashes: OnceLock<Vec<B256>>,
    pub authorization_list: OnceLock<Vec<SignedAuthorization>>,
}

/// A typed transaction of an unknown Network
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "UnknownTypedTx")]
pub struct UnknownTypedTransaction {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    /// Transaction type.
    pub ty: AnyTxType,

    /// Additional fields.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub fields: OtherFields,

    /// Memoization for deserialization.
    #[cfg_attr(feature = "serde", serde(skip, default))]
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
        self.gas_price().or(self.max_priority_fee_per_gas()).unwrap_or_default()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        if let Some(gas_price) = self.gas_price() {
            return gas_price;
        }

        base_fee.map_or(self.max_fee_per_gas(), |base_fee| {
            // if the tip is greater than the max priority fee per gas, set it to the max
            // priority fee per gas + base fee
            let max_fee = self.max_fee_per_gas();
            if max_fee == 0 {
                return 0;
            }
            let Some(max_prio_fee) = self.max_priority_fee_per_gas() else { return max_fee };
            let tip = max_fee.saturating_sub(base_fee as u128);
            if tip > max_prio_fee {
                max_prio_fee + base_fee as u128
            } else {
                // otherwise return the max fee per gas
                max_fee
            }
        })
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
        self.fields.get("to").map_or(true, |v| v.is_null())
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
    fn ty(&self) -> u8 {
        self.ty.0
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
