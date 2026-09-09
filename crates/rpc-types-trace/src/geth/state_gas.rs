//! Transaction gas accounting for [EIP-8037](https://eips.ethereum.org/EIPS/eip-8037).
//!
//! **Execution gas**, also called **regular gas**, pays for computation, memory, access costs,
//! and intrinsic transaction costs. **State gas** pays for state creation, such as a new storage
//! slot, account, or deployed code. Reading or updating existing state still costs execution gas.
//!
//! A transaction supplies one gas limit covering both dimensions. EIP-8037 splits the available
//! gas into regular gas and a state gas reservoir. Execution charges use regular gas only; state
//! charges use the reservoir first and spill into regular gas when it is empty. `GAS`/`gasleft()`
//! exposes only regular gas. Child calls receive the reservoir in full, independently of the
//! 63/64 rule for regular gas, so a call's gas allowance is not a combined state/execution budget.
//!
//! State **refills** undo state creation charges during execution and are already deducted from
//! reported transaction state gas. They differ from the ordinary transaction **refund counter**,
//! whose application is capped by [EIP-3529](https://eips.ethereum.org/EIPS/eip-3529).
//!
//! # Choosing a value
//!
//! - Use [`StateGasTrace::gas_used`] or the transaction receipt's `gasUsed` for charged gas. It
//!   includes both dimensions, after refunds and the calldata floor; do not add state gas again.
//! - Use [`StateGasTrace::execution_gas_used`] and [`StateGasTrace::state_gas_used`] for the
//!   transaction's separate block-accounting contributions. With EIP-7778, execution gas is before
//!   ordinary refunds; state gas is still net of state refills.
//! - Use [`StateGasTrace::gas_refund`] for the reported transaction refund, not a nested frame's
//!   uncapped refund counter. It is separate from state refills and unused gas returned to the
//!   sender.
//! - Use `eth_estimateGas` on a node implementing the target network's rules to choose a gas limit.
//!   Neither charged gas nor the sum of the two reported dimensions guarantees enough gas:
//!   temporary state charges, forwarding rules, and the regular gas cap also matter.
//!
//! Without a binding calldata floor, `execution_gas_used + state_gas_used - gas_refund` equals
//! receipt gas under the EIP-8037/EIP-7778 accounting model. With a binding floor, use `gas_used`
//! directly: the execution contribution has its own floor and cannot always reconstruct the
//! receipt. Neither figure includes EIP-4844 blob gas, which has separate accounting and fees.
//!
//! # Network and RPC support
//!
//! EIP-8037 applies only when activated by the target network's hardfork rules (Amsterdam in
//! Ethereum EVM implementations). An EVM-compatible network or an Alloy type does not imply
//! activation. Before activation, state creation uses the ordinary gas schedule and the separate
//! state dimension is zero. A node may omit optional breakdown fields or reject `stateGasTracer`
//! if it does not implement the tracer. `None` means not reported, not zero.
//!
//! The tracing wire format is still described by an
//! [execution-apis proposal](https://github.com/ethereum/execution-apis/pull/852); support and
//! historical responses depend on the node. Alloy uses the wire name `executionGasUsed`; older
//! proposal/node revisions may use `regularGasUsed`, which these fields do not alias. Match the
//! node's response schema to the Alloy version. These types decode responses; they do not enable
//! EIPs.

use serde::{Deserialize, Serialize};

