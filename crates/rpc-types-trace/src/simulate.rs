use alloy_primitives::{Bytes, Log, B256, U64};
use alloy_rpc_types_eth::{state::StateOverride, BlockOverrides, Header, TransactionRequest};
use serde::{Deserialize, Serialize};

/// Represents a batch of calls to be simulated sequentially within a block.
/// This struct includes block and state overrides as well as the transaction requests to be
/// executed.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimBlock {
    /// Modifications to the default block characteristics.
    pub block_overrides: BlockOverrides,
    /// State modifications to apply before executing the transactions.
    pub state_overrides: StateOverride,
    /// A vector of transactions to be simulated.
    pub calls: Vec<TransactionRequest>,
}

/// Captures the outcome of a transaction simulation.
/// It includes the return value, logs produced, gas used, and the status of the transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimCallResult {
    /// The raw bytes returned by the transaction.
    pub return_value: Bytes,
    /// Logs generated during the execution of the transaction.
    pub logs: Vec<Log>,
    /// The amount of gas used by the transaction.
    pub gas_used: U64,
    /// The final status of the transaction, typically indicating success or failure.
    pub status: U64,
    //what we should use here ?
    //Error       *callError     `json:"error,omitempty"`
}

/// Simulation options for executing multiple blocks and transactions.
/// This struct configures how simulations are executed, including whether to trace token transfers,
/// validate transaction sequences, and whether to return full transaction objects.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimOpts {
    /// A vector of simulated blocks each containing state and transaction overrides.
    pub block_state_calls: Vec<SimBlock>,
    /// Flag to determine whether to trace ERC20/ERC721 token transfers within transactions.
    pub trace_transfers: bool,
    /// Flag to enable or disable validation of the transaction sequence in the blocks.
    pub validation: bool,
    /// Flag to decide if full transactions should be returned instead of just their outcomes.
    pub return_full_transactions: bool,
}

/// Represents a simulator backend to handle state and transaction processing.
#[derive(Clone, Debug)]
pub struct Simulator {
    //What we should use here for backend ?
    //b              Backend
    /// List of hashes representing the blocks to be simulated.
    pub hashes: Vec<B256>,
    // should we use StadeDB of revm here ?

    // state          *state.StateDB
    /// The base block header from which the simulation starts.
    pub base: Header,
    /// 
    pub trace_transfers: bool,
    /// Indicates whether transaction validation is performed.
    pub validate: bool,
    /// Indicates whether full transaction details are returned.
    pub full_tx: bool,
}
