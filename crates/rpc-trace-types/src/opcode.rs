//! Types for opcode tracing.

use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

/// Opcode gas usage for a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockOpcodeGas {
    /// The block hash
    pub block_hash: B256,
    /// The block number
    pub block_number: u64,
    /// The gas used by each opcode in the transaction
    pub transactions: Vec<TransactionOpcodeGas>,
}

/// Opcode gas usage for a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionOpcodeGas {
    /// The gas used by each opcode in the transaction
    pub opcode_gas: Vec<OpcodeGas>,
    /// The transaction hash
    pub transaction_hash: Vec<OpcodeGas>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpcodeGas {
    /// The name of the opcode
    pub opcode: String,
    /// How many times the opcode was executed
    pub count: u64,
    /// Combined gas used by all instances of the opcode
    ///
    /// For dynamic gas costs, this is the sum of all gas used by the opcode.
    /// For constant gas costs, this is the gas cost times the count.
    pub gas_used: u64,
}
