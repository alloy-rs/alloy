//! Misc Optimism-specific types.

use alloy_primitives::B256;
use alloy_serde::OtherFields;
use serde::{Deserialize, Serialize};

/// Optimism specific transaction fields: <https://github.com/ethereum-optimism/op-geth/blob/641e996a2dcf1f81bac9416cb6124f86a69f1de7/internal/ethapi/api.go#L1479-L1479>
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[doc(alias = "OptimismTxFields")]
#[serde(rename_all = "camelCase")]
pub struct OptimismTransactionFields {
    /// Hash that uniquely identifies the source of the deposit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<B256>,
    /// The ETH value to mint on L2
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub mint: Option<u128>,
    /// Field indicating whether the transaction is a system transaction, and therefore
    /// exempt from the L2 gas limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[doc(alias = "is_system_transaction")]
    pub is_system_tx: Option<bool>,
    /// Deposit receipt version for Optimism deposit transactions, post-Canyon only
    ///
    ///
    /// The deposit receipt version was introduced in Canyon to indicate an update to how
    /// receipt hashes should be computed when set. The state transition process
    /// ensures this is only set for post-Canyon deposit transactions.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub deposit_receipt_version: Option<u64>,
}

/// Additional fields for Optimism transaction receipts: <https://github.com/ethereum-optimism/op-geth/blob/f2e69450c6eec9c35d56af91389a1c47737206ca/core/types/receipt.go#L87-L87>
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc(alias = "OptimismTxReceiptFields")]
pub struct OptimismTransactionReceiptFields {
    /// Deposit nonce for deposit transactions post-regolith
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub deposit_nonce: Option<u64>,
    /// Deposit receipt version for deposit transactions post-canyon
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub deposit_receipt_version: Option<u64>,
    /// Present from pre-bedrock. L1 Basefee after Bedrock
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub l1_gas_price: Option<u128>,
    /// Always null prior to the Ecotone hardfork.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub l1_blob_base_fee: Option<u128>,
    /// Present from pre-bedrock, deprecated as of Fjord.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub l1_gas_used: Option<u128>,
    /// Present from pre-bedrock. L1 fee for the transaction
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub l1_fee: Option<u128>,
    /// Present from pre-bedrock to Ecotone. Nil after Ecotone
    #[serde(default, skip_serializing_if = "Option::is_none", with = "l1_fee_scalar_serde")]
    pub l1_fee_scalar: Option<f64>,
    /// Always null prior to the Ecotone hardfork.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub l1_base_fee_scalar: Option<u128>,
    /// Always null prior to the Ecotone hardfork
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub l1_blob_base_fee_scalar: Option<u128>,
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

    #[test]
    fn deserialize_op_receipt() {
        let s = r#"{
    "blockHash": "0x70a8a64a0f8b141718f60e49c30f027cb9e4f91753d5f13a48d8e1ad263c08bf",
    "blockNumber": "0x1185e55",
    "contractAddress": null,
    "cumulativeGasUsed": "0xc74f5e",
    "effectiveGasPrice": "0x31b41b",
    "from": "0x889ebdac39408782b5165c5185c1a769b4dd3ce6",
    "gasUsed": "0x5208",
    "l1BaseFeeScalar": "0x8dd",
    "l1BlobBaseFee": "0x1",
    "l1BlobBaseFeeScalar": "0x101c12",
    "l1Fee": "0x125f723f3",
    "l1GasPrice": "0x50f928b4",
    "l1GasUsed": "0x640",
    "logs": [
    ],
    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "status": "0x1",
    "to": "0x7449061f45d7b39b3b80b4159286cd8682f60a3c",
    "transactionHash": "0xca564948e3e825f65731424da063240eec34ba921dd117ac5d06b8c2e0b2d962",
    "transactionIndex": "0x3e",
    "type": "0x2"
}
"#;
        let _receipt = serde_json::from_str::<OptimismTransactionReceiptFields>(s).unwrap();
    }
}
