//! Otterscan specific types for RPC responses.
//!
//! <https://www.quicknode.com/docs/ethereum/ots_getBlockTransactions>
//! <https://github.com/otterscan/otterscan/blob/develop/docs/custom-jsonrpc.md>

use alloy_primitives::{Address, Bloom, Bytes, U256};
use alloy_rpc_types::{Block, Rich, Transaction, TransactionReceipt};
use serde::{Deserialize, Serialize};

/// Operation type enum for `InternalOperation` struct
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalOperation {
    /// The type of the internal operation.
    pub r#type: OperationType,
    /// The address from which the operation originated.
    pub from: Address,
    /// The address to which the operation is directed.
    pub to: Address,
    /// The value transferred in the operation.
    pub value: U256,
}

/// Custom struct for otterscan `traceTransaction` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEntry {
    /// The type of trace entry.
    pub r#type: String,
    /// The depth of the trace entry.
    pub depth: u32,
    /// The address from which the trace originated.
    pub from: Address,
    /// The address to which the trace is directed.
    pub to: Address,
    /// The value transferred in the trace.
    pub value: U256,
    /// The input data for the trace.
    pub input: Bytes,
}

/// Internal issuance struct for `BlockDetails` struct
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_copy_implementations)]
#[serde(rename_all = "camelCase")]
pub struct InternalIssuance {
    /// The block reward issued.
    pub block_reward: U256,
    /// The uncle reward issued.
    pub uncle_reward: U256,
    /// The total issuance amount.
    pub issuance: U256,
}

/// Custom `Block` struct that includes transaction count for Otterscan responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsBlock {
    /// The block information.
    #[serde(flatten)]
    pub block: Block,
    /// The number of transactions in the block.
    #[doc(alias = "tx_count")]
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
    /// The block information with transaction count.
    pub block: OtsBlock,
    /// The issuance information for the block.
    pub issuance: InternalIssuance,
    /// The total fees for the block.
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
    /// The timestamp of the transaction.
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub timestamp: Option<u64>,
}

/// The receipt of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsReceipt {
    /// If the transaction is executed successfully.
    ///
    /// This is the `statusCode`
    #[serde(with = "alloy_serde::quantity")]
    pub status: bool,
    /// The cumulative gas used.
    #[serde(with = "alloy_serde::quantity")]
    pub cumulative_gas_used: u64,
    /// The logs sent from contracts.
    ///
    /// Note: this is set to null.
    pub logs: Option<Vec<alloy_primitives::Log>>,
    /// The bloom filter.
    ///
    /// Note: this is set to null.
    pub logs_bloom: Option<Bloom>,
    /// The transaction type.
    #[serde(with = "alloy_serde::quantity")]
    pub r#type: u8,
}

/// Custom struct for otterscan `getBlockTransactions` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtsBlockTransactions {
    /// The full block information with transaction count.
    pub fullblock: OtsBlock,
    /// The list of transaction receipts.
    pub receipts: Vec<OtsTransactionReceipt>,
}

/// Custom struct for otterscan `searchTransactionsAfter` and `searchTransactionsBefore` RPC
/// responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc(alias = "TxWithReceipts")]
pub struct TransactionsWithReceipts {
    /// The list of transactions.
    #[doc(alias = "transactions")]
    pub txs: Vec<Transaction>,
    /// The list of transaction receipts.
    pub receipts: Vec<OtsTransactionReceipt>,
    /// Indicates if this is the first page of results.
    pub first_page: bool,
    /// Indicates if this is the last page of results.
    pub last_page: bool,
}

/// Custom struct for otterscan `getContractCreator` RPC responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCreator {
    /// The transaction used to create the contract.
    #[doc(alias = "transaction")]
    pub tx: Transaction,
    /// The address of the contract creator.
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
