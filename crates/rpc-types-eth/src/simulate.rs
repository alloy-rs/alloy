//! 'eth_simulateV1' Request / Response types: <https://github.com/ethereum/execution-apis/pull/484>

use crate::{
    alloc::string::ToString, error::EthRpcErrorCode, state::StateOverride, Block, BlockOverrides,
    Log, TransactionRequest,
};
use alloc::{string::String, vec::Vec};
use alloy_eips::eip8141::FrameStatus;
use alloy_primitives::{Address, Bytes, U256};

/// The maximum number of blocks that can be simulated in a single request,
pub const MAX_SIMULATE_BLOCKS: u64 = 256;

/// Represents a batch of calls to be simulated sequentially within a block.
/// This struct includes block and state overrides as well as the transaction requests to be
/// executed.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        rename_all = "camelCase",
        bound(
            deserialize = "TxReq: serde::Deserialize<'de>",
            serialize = "TxReq: serde::Serialize"
        )
    )
)]
pub struct SimBlock<TxReq = TransactionRequest> {
    /// Modifications to the default block characteristics.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub block_overrides: Option<BlockOverrides>,
    /// State modifications to apply before executing the transactions.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub state_overrides: Option<StateOverride>,
    /// A vector of transactions to be simulated.
    #[cfg_attr(feature = "serde", serde(default = "Vec::new"))]
    pub calls: Vec<TxReq>,
}

impl<TxReq> Default for SimBlock<TxReq> {
    fn default() -> Self {
        Self { block_overrides: None, state_overrides: None, calls: Vec::new() }
    }
}

impl<TxReq> SimBlock<TxReq> {
    /// Enables state overrides
    pub fn with_state_overrides(mut self, overrides: StateOverride) -> Self {
        self.state_overrides = Some(overrides);
        self
    }

    /// Enables block overrides
    pub fn with_block_overrides(mut self, overrides: BlockOverrides) -> Self {
        self.block_overrides = Some(overrides);
        self
    }

    /// Adds a call to the block.
    pub fn call(mut self, call: TxReq) -> Self {
        self.calls.push(call);
        self
    }

    /// Adds multiple calls to the block.
    pub fn extend_calls(mut self, calls: impl IntoIterator<Item = TxReq>) -> Self {
        self.calls.extend(calls);
        self
    }

    /// Returns the block's block number override if it exists.
    pub fn block_number_override(&self) -> Option<U256> {
        self.block_overrides.as_ref().and_then(|overrides| overrides.number)
    }
}

/// Represents the result of simulating a block.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SimulatedBlock<B = Block> {
    /// The simulated block.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: B,
    /// A vector of results for each call in the block.
    pub calls: Vec<SimCallResult>,
}

impl<B> SimulatedBlock<B> {
    /// Returns an iterator over all logs emitted by the calls in this block, in call order.
    pub fn logs(&self) -> impl Iterator<Item = &Log> {
        self.calls.iter().flat_map(|call| &call.logs)
    }

    /// Consumes the block and returns an iterator over all logs emitted by the calls in this
    /// block, in call order.
    pub fn into_logs(self) -> impl Iterator<Item = Log> {
        self.calls.into_iter().flat_map(|call| call.logs)
    }
}

/// Captures the outcome of a transaction simulation.
/// It includes the return value, logs produced, gas used, and the status of the transaction.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SimCallResult {
    /// The raw bytes returned by the transaction.
    pub return_data: Bytes,
    /// Logs generated during the execution of the transaction.
    #[cfg_attr(feature = "serde", serde(default))]
    pub logs: Vec<Log>,
    /// The amount of gas used by the transaction.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas_used: u64,
    /// Maximum gas consumed during execution, before refunds.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub max_used_gas: Option<u64>,
    /// The final status of the transaction, typically indicating success or failure.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub status: bool,
    /// Error in case the call failed
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub error: Option<SimulateError>,
}

/// Execution result of a single [EIP-8141] frame in `eth_simulateV1`.
///
/// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FrameCallResult {
    /// Whether the frame succeeded, failed or was skipped.
    pub status: FrameStatus,
    /// Total gas consumed by the frame.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas_used: u64,
    /// Execution gas consumed by the frame.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub execution_gas_used: u64,
    /// State gas consumed by the frame.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub state_gas_used: u64,
    /// Surviving logs emitted by the frame.
    pub logs: Vec<Log>,
    /// Frame output or revert bytes.
    pub return_data: Bytes,
    /// Error in case the frame failed.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub error: Option<SimulateError>,
}

