//! Otterscan specific types for RPC responses.
//!
//! <https://www.quicknode.com/docs/ethereum/ots_getBlockTransactions>
//! <https://github.com/otterscan/otterscan/blob/develop/docs/custom-jsonrpc.md>

#![allow(missing_docs)]

use alloy_primitives::{Address, Bloom, Bytes, U256};
use alloy_rpc_types::{Block, Rich, Transaction, TransactionReceipt};
use serde::{Deserialize, Serialize};

/// Operation type enum for `InternalOperation` struct
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_copy_implementations)]
pub enum OperationType {
    /// Operation Transfer
    OpTransfer = 0,
    /// Operation Contract self destruct
    OpSelfDestruct = 1,
    /// Operation Create
    OpCreate = 2,
    /// Operation Create2
    OpCreate2 = 3,
}

/// Custom struct for otterscan `getInternalOperations` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalOperation {
    pub r#type: OperationType,
    pub from: Address,
    pub to: Address,
    pub value: U256,
}

/// Custom struct for otterscan `traceTransaction` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEntry {
    pub r#type: String,
    pub depth: u32,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub input: Bytes,
}

/// Internal issuance struct for `BlockDetails` struct
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_copy_implementations)]
#[serde(rename_all = "camelCase")]
pub struct InternalIssuance {
    pub block_reward: U256,
    pub uncle_reward: U256,
    pub issuance: U256,
}

/// Custom `Block` struct that includes transaction count for Otterscan responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsBlock {
    #[serde(flatten)]
    pub block: Block,
    pub transaction_count: usize,
}

impl From<Block> for OtsBlock {
    fn from(block: Block) -> Self {
        Self { transaction_count: block.transactions.len(), block }
    }
}

/// Custom struct for otterscan `getBlockDetails` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDetails {
    pub block: OtsBlock,
    pub issuance: InternalIssuance,
    pub total_fees: U256,
}

impl From<Rich<Block>> for BlockDetails {
    fn from(rich_block: Rich<Block>) -> Self {
        Self {
            block: rich_block.inner.into(),
            issuance: Default::default(),
            total_fees: U256::default(),
        }
    }
}

/// Custom transaction receipt struct for otterscan `OtsBlockTransactions` struct
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsTransactionReceipt {
    /// The transaction receipt.
    ///
    /// Note: the otterscan API sets all log fields to null.
    #[serde(flatten)]
    pub receipt: TransactionReceipt<OtsReceipt>,
    #[serde(default, with = "alloy_serde::u64_opt_via_ruint")]
    pub timestamp: Option<u64>,
}

/// The receipt of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsReceipt {
    /// If transaction is executed successfully.
    ///
    /// This is the `statusCode`
    #[serde(with = "alloy_serde::quantity_bool")]
    pub status: bool,
    /// Gas used
    #[serde(with = "alloy_serde::u64_via_ruint")]
    pub cumulative_gas_used: u64,
    /// Log send from contracts.
    ///
    /// Note: this is set to null,
    pub logs: Option<Vec<alloy_primitives::Log>>,
    /// The bloom filter.
    ///
    /// Note: this is set to null
    pub logs_bloom: Option<Bloom>,
    /// The transaction type.
    #[serde(with = "alloy_serde::num::u8_via_ruint")]
    pub r#type: u8,
}

/// Custom struct for otterscan `getBlockTransactions` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtsBlockTransactions {
    pub fullblock: OtsBlock,
    pub receipts: Vec<OtsTransactionReceipt>,
}

/// Custom struct for otterscan `searchTransactionsAfter`and `searchTransactionsBefore` RPC
/// responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsWithReceipts {
    pub txs: Vec<Transaction>,
    pub receipts: Vec<OtsTransactionReceipt>,
    pub first_page: bool,
    pub last_page: bool,
}

/// Custom struct for otterscan `getContractCreator` RPC responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCreator {
    pub tx: Transaction,
    pub creator: Address,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otterscan_receipt() {
        let s = r#"{
      "blockHash": "0xf05aa8b73b005314684595adcff8e6149917b3239b6316247ce5e88eba9fd3f5",
      "blockNumber": "0x1106fe7",
      "contractAddress": null,
      "cumulativeGasUsed": "0x95fac3",
      "effectiveGasPrice": "0x2e9f0055d",
      "from": "0x793abeea78d94c14b884a56788f549836a35db65",
      "gasUsed": "0x14427",
      "logs": null,
      "logsBloom": null,
      "status": "0x1",
      "to": "0x06450dee7fd2fb8e39061434babcfc05599a6fb8",
      "transactionHash": "0xd3cead022cbb5d6d18091f8b375e3a3896ec139e986144b9448290d55837275a",
      "transactionIndex": "0x90",
      "type": "0x2"
    }"#;

        let _receipt: OtsTransactionReceipt = serde_json::from_str(s).unwrap();
    }
}
