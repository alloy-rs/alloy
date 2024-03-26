use alloy_consensus::ReceiptEnvelope;
use alloy_primitives::{Address, B256, U64, U8};
use serde::{Deserialize, Serialize};

/// Transaction receipt
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    /// The receipt envelope, which contains the consensus receipt data..
    #[serde(flatten)]
    pub inner: ReceiptEnvelope,

    /// Transaction Hash.
    pub transaction_hash: B256,
    /// Index within the block.
    #[serde(with = "alloy_serde::u64_hex_or_decimal")]
    pub transaction_index: u64,
    /// Hash of the block this transaction was included within.
    pub block_hash: Option<B256>,
    /// Number of the block this transaction was included within.

    #[serde(with = "alloy_serde::u64_hex_opt")]
    pub block_number: Option<u64>,
    /// Gas used by this transaction alone.
    #[serde(with = "alloy_serde::u64_hex_opt")]
    pub gas_used: Option<u64>,
    /// The price paid post-execution by the transaction (i.e. base fee + priority fee). Both
    /// fields in 1559-style transactions are maximums (max fee + max priority fee), the amount
    /// that's actually paid by users can only be determined post-execution
    #[serde(with = "alloy_serde::u64_hex_or_decimal")]
    pub effective_gas_price: u64,
    /// Blob gas used by the eip-4844 transaction
    ///
    /// This is None for non eip-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none", with = "alloy_serde::u64_hex_opt", default)]
    pub blob_gas_used: Option<u64>,
    /// The price paid by the eip-4844 transaction per blob gas.
    #[serde(skip_serializing_if = "Option::is_none", with = "alloy_serde::u64_hex_opt", default)]
    pub blob_gas_price: Option<u64>,
    /// Address of the sender
    pub from: Address,
    /// Address of the receiver. None when its a contract creation transaction.
    pub to: Option<Address>,
    /// Contract address created, or None if not a deployment.
    pub contract_address: Option<Address>,
    /// The post-transaction stateroot (pre Byzantium)
    ///
    /// EIP98 makes this optional field, if it's missing then skip serializing it
    #[serde(skip_serializing_if = "Option::is_none", rename = "root")]
    pub state_root: Option<B256>,
    /// Status: either 1 (success) or 0 (failure). Only present after activation of EIP-658
    #[serde(skip_serializing_if = "Option::is_none", rename = "status")]
    pub status_code: Option<U64>,
    /// EIP-2718 Transaction type.
    ///
    /// For legacy transactions this returns `0`. For EIP-2718 transactions this returns the type.
    #[serde(rename = "type")]
    pub transaction_type: U8,
}

impl AsRef<ReceiptEnvelope> for TransactionReceipt {
    fn as_ref(&self) -> &ReceiptEnvelope {
        &self.inner
    }
}

impl TransactionReceipt {
    /// Calculates the address that will be created by the transaction, if any.
    ///
    /// Returns `None` if the transaction is not a contract creation (the `to` field is set), or if
    /// the `from` field is not set.
    pub fn calculate_create_address(&self, nonce: u64) -> Option<Address> {
        if self.to.is_some() {
            return None;
        }
        Some(self.from.create(nonce))
    }
}