/// Recognized shape of an [EIP-8141] validation prefix.
///
/// An optional expiry verifier frame may precede each shape and is not represented in this value.
///
/// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum FrameSimulationPrefixShape {
    /// The sender verifies and approves both execution and payment.
    SelfVerify,
    /// A deployment frame precedes sender verification and approval.
    DeploySelfVerify,
    /// The sender approves execution before a separate payer approves payment.
    OnlyVerifyPay,
    /// A deployment frame precedes separate sender and payer approvals.
    DeployOnlyVerifyPay,
}

impl FrameSimulationPrefixShape {
    /// Classifies a validation prefix from its layout.
    ///
    /// `prefix_end` is the number of frames in the validation prefix, `deploy_index` the position
    /// of its deployment frame and `expiry_index` the position of its leading expiry verifier
    /// frame, if any. The expiry verifier is not part of the shape and is subtracted from the
    /// prefix length. Returns `None` if the remaining frames match none of the known shapes.
    pub const fn from_validation_prefix(
        prefix_end: usize,
        deploy_index: Option<usize>,
        expiry_index: Option<usize>,
    ) -> Option<Self> {
        let leading_expiry_frame = if expiry_index.is_some() { 1 } else { 0 };
        match (deploy_index.is_some(), prefix_end.saturating_sub(leading_expiry_frame)) {
            (false, 1) => Some(Self::SelfVerify),
            (true, 2) => Some(Self::DeploySelfVerify),
            (false, 2) => Some(Self::OnlyVerifyPay),
            (true, 3) => Some(Self::DeployOnlyVerifyPay),
            _ => None,
        }
    }
}

/// Gas usage and outcome of a single frame in a [`FrameSimulationResult`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FrameSimulationFrameResult {
    /// Execution gas consumed by the frame.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub execution_gas: u64,
    /// State gas consumed by the frame.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub state_gas: u64,
    /// Whether the frame succeeded, failed or was skipped.
    pub status: FrameStatus,
}

/// Result of simulating an [EIP-8141] frame transaction.
///
/// `valid` reports whether the transaction's validation prefix passed against the selected state.
/// It does not assert pool admission: nonce ordering and other pool-local policy remain outside the
/// simulation. When the prefix is valid, the optional execution fields describe a separate,
/// non-committing execution of the complete transaction.
///
/// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FrameSimulationResult {
    /// Whether the validation prefix passed.
    pub valid: bool,
    /// Maximum transaction cost the payer has to cover.
    pub max_cost: U256,
    /// Recognized shape of the validation prefix, `None` if it is structurally invalid.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub prefix_shape: Option<FrameSimulationPrefixShape>,
    /// Account that approved payment of the maximum transaction cost.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub payer: Option<Address>,
    /// Reason the validation prefix was not accepted.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub violation: Option<String>,
    /// Gas used by the complete transaction for fee accounting.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub gas_used: Option<u64>,
    /// Results of the frames reached during the complete execution.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub frames: Option<Vec<FrameSimulationFrameResult>>,
}

impl FrameSimulationResult {
    /// Creates the result for a transaction whose validation prefix was not accepted.
    pub fn invalid(
        max_cost: U256,
        prefix_shape: Option<FrameSimulationPrefixShape>,
        violation: impl Into<String>,
    ) -> Self {
        Self {
            valid: false,
            max_cost,
            prefix_shape,
            payer: None,
            violation: Some(violation.into()),
            gas_used: None,
            frames: None,
        }
    }
}

