//! Alloy basic Transaction Request type.
use crate::{eth::transaction::AccessList, other::OtherFields, BlobTransactionSidecar};
use alloy_primitives::{Address, Bytes, B256, U128, U256, U64, U8};
use serde::{Deserialize, Serialize};

/// Represents _all_ transaction requests received from RPC
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TransactionRequest {
    /// from address
    pub from: Option<Address>,
    /// to address
    pub to: Option<Address>,
    /// legacy, gas Price
    #[serde(default)]
    pub gas_price: Option<U128>,
    /// max base fee per gas sender is willing to pay
    #[serde(default)]
    pub max_fee_per_gas: Option<U128>,
    /// miner tip
    #[serde(default)]
    pub max_priority_fee_per_gas: Option<U128>,
    /// gas
    pub gas: Option<U256>,
    /// value of th tx in wei
    pub value: Option<U256>,
    /// Any additional data sent
    #[serde(alias = "input")]
    pub data: Option<Bytes>,
    /// Transaction nonce
    pub nonce: Option<U64>,
    /// warm storage access pre-payment
    #[serde(default)]
    pub access_list: Option<AccessList>,
    /// EIP-2718 type
    #[serde(rename = "type")]
    pub transaction_type: Option<U8>,
    /// sidecar for EIP-4844 transactions
    pub sidecar: Option<BlobTransactionSidecar>,
    /// Support for arbitrary additional fields.
    #[serde(flatten)]
    pub other: OtherFields,
}

// == impl TransactionRequest ==

impl TransactionRequest {
    /// Sets the gas limit for the transaction.
    pub fn gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas = Some(U256::from(gas_limit));
        self
    }

    /// Sets the nonce for the transaction.
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(U64::from(nonce));
        self
    }

    /// Sets the maximum fee per gas for the transaction.
    pub fn max_fee_per_gas(mut self, max_fee_per_gas: u128) -> Self {
        self.max_fee_per_gas = Some(U128::from(max_fee_per_gas));
        self
    }

    /// Sets the maximum priority fee per gas for the transaction.
    pub fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: u128) -> Self {
        self.max_priority_fee_per_gas = Some(U128::from(max_priority_fee_per_gas));
        self
    }

    /// Sets the recipient address for the transaction.
    pub const fn to(mut self, to: Address) -> Self {
        self.to = Some(to);
        self
    }
    /// Sets the value (amount) for the transaction.

    pub fn value(mut self, value: u128) -> Self {
        self.value = Some(U256::from(value));
        self
    }

    /// Sets the access list for the transaction.
    pub fn access_list(mut self, access_list: AccessList) -> Self {
        self.access_list = Some(access_list);
        self
    }

    /// Sets the input data for the transaction.
    pub fn input(mut self, input: Bytes) -> Self {
        self.data = Some(input);
        self
    }

    /// Sets the transactions type for the transactions.
    pub fn transaction_type(mut self, transaction_type: u8) -> Self {
        self.transaction_type = Some(U8::from(transaction_type));
        self
    }
}

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

impl From<OptimismTransactionFields> for OtherFields {
    fn from(value: OptimismTransactionFields) -> Self {
        serde_json::to_value(value).unwrap().try_into().unwrap()
    }
}
