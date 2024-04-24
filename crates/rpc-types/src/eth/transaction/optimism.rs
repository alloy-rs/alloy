//! Misc Optimism-specific types.

use crate::other::OtherFields;
use alloy_primitives::{B256, U128, U64};
use serde::{Deserialize, Serialize};

/// Optimism specific transaction fields
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptimismTransactionFields {
    /// Hash that uniquely identifies the source of the deposit.
    #[serde(rename = "sourceHash", skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<B256>,
    /// The ETH value to mint on L2
    #[serde(rename = "mint", skip_serializing_if = "Option::is_none")]
    pub mint: Option<U128>,
    /// Field indicating whether the transaction is a system transaction, and therefore
    /// exempt from the L2 gas limit.
    #[serde(rename = "isSystemTx", skip_serializing_if = "Option::is_none")]
    pub is_system_tx: Option<bool>,
}

/// Additional fields for Optimism transaction receipts
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimismTransactionReceiptFields {
    /// Deposit nonce for deposit transactions post-regolith
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_nonce: Option<U64>,
    /// Deposit receipt version for deposit transactions post-canyon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_receipt_version: Option<U64>,
    /// L1 fee for the transaction
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub l1_fee: Option<u128>,
    /// L1 fee scalar for the transaction
    #[serde(default, skip_serializing_if = "Option::is_none", with = "l1_fee_scalar_serde")]
    pub l1_fee_scalar: Option<f64>,
    /// L1 gas price for the transaction
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub l1_gas_price: Option<u128>,
    /// L1 gas used for the transaction
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub l1_gas_used: Option<u128>,
}

impl From<OptimismTransactionFields> for OtherFields {
    fn from(value: OptimismTransactionFields) -> Self {
        serde_json::to_value(value).unwrap().try_into().unwrap()
    }
}

impl From<OptimismTransactionReceiptFields> for OtherFields {
    fn from(value: OptimismTransactionReceiptFields) -> Self {
        serde_json::to_value(value).unwrap().try_into().unwrap()
    }
}

/// Serialize/Deserialize l1FeeScalar to/from string
mod l1_fee_scalar_serde {
    use serde::{de, Deserialize};

    pub(super) fn serialize<S>(value: &Option<f64>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(v) = value {
            return s.serialize_str(&v.to_string());
        }
        s.serialize_none()
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        if let Some(s) = s {
            return Ok(Some(s.parse::<f64>().map_err(de::Error::custom)?));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn serialize_empty_optimism_transaction_receipt_fields_struct() {
        let op_fields = OptimismTransactionReceiptFields::default();

        let json = serde_json::to_value(op_fields).unwrap();
        assert_eq!(json, json!({}));
    }

    #[test]
    fn serialize_l1_fee_scalar() {
        let op_fields = OptimismTransactionReceiptFields {
            l1_fee_scalar: Some(0.678),
            ..OptimismTransactionReceiptFields::default()
        };

        let json = serde_json::to_value(op_fields).unwrap();

        assert_eq!(json["l1FeeScalar"], serde_json::Value::String("0.678".to_string()));
    }

    #[test]
    fn deserialize_l1_fee_scalar() {
        let json = json!({
            "l1FeeScalar": "0.678"
        });

        let op_fields: OptimismTransactionReceiptFields = serde_json::from_value(json).unwrap();
        assert_eq!(op_fields.l1_fee_scalar, Some(0.678f64));

        let json = json!({
            "l1FeeScalar": Value::Null
        });

        let op_fields: OptimismTransactionReceiptFields = serde_json::from_value(json).unwrap();
        assert_eq!(op_fields.l1_fee_scalar, None);

        let json = json!({});

        let op_fields: OptimismTransactionReceiptFields = serde_json::from_value(json).unwrap();
        assert_eq!(op_fields.l1_fee_scalar, None);
    }
}
