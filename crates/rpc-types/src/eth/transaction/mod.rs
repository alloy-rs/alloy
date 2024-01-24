//! RPC types for transactions

use crate::eth::other::OtherFields;
pub use access_list::{AccessList, AccessListItem, AccessListWithGasUsed};
use alloy_primitives::{Address, Bytes, B256, U128, U256, U64};
pub use blob::BlobTransactionSidecar;
pub use common::TransactionInfo;
pub use optimism::OptimismTransactionReceiptFields;
pub use receipt::TransactionReceipt;
use serde::{Deserialize, Serialize};
pub use signature::{Parity, Signature};

mod access_list;
mod common;
pub mod kzg;
pub mod optimism;
mod receipt;
pub mod request;
mod signature;

mod blob;

/// Transaction object used in RPC
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Hash
    pub hash: B256,
    /// Nonce
    pub nonce: U64,
    /// Block hash
    pub block_hash: Option<B256>,
    /// Block number
    pub block_number: Option<U256>,
    /// Transaction Index
    pub transaction_index: Option<U256>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U128>,
    /// Gas amount
    pub gas: U256,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<U128>,
    /// The miner's tip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_blob_gas: Option<U128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    pub chain_id: Option<U64>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blob_versioned_hashes: Vec<B256>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_list: Option<Vec<AccessListItem>>,
    /// EIP2718
    ///
    /// Transaction type, Some(2) for EIP-1559 transaction,
    /// Some(1) for AccessList transaction, None for Legacy
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U64>,

    /// Arbitrary extra fields.
    ///
    /// This captures fields that are not native to ethereum but included in ethereum adjacent networks, for example fields the [optimism `eth_getTransactionByHash` request](https://docs.alchemy.com/alchemy/apis/optimism/eth-gettransactionbyhash) returns additional fields that this type will capture
    #[serde(flatten)]
    pub other: OtherFields,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eth::transaction::signature::Parity;

    #[test]
    fn serde_transaction() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: U64::from(2),
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(U256::from(4)),
            transaction_index: Some(U256::from(5)),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(8),
            gas_price: Some(U128::from(9)),
            gas: U256::from(10),
            input: Bytes::from(vec![11, 12, 13]),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: None,
            }),
            chain_id: Some(U64::from(17)),
            blob_versioned_hashes: vec![],
            access_list: None,
            transaction_type: Some(U64::from(20)),
            max_fee_per_gas: Some(U128::from(21)),
            max_priority_fee_per_gas: Some(U128::from(22)),
            max_fee_per_blob_gas: None,
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0x8","gasPrice":"0x9","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x11","type":"0x14"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    fn serde_transaction_with_parity_bit() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: U64::from(2),
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(U256::from(4)),
            transaction_index: Some(U256::from(5)),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(8),
            gas_price: Some(U128::from(9)),
            gas: U256::from(10),
            input: Bytes::from(vec![11, 12, 13]),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Some(Parity(true)),
            }),
            chain_id: Some(U64::from(17)),
            blob_versioned_hashes: vec![],
            access_list: None,
            transaction_type: Some(U64::from(20)),
            max_fee_per_gas: Some(U128::from(21)),
            max_priority_fee_per_gas: Some(U128::from(22)),
            max_fee_per_blob_gas: None,
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0x8","gasPrice":"0x9","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x11","type":"0x14"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }
}