/// Simulation options for executing multiple blocks and transactions.
///
/// This struct configures how simulations are executed, including whether to trace token transfers,
/// validate transaction sequences, and whether to return full transaction objects.
/// The RPC accepts at most [`MAX_SIMULATE_BLOCKS`] blocks; this type and its builder methods do not
/// enforce that limit.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        rename_all = "camelCase",
        bound(
            deserialize = "TxReq: serde::Deserialize<'de>",
            serialize = "TxReq: serde::Serialize"
        )
    )
)]
pub struct SimulatePayload<TxReq = TransactionRequest> {
    /// Array of block state calls to be executed at specific, optional block/state.
    #[cfg_attr(feature = "serde", serde(default))]
    pub block_state_calls: Vec<SimBlock<TxReq>>,
    /// Flag to determine whether to trace ERC20/ERC721 token transfers within transactions.
    #[cfg_attr(feature = "serde", serde(default))]
    pub trace_transfers: bool,
    /// Flag to enable or disable validation of the transaction sequence in the blocks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub validation: bool,
    /// Flag to decide if full transactions should be returned instead of just their hashes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub return_full_transactions: bool,
}

impl<TxReq> Default for SimulatePayload<TxReq> {
    fn default() -> Self {
        Self {
            block_state_calls: Vec::new(),
            trace_transfers: false,
            validation: false,
            return_full_transactions: false,
        }
    }
}

impl<TxReq> SimulatePayload<TxReq> {
    /// Adds a block to the simulation payload.
    pub fn extend(mut self, block: SimBlock<TxReq>) -> Self {
        self.block_state_calls.push(block);
        self
    }

    /// Adds multiple blocks to the simulation payload.
    pub fn extend_blocks(mut self, blocks: impl IntoIterator<Item = SimBlock<TxReq>>) -> Self {
        self.block_state_calls.extend(blocks);
        self
    }

    /// Enables tracing of token transfers.
    pub const fn with_trace_transfers(mut self) -> Self {
        self.trace_transfers = true;
        self
    }

    /// Enables validation of the transaction sequence.
    pub const fn with_validation(mut self) -> Self {
        self.validation = true;
        self
    }

    /// Enables returning full transactions.
    pub const fn with_full_transactions(mut self) -> Self {
        self.return_full_transactions = true;
        self
    }
}

/// The error response returned by the `eth_simulateV1` method.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SimulateError {
    /// Error code.
    ///
    /// Known values:
    /// - [`Self::EXECUTION_REVERTED_CODE`] for `Execution reverted`
    /// - [`Self::VM_EXECUTION_ERROR_CODE`] for `VM execution error`
    pub code: i32,
    /// Message error
    pub message: String,
    /// Data for the error, e.g. revert reason.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub data: Option<Bytes>,
}

impl SimulateError {
    /// `Execution reverted` error code.
    pub const EXECUTION_REVERTED_CODE: i32 = EthRpcErrorCode::ExecutionError.code();
    /// `VM execution error` error code.
    pub const VM_EXECUTION_ERROR_CODE: i32 = -32015;
    /// `Invalid params` error code.
    pub const INVALID_PARAMS_ERROR_CODE: i32 = -32602;

