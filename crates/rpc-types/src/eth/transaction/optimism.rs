//! Misc Optimism-specific types
use alloy_primitives::{B256, U128, U256, U64};
use serde::{Deserialize, Serialize};

use crate::other::OtherFields;

/// Optimism specific transaction fields
#[derive(Debug, Copy, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimismTransactionReceiptFields {
    /// Deposit nonce for deposit transactions post-regolith
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_nonce: Option<U64>,
    /// L1 fee for the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_fee: Option<U256>,
    /// L1 fee scalar for the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_fee_scalar: Option<U256>,
    /// L1 gas price for the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_gas_price: Option<U256>,
    /// L1 gas used for the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_gas_used: Option<U256>,
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
