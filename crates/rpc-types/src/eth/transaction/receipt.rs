use crate::{other::OtherFields, ConversionError, Log};
use alloy_consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom, TxType};
use alloy_primitives::{Address, Bloom, B256, U128, U256, U64, U8};
use itertools::Itertools;
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
            Some(status) => status.to::<u64>() == 1,
            None => false,
        }
    }

    /// Returns an iterator over the logs for prmitives type conversion.
    fn logs_iter(
        &self,
    ) -> impl Iterator<Item = alloy_primitives::Log<alloy_primitives::LogData>> + '_ {
        self.logs.iter().map(|log| {
            alloy_primitives::Log::new_unchecked(log.address, log.topics.clone(), log.data.clone())
        })
    }
}

impl TryFrom<TransactionReceipt> for ReceiptWithBloom {
    type Error = ConversionError;

    fn try_from(tx_receipt: TransactionReceipt) -> Result<Self, Self::Error> {
        let receipt_with_bloom = ReceiptWithBloom {
            receipt: Receipt {
                success: tx_receipt.success(),
                cumulative_gas_used: tx_receipt.cumulative_gas_used.to::<u64>(),
                logs: tx_receipt.logs_iter().collect_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_transaction_receipt_to_receipts_envelope_4844() {
        // cast rpc eth_getTransactionReceipt
        // 0x9c1fbda4f649ac806ab0faefbe94e1a60282eb374ead6aa01bac042f52b28a8c --rpc-url mainnet
        let rpc_tx_receipt = r#"{"blobGasPrice":"0x1","blobGasUsed":"0x20000","blockHash":"0xa2917e0758c98640d868182838c93bb12f0d07b6b17efe6b62d9df42c7643791","blockNumber":"0x1286d1d","contractAddress":null,"cumulativeGasUsed":"0x56b224","effectiveGasPrice":"0xd364c1438","from":"0x40c35d4faf69234986cb599890c2d2ef546074a9","gasUsed":"0x5208","logs":[],"logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","status":"0x1","to":"0x0000000000000000000000000000000000000000","transactionHash":"0x9c1fbda4f649ac806ab0faefbe94e1a60282eb374ead6aa01bac042f52b28a8c","transactionIndex":"0x46","type":"0x3"}"#;
        let transaction_receipt: TransactionReceipt = serde_json::from_str(rpc_tx_receipt).unwrap();

        let receipt_with_bloom = ReceiptWithBloom {
            receipt: Receipt { success: true, cumulative_gas_used: 0x56b224, logs: vec![] },
            bloom: Bloom::default(),
        };

        let receipt_envelope: ReceiptEnvelope = transaction_receipt.try_into().unwrap();

        assert_eq!(receipt_envelope, ReceiptEnvelope::Eip4844(receipt_with_bloom));
    }

    #[test]
    fn try_from_transaction_receipt_to_receipts_envelope_1559() {
        // cast rpc eth_getTransactionReceipt
        // 0xd271efca8906538124cca4213bc61aa3def380cf5e3b068b3215c09d87219c99 --rpc-url mainnet
        let rpc_tx_receipt = r#"{"blockHash":"0x851a0e708a669d9f9838c251b72d0b616b7f38c3ad38fa20a23c1144791bbdd6","blockNumber":"0x129cc66","contractAddress":null,"cumulativeGasUsed":"0x71a71f","effectiveGasPrice":"0x465e36461","from":"0xcb96aca8719987d15aecd066b7a1ad5d4d92fdd3","gasUsed":"0x74d8","logs":[{"address":"0x06450dee7fd2fb8e39061434babcfc05599a6fb8","topics":["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef","0x000000000000000000000000cb96aca8719987d15aecd066b7a1ad5d4d92fdd3","0x00000000000000000000000085f7459e29d5626b5d2f16c00a97e0c48ce1654e"],"data":"0x000000000000000000000000000000000000000000198b0b16f55ce58edc0000","blockNumber":"0x129cc66","transactionHash":"0xd271efca8906538124cca4213bc61aa3def380cf5e3b068b3215c09d87219c99","transactionIndex":"0x7a","blockHash":"0x851a0e708a669d9f9838c251b72d0b616b7f38c3ad38fa20a23c1144791bbdd6","logIndex":"0xa9","removed":false}],"logsBloom":"0x0000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000a00010000c000000000008400010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000","status":"0x1","to":"0x06450dee7fd2fb8e39061434babcfc05599a6fb8","transactionHash":"0xd271efca8906538124cca4213bc61aa3def380cf5e3b068b3215c09d87219c99","transactionIndex":"0x7a","type":"0x2"}"#;
        let transaction_receipt: TransactionReceipt = serde_json::from_str(rpc_tx_receipt).unwrap();

        let receipt_envelope: ReceiptEnvelope = transaction_receipt.try_into().unwrap();

        assert_eq!(receipt_envelope.tx_type(), TxType::Eip1559);
    }
}