    /// Creates a new invalid params error.
    pub fn invalid_params() -> Self {
        Self {
            code: Self::INVALID_PARAMS_ERROR_CODE,
            message: "invalid params".to_string(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{bytes, Address, TxKind};
    #[cfg(feature = "serde")]
    use serde_json::json;
    use similar_asserts::assert_eq;

    #[test]
    #[cfg(feature = "serde")]
    fn test_deserialize_simulate_error_no_data() {
        let error_json = json!({
            "code": -32000,
            "message": "Execution reverted"
        });
        let err: SimulateError = serde_json::from_value(error_json).unwrap();
        assert_eq!(err.data, None);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_deserialize_simulate_error_with_data() {
        let error_json = json!({
            "code": -32000,
            "message": "Execution reverted",
            "data": "0xcabedea8"
        });
        let err: SimulateError = serde_json::from_value(error_json).unwrap();
        assert_eq!(err.data, Some(bytes!("cabedea8")));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn frame_simulation_result_serde() {
        let result = FrameSimulationResult {
            valid: true,
            max_cost: U256::from(123),
            prefix_shape: Some(FrameSimulationPrefixShape::OnlyVerifyPay),
            payer: Some(Address::repeat_byte(0x11)),
            violation: None,
            gas_used: Some(456),
            frames: Some(vec![FrameSimulationFrameResult {
                execution_gas: 5,
                state_gas: 7,
                status: FrameStatus::Failure,
            }]),
        };
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(
            value,
            json!({
                "valid": true,
                "maxCost": "0x7b",
                "prefixShape": "onlyVerifyPay",
                "payer": "0x1111111111111111111111111111111111111111",
                "gasUsed": "0x1c8",
                "frames": [{
                    "executionGas": "0x5",
                    "stateGas": "0x7",
                    "status": "0x0",
                }],
            })
        );
        assert_eq!(serde_json::from_value::<FrameSimulationResult>(value).unwrap(), result);

        let invalid = FrameSimulationResult::invalid(U256::from(123), None, "nonce mismatch");
        let value = serde_json::to_value(&invalid).unwrap();
        assert_eq!(
            value,
            json!({"valid": false, "maxCost": "0x7b", "violation": "nonce mismatch"})
        );
        assert_eq!(serde_json::from_value::<FrameSimulationResult>(value).unwrap(), invalid);
    }

    #[test]
    fn frame_simulation_prefix_shape_ignores_expiry_frame() {
        assert_eq!(
            FrameSimulationPrefixShape::from_validation_prefix(1, None, None),
            Some(FrameSimulationPrefixShape::SelfVerify)
        );
        assert_eq!(
            FrameSimulationPrefixShape::from_validation_prefix(3, Some(1), Some(0)),
            Some(FrameSimulationPrefixShape::DeploySelfVerify)
        );
        assert_eq!(
            FrameSimulationPrefixShape::from_validation_prefix(2, None, None),
            Some(FrameSimulationPrefixShape::OnlyVerifyPay)
        );
        assert_eq!(
            FrameSimulationPrefixShape::from_validation_prefix(4, Some(1), Some(0)),
            Some(FrameSimulationPrefixShape::DeployOnlyVerifyPay)
        );
        assert_eq!(FrameSimulationPrefixShape::from_validation_prefix(4, None, None), None);
    }

    #[test]
    #[cfg(feature = "serde")]
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
        assert!(block_state_call_1.state_overrides.as_ref().unwrap().contains_key(&address_1));
        assert_eq!(
            block_state_call_1
                .state_overrides
                .as_ref()
                .unwrap()
                .get(&address_1)
                .unwrap()
                .nonce
                .unwrap(),
            5
        );

        let block_state_call_2 = &sim_opts.block_state_calls[1];
        assert!(block_state_call_2.state_overrides.as_ref().unwrap().contains_key(&address_1));

        assert_eq!(block_state_call_2.calls.len(), 2);
        assert_eq!(block_state_call_2.calls[0].from.unwrap(), address_1);
        assert_eq!(block_state_call_2.calls[0].to.unwrap(), TxKind::Call(address_1));
        assert_eq!(block_state_call_2.calls[0].nonce.unwrap(), 0);
        assert_eq!(block_state_call_2.calls[1].from.unwrap(), address_2);
        assert_eq!(block_state_call_2.calls[1].to.unwrap(), TxKind::Call(address_2));
        assert_eq!(block_state_call_2.calls[1].nonce.unwrap(), 5);
    }

    #[test]
    fn test_simulated_block_logs() {
        let log = |address| Log {
            inner: alloy_primitives::Log::new_unchecked(address, Vec::new(), Bytes::new()),
            ..Default::default()
        };
        let block: SimulatedBlock = SimulatedBlock {
            inner: Default::default(),
            calls: vec![
                SimCallResult {
                    logs: vec![log(Address::with_last_byte(1)), log(Address::with_last_byte(2))],
                    ..Default::default()
                },
                SimCallResult::default(),
                SimCallResult { logs: vec![log(Address::with_last_byte(3))], ..Default::default() },
            ],
        };

        let expected: Vec<_> = (1..=3).map(Address::with_last_byte).collect();
        assert_eq!(block.logs().map(|log| log.address()).collect::<Vec<_>>(), expected);
        assert_eq!(block.into_logs().map(|log| log.address()).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn test_simulate_error_codes() {
        assert_eq!(SimulateError::EXECUTION_REVERTED_CODE, EthRpcErrorCode::ExecutionError.code());
        assert_eq!(SimulateError::VM_EXECUTION_ERROR_CODE, -32015);
        assert_eq!(SimulateError::invalid_params().code, SimulateError::INVALID_PARAMS_ERROR_CODE);
    }
}
