//! 'eth_simulateV1' Request / Response types: <https://github.com/ethereum/execution-apis/pull/484>

use alloy_primitives::{Address, Bytes, Log, B256};
use serde::{Deserialize, Serialize};

use crate::{state::StateOverride, BlockOverrides, TransactionRequest};

/// The maximum number of blocks that can be simulated in a single request,
pub const MAX_SIMULATE_BLOCKS: u64 = 256;

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
/// Represents the result of simulating a block.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatedBlock {
    /// The number of the block.
    #[serde(with = "alloy_serde::quantity")]
    pub number: u64,
    /// The hash of the block.
    pub hash: B256,
    /// The timestamp of the block.
    #[serde(with = "alloy_serde::quantity")]
    pub timestamp: u64,
    /// The gas limit of the block.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    /// The amount of gas used in the block.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// The recipient of the block's fees.
    pub fee_recipient: Address,
    /// The base fee per gas unit for the block.
    #[serde(with = "alloy_serde::quantity")]
    pub base_fee_per_gas: u64,
    /// The previous RANDAO value of the block.
    pub prev_randao: B256,
    /// A vector of results for each call in the block.
    pub calls: Vec<SimCallResult>,
}
/// The response type for the eth_simulateV1 method.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulateV1Response {
    /// Simulated blocks vector.
    pub simulated_blocks: Vec<SimulatedBlock>,
}
/// Captures the outcome of a transaction simulation.
/// It includes the return value, logs produced, gas used, and the status of the transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimCallResult {
    /// The raw bytes returned by the transaction.
    pub return_value: Bytes,
    /// Logs generated during the execution of the transaction.
    #[serde(default)]
    pub logs: Vec<Log>,
    /// The amount of gas used by the transaction.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// The final status of the transaction, typically indicating success or failure.
    #[serde(with = "alloy_serde::quantity")]
    pub status: u64,
    /// Error in case the call failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<SimulateError>,
}

/// Simulation options for executing multiple blocks and transactions.
///
/// This struct configures how simulations are executed, including whether to trace token transfers,
/// validate transaction sequences, and whether to return full transaction objects.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatePayload {
    /// Array of block state calls to be executed at specific, optional block/state.
    pub block_state_calls: Vec<SimBlock>,
    /// Flag to determine whether to trace ERC20/ERC721 token transfers within transactions.
    #[serde(default)]
    pub trace_transfers: bool,
    /// Flag to enable or disable validation of the transaction sequence in the blocks.
    #[serde(default)]
    pub validation: bool,
    /// Flag to decide if full transactions should be returned instead of just their hashes.
    pub return_full_transactions: bool,
}

/// The error response returned by the `eth_simulateV1` method.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulateError {
    /// Code error
    /// -3200: Execution reverted
    /// -32015: VM execution error
    pub code: i32,
    /// Message error
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, TxKind};
    use serde_json::json;

    #[test]
    fn test_eth_simulate_v1_account_not_precompile() {
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_simulateV1",
            "params": [{
                "blockStateCalls": [
                    {
                        "blockOverrides": {},
                        "stateOverrides": {
                            "0xc000000000000000000000000000000000000000": {
                                "nonce": "0x5"
                            }
                        },
                        "calls": []
                    },
                    {
                        "blockOverrides": {},
                        "stateOverrides": {
                            "0xc000000000000000000000000000000000000000": {
                                "code": "0x600035600055"
                            }
                        },
                        "calls": [
                            {
                                "from": "0xc000000000000000000000000000000000000000",
                                "to": "0xc000000000000000000000000000000000000000",
                                "nonce": "0x0"
                            },
                            {
                                "from": "0xc100000000000000000000000000000000000000",
                                "to": "0xc100000000000000000000000000000000000000",
                                "nonce": "0x5"
                            }
                        ]
                    }
                ],
                "traceTransfers": false,
                "validation": true,
                "returnFullTransactions": false
            }, "latest"]
        });

        let sim_opts: SimulatePayload =
            serde_json::from_value(request_json["params"][0].clone()).unwrap();

        let address_1: Address = "0xc000000000000000000000000000000000000000".parse().unwrap();
        let address_2: Address = "0xc100000000000000000000000000000000000000".parse().unwrap();

        assert!(sim_opts.validation);
        assert_eq!(sim_opts.block_state_calls.len(), 2);

        let block_state_call_1 = &sim_opts.block_state_calls[0];
        assert!(block_state_call_1.state_overrides.contains_key(&address_1));
        assert_eq!(block_state_call_1.state_overrides.get(&address_1).unwrap().nonce.unwrap(), 5);

        let block_state_call_2 = &sim_opts.block_state_calls[1];
        assert!(block_state_call_2.state_overrides.contains_key(&address_1));

        assert_eq!(block_state_call_2.calls.len(), 2);
        assert_eq!(block_state_call_2.calls[0].from.unwrap(), address_1);
        assert_eq!(block_state_call_2.calls[0].to.unwrap(), TxKind::Call(address_1));
        assert_eq!(block_state_call_2.calls[0].nonce.unwrap(), 0);
        assert_eq!(block_state_call_2.calls[1].from.unwrap(), address_2);
        assert_eq!(block_state_call_2.calls[1].to.unwrap(), TxKind::Call(address_2));
        assert_eq!(block_state_call_2.calls[1].nonce.unwrap(), 5);
    }
}
