//! EIP-8037 state-gas tracer types.

use serde::{Deserialize, Serialize};

/// The per-transaction two-dimensional gas summary returned by `stateGasTracer`.
///
/// The shape is specified by the
/// [execution-apis state-gas tracer proposal](https://github.com/ethereum/execution-apis/pull/852).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateGasTrace {
    /// Receipt gas used, after refunds and any calldata floor.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// Gross execution-dimension gas used by the transaction.
    #[serde(with = "alloy_serde::quantity")]
    pub execution_gas_used: u64,
    /// Gross state-dimension gas used by the transaction.
    #[serde(with = "alloy_serde::quantity")]
    pub state_gas_used: u64,
    /// EIP-3529 gas refund applied at the transaction boundary.
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