/// The per-transaction two-dimensional gas summary returned by `stateGasTracer`.
///
/// Based on the
/// [execution-apis state-gas tracer proposal](https://github.com/ethereum/execution-apis/pull/852).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateGasTrace {
    /// Total gas charged for this transaction, including execution and state gas.
    ///
    /// Matches receipt `gasUsed`: includes intrinsic costs, subtracts applicable transaction
    /// refunds, and applies the [EIP-7623](https://eips.ethereum.org/EIPS/eip-7623) calldata floor.
    /// State refills are already accounted for. Excludes blob gas. Do not add `state_gas_used`
    /// again. This is gas charged, not the minimum gas limit required to execute.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// Regular (execution) gas contribution to block accounting, excluding EIP-8037 state gas.
    ///
    /// Includes intrinsic gas and the execution-dimension calldata floor. With
    /// [EIP-7778](https://eips.ethereum.org/EIPS/eip-7778), this is before ordinary transaction refunds.
    /// Without EIP-8037, state creation remains part of the ordinary gas schedule. A floor can
    /// make this value larger than the execution work actually consumed; use `gas_used` for
    /// receipt gas.
    #[serde(with = "alloy_serde::quantity")]
    pub execution_gas_used: u64,
    /// Net [EIP-8037](https://eips.ethereum.org/EIPS/eip-8037) state gas for the whole transaction.
    ///
    /// Covers state creation and excludes regular execution gas. State refills and rollback have
    /// already been deducted; this is not the peak or the sum of all positive state charges.
    /// It remains net even under EIP-7778. Do not subtract `gas_refund` from this field.
    /// Zero when EIP-8037 is inactive. Unlike a nested frame's signed state delta, this is
    /// unsigned.
    #[serde(with = "alloy_serde::quantity")]
    pub state_gas_used: u64,
    /// Ordinary transaction gas refund reported after transaction-boundary refund processing.
    ///
    /// Subject to the [EIP-3529](https://eips.ethereum.org/EIPS/eip-3529) cap, rather than the raw
    /// refund counter. The calldata floor can limit the fee saving; use `gas_used` for charged
    /// gas. Excludes EIP-8037 state refills (already netted into `state_gas_used`) and unused
    /// gas. Refunds do not fund execution and must not be subtracted when choosing a gas
    /// limit.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_refund: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geth::{GethDebugTracingOptions, GethTrace, StructLog};

    #[test]
    fn test_state_gas_trace_serde() {
        let trace: StateGasTrace = serde_json::from_str(
            r#"{
                "gasUsed": "0x5208",
                "executionGasUsed": "0x5208",
                "stateGasUsed": "0x0",
                "gasRefund": "0x0"
            }"#,
        )
        .unwrap();

        assert_eq!(trace.gas_used, 21000);
        assert_eq!(trace.execution_gas_used, 21000);
        assert_eq!(trace.state_gas_used, 0);
        assert_eq!(trace.gas_refund, 0);
        assert_eq!(
            serde_json::to_value(&trace).unwrap(),
            serde_json::json!({
                "gasUsed": "0x5208",
                "executionGasUsed": "0x5208",
                "stateGasUsed": "0x0",
                "gasRefund": "0x0"
            })
        );
    }

    #[test]
    fn test_state_gas_trace_response_and_options() {
        let trace: GethTrace = serde_json::from_str(
            r#"{
                "gasUsed": "0x5208",
                "executionGasUsed": "0x5208",
                "stateGasUsed": "0x0",
                "gasRefund": "0x0"
            }"#,
        )
        .unwrap();
        assert!(trace.is_state_gas());
        assert_eq!(trace.try_into_state_gas_trace().unwrap().gas_used, 21000);

        let options = GethDebugTracingOptions::state_gas_tracer();
        assert_eq!(options.tracer.unwrap().as_str(), "stateGasTracer");
    }

    #[test]
    fn test_signed_state_gas_cost() {
        let log: StructLog = serde_json::from_str(
            r#"{
                "pc": 0,
                "op": "CREATE",
                "gas": 100,
                "gasCost": 10,
                "stateGasCost": -5,
                "stateGasReservoir": 90,
                "depth": 1
            }"#,
        )
        .unwrap();

        assert_eq!(log.state_gas_cost, Some(-5));
        assert_eq!(log.state_gas_reservoir, Some(90));
        assert_eq!(serde_json::to_value(&log).unwrap()["stateGasCost"], -5);
    }
}
