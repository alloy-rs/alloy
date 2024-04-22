//! Types for opcode tracing.

use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

/// Opcode gas usage for a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockOpcodeGas {
    /// The block hash
    pub block_hash: B256,
    /// The block number
    pub block_number: u64,
    /// All executed transactions in the block in the order they were executed, with their opcode
    /// gas usage.
    pub transactions: Vec<TransactionOpcodeGas>,
}

/// Opcode gas usage for a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionOpcodeGas {
    /// The transaction hash
    pub transaction_hash: B256,
    /// The gas used by each opcode in the transaction
    pub opcode_gas: Vec<OpcodeGas>,
}

/// Gas information for a single opcode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpcodeGas {
    /// The name of the opcode
    pub opcode: String,
    /// How many times the opcode was executed
    pub count: u64,
    /// Combined gas used by all instances of the opcode
    ///
    /// For opcodes with constant gas costs, this is the constant opcode gas cost times the count.
    pub gas_used: u64,
}
