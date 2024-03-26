use crate::{other::OtherFields, ConversionError, Log};
use alloy_consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom, TxType};
use alloy_primitives::{Address, Bloom, Log as PrimitivesLog, LogData, B256, U128, U256, U64, U8};
use serde::{Deserialize, Serialize};

/// Transaction receipt
#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    /// Transaction Hash.
    pub transaction_hash: B256,
    /// Index within the block.
    pub transaction_index: U64,
    /// Hash of the block this transaction was included within.
    pub block_hash: Option<B256>,
    /// Number of the block this transaction was included within.
    pub block_number: Option<U256>,
    /// Cumulative gas used within the block after this was executed.
    pub cumulative_gas_used: U256,
    /// Gas used by this transaction alone.
    pub gas_used: Option<U256>,
    /// The price paid post-execution by the transaction (i.e. base fee + priority fee). Both
    /// fields in 1559-style transactions are maximums (max fee + max priority fee), the amount
    /// that's actually paid by users can only be determined post-execution
    pub effective_gas_price: U128,
    /// Blob gas used by the eip-4844 transaction
    ///
    /// This is None for non eip-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_gas_used: Option<U128>,
    /// The price paid by the eip-4844 transaction per blob gas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_gas_price: Option<U128>,
    /// Address of the sender
    pub from: Address,
    /// Address of the receiver. None when its a contract creation transaction.
    pub to: Option<Address>,
    /// Contract address created, or None if not a deployment.
    pub contract_address: Option<Address>,
    /// Logs emitted by this transaction.
    pub logs: Vec<Log>,
    /// Logs bloom
    pub logs_bloom: Bloom,
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
    /// Arbitrary extra fields.
    #[serde(flatten)]
    pub other: OtherFields,
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

    /// Returns `true` if the transaction was successful.
    /// A transaction is considered successful if the status code is `1`.
    fn success(&self) -> bool {
        match &self.status_code {
            Some(status) => status == &U64::from(1),
            None => false,
        }
    }

    /// Returns the logs emitted by the transaction.
    /// Converts the logs from the RPC type to the internal type.
    fn logs(&self) -> Vec<PrimitivesLog<LogData>> {
        let mut logs = Vec::new();
        for log in &self.logs {
            let rpc_log: Log = log.clone();
            let log_data = LogData::try_from(rpc_log).unwrap_or_default();
            let result = PrimitivesLog { address: log.address, data: log_data };
            logs.push(result);
        }

        logs
    }
}

impl TryFrom<TransactionReceipt> for ReceiptWithBloom {
    type Error = ConversionError;

    fn try_from(tx_receipt: TransactionReceipt) -> Result<Self, Self::Error> {
        let receipt_with_bloom = ReceiptWithBloom {
            receipt: Receipt {
                success: tx_receipt.success(),
                cumulative_gas_used: tx_receipt.cumulative_gas_used.to::<u64>(),
                logs: tx_receipt.logs(),
            },
            bloom: tx_receipt.logs_bloom,
        };
        Ok(receipt_with_bloom)
    }
}

impl TryFrom<TransactionReceipt> for ReceiptEnvelope {
    type Error = ConversionError;

    fn try_from(tx_receipt: TransactionReceipt) -> Result<Self, Self::Error> {
        match tx_receipt.transaction_type.to::<u8>().try_into()? {
            TxType::Legacy => Ok(ReceiptEnvelope::Legacy(tx_receipt.try_into()?)),
            TxType::Eip2930 => Ok(ReceiptEnvelope::Eip2930(tx_receipt.try_into()?)),
            TxType::Eip1559 => Ok(ReceiptEnvelope::Eip1559(tx_receipt.try_into()?)),
            TxType::Eip4844 => Ok(ReceiptEnvelope::Eip4844(tx_receipt.try_into()?)),
        }
    }
}
