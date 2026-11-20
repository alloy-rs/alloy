//! Simulation payloads that are decoded after the caller's limits are known.

use super::{SimulatePayload, SimulatePayloadLimitExceeded, SimulatePayloadSeed};
use crate::TransactionRequest;
use alloc::boxed::Box;
use serde::{
    de::{DeserializeOwned, DeserializeSeed},
    Deserialize, Deserializer, Serialize,
};
use serde_json::value::RawValue;

/// An `eth_simulateV1` payload whose decoding is deferred until the caller's limits are known.
///
/// Deserializing this type only captures the raw JSON value, without allocating any blocks or
/// calls. [`Self::parse`] decodes it later with [`SimulatePayloadSeed`] and the block and call
/// limits supplied by the caller.
///
/// Use it as the payload parameter of an RPC server method whose limits are only known inside the
/// handler, for example because they come from runtime configuration. Server frameworks such as
/// `jsonrpsee` deserialize all parameters before the handler runs, so a [`SimulatePayload`]
/// parameter would be decoded without any limits.
///
/// Serializing [`Self::Raw`] directly to JSON text preserves the original JSON. Converting it
/// to [`serde_json::Value`] parses that text again using the rules for a generic JSON value.
/// To forward a request with enforced limits, serialize the [`SimulatePayload`] returned by
/// [`Self::parse`].
///
/// Deserialization requires a `serde_json` deserializer and does not work inside
/// `#[serde(untagged)]` or `#[serde(flatten)]` containers, which buffer their input.
///
/// # Example
///
/// ```
/// use alloy_rpc_types_eth::simulate::{
///     RawSimulatePayload, RawSimulatePayloadError, SimulatePayloadLimitExceeded,
/// };
///
/// let input = r#"{"blockStateCalls":[{"calls":[{}]},{"calls":[]}]}"#;
/// let payload: RawSimulatePayload = serde_json::from_str(input)?;
///
/// let err = payload.clone().parse(1, 16).unwrap_err();
/// assert!(matches!(
///     err,
///     RawSimulatePayloadError::LimitExceeded(SimulatePayloadLimitExceeded::Blocks)
/// ));
///
/// let payload = payload.parse(2, 16)?;
/// let forwarded = serde_json::to_value(&payload)?;
/// assert_eq!(forwarded["blockStateCalls"].as_array().unwrap().len(), 2);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum RawSimulatePayload<TxReq = TransactionRequest> {
    /// Raw JSON input, kept until [`RawSimulatePayload::parse`] applies the caller's limits.
    Raw(Box<RawValue>),
    /// A decoded payload, for example one built by a client or an internal caller.
    Parsed(SimulatePayload<TxReq>),
}

impl<TxReq> From<SimulatePayload<TxReq>> for RawSimulatePayload<TxReq> {
    fn from(payload: SimulatePayload<TxReq>) -> Self {
        Self::Parsed(payload)
    }
}

impl<'de, TxReq> Deserialize<'de> for RawSimulatePayload<TxReq> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Box::<RawValue>::deserialize(deserializer).map(Self::Raw)
    }
}

impl<TxReq: DeserializeOwned> RawSimulatePayload<TxReq> {
    /// Decodes the payload, allowing at most `max_blocks` explicit blocks and `max_calls` calls
    /// across all blocks.
    ///
    /// Raw input is decoded with [`SimulatePayloadSeed`], which skips excess blocks and calls
    /// without deserializing them, and must be a single JSON value with no trailing input. A
    /// [`Parsed`](Self::Parsed) payload is checked against the same limits, so a returned payload
    /// always satisfies both.
    ///
    /// # Errors
    ///
    /// - [`RawSimulatePayloadError::Json`] if the raw input is not a valid simulation payload.
    /// - [`RawSimulatePayloadError::LimitExceeded`] with [`SimulatePayloadLimitExceeded::Blocks`]
    ///   if the payload has more than `max_blocks` blocks. This takes precedence over the call
    ///   limit.
    /// - [`RawSimulatePayloadError::LimitExceeded`] with [`SimulatePayloadLimitExceeded::Calls`] if
    ///   the payload has more than `max_calls` calls in total.
    pub fn parse(
        self,
        max_blocks: usize,
        max_calls: usize,
    ) -> Result<SimulatePayload<TxReq>, RawSimulatePayloadError> {
        match self {
            Self::Raw(raw) => parse_str(raw.get(), max_blocks, max_calls),
            Self::Parsed(payload) => {
                check_limits(&payload, max_blocks, max_calls)?;
                Ok(payload)
            }
        }
    }
}

/// Error returned by [`RawSimulatePayload::parse`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RawSimulatePayloadError {
    /// The raw input is not a valid simulation payload, for example malformed JSON, a field of
    /// the wrong type, or trailing input.
    ///
    /// JSON-RPC servers usually report this as invalid params (`-32602`).
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The payload exceeds the block or call limit passed to [`RawSimulatePayload::parse`].
    ///
    /// Servers choose the error code. For example, a server that takes its block limit from its
    /// configuration may report [`SimulatePayloadLimitExceeded::Blocks`] as the client limit error
    /// (`-38026`) and [`SimulatePayloadLimitExceeded::Calls`] as invalid params (`-32602`).
    #[error(transparent)]
    LimitExceeded(#[from] SimulatePayloadLimitExceeded),
}

fn parse_str<TxReq: DeserializeOwned>(
    input: &str,
    max_blocks: usize,
    max_calls: usize,
) -> Result<SimulatePayload<TxReq>, RawSimulatePayloadError> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let payload = SimulatePayloadSeed::new(max_blocks, max_calls).deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(payload?)
}

fn check_limits<TxReq>(
    payload: &SimulatePayload<TxReq>,
    max_blocks: usize,
    max_calls: usize,
) -> Result<(), SimulatePayloadLimitExceeded> {
    if payload.block_state_calls.len() > max_blocks {
        return Err(SimulatePayloadLimitExceeded::Blocks);
    }
    let calls = payload
        .block_state_calls
        .iter()
        .fold(0usize, |calls, block| calls.saturating_add(block.calls.len()));
    if calls > max_calls {
        return Err(SimulatePayloadLimitExceeded::Calls);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulate::SimBlock;
    use alloc::{string::String, vec::Vec};
    use assert_matches::assert_matches;

    const MAX_CALLS: usize = 4096;

    fn payload_with_calls(counts: &[usize]) -> String {
        let blocks = counts
            .iter()
            .map(|&count| format!(r#"{{"calls":[{}]}}"#, vec!["{}"; count].join(",")))
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"blockStateCalls":[{blocks}]}}"#)
    }

    fn parse(
        input: &str,
        max_blocks: usize,
    ) -> Result<SimulatePayload<TransactionRequest>, RawSimulatePayloadError> {
        serde_json::from_str::<RawSimulatePayload>(input).unwrap().parse(max_blocks, MAX_CALLS)
    }

    #[test]
    fn uses_configured_block_limit() {
        let input = payload_with_calls(&[0; 1025]);
        assert_eq!(parse(&input, 1025).unwrap().block_state_calls.len(), 1025);
        assert_matches!(
            parse(&input, 1024),
            Err(RawSimulatePayloadError::LimitExceeded(SimulatePayloadLimitExceeded::Blocks))
        );
    }

    #[test]
    fn bounds_calls_across_blocks() {
        let input = payload_with_calls(&[MAX_CALLS - 1, 2]);
        assert_matches!(
            parse(&input, 2),
            Err(RawSimulatePayloadError::LimitExceeded(SimulatePayloadLimitExceeded::Calls))
        );
    }

    #[test]
    fn accepts_call_limit() {
        let input = payload_with_calls(&[MAX_CALLS]);
        assert_eq!(parse(&input, 1).unwrap().block_state_calls[0].calls.len(), MAX_CALLS);
    }

    #[test]
    fn preserves_payload_fields() {
        let input = r#"{"blockStateCalls":[{"calls":[{}]}],"traceTransfers":true,"validation":true,"returnFullTransactions":true}"#;
        let expected = serde_json::from_str::<SimulatePayload<TransactionRequest>>(input).unwrap();
        let parsed = parse(input, 1).unwrap();
        assert!(parsed.trace_transfers && parsed.validation && parsed.return_full_transactions);
        assert_eq!(serde_json::to_value(parsed).unwrap(), serde_json::to_value(expected).unwrap());
    }

    #[test]
    fn serializes_like_simulate_payload() {
        let payload = SimulatePayload::<TransactionRequest> {
            block_state_calls: vec![SimBlock::default()],
            trace_transfers: true,
            ..Default::default()
        };
        let original = serde_json::to_value(&payload).unwrap();
        let wrapped = serde_json::to_value(RawSimulatePayload::from(payload)).unwrap();
        assert_eq!(wrapped, original);
    }

    #[test]
    fn forwards_raw_payload_without_changing_it() {
        let input = r#"{"blockStateCalls":[{"calls":[{}]}],"validation":true}"#;
        let wrapped = serde_json::from_str::<RawSimulatePayload>(input).unwrap();
        assert_matches!(&wrapped, RawSimulatePayload::Raw(raw) if raw.get() == input);
        assert_eq!(serde_json::to_string(&wrapped).unwrap(), input);
    }

    #[test]
    fn forwards_the_parsed_simulation_fields() {
        let input = r#"{"$serde_json::private::RawValue":"metadata","blockStateCalls":[{"calls":[]}],"validation":true}"#;
        let wrapped: RawSimulatePayload = serde_json::from_str(input).unwrap();
        assert_eq!(serde_json::to_string(&wrapped).unwrap(), input);

        let forwarded = serde_json::to_value(wrapped.parse(1, 0).unwrap()).unwrap();
        let expected = SimulatePayload::<TransactionRequest> {
            block_state_calls: vec![SimBlock::default()],
            validation: true,
            ..Default::default()
        };
        assert_eq!(forwarded, serde_json::to_value(expected).unwrap());
    }

    #[test]
    fn rejects_trailing_input() {
        let input = r#"{"blockStateCalls":[]} {}"#;
        let err = parse_str::<TransactionRequest>(input, 1, 1).unwrap_err();
        assert_eq!(err.to_string(), "trailing characters at line 1 column 24");
        assert_matches!(err, RawSimulatePayloadError::Json(_));
        assert_eq!(
            serde_json::from_str::<RawSimulatePayload>(input).unwrap_err().to_string(),
            "trailing characters at line 1 column 24"
        );
    }

    #[test]
    fn reports_invalid_payload_as_json_error() {
        let err = parse(r#"{"blockStateCalls":{}}"#, 1).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid type: map, expected an array of simulation blocks at line 1 column 19"
        );
        assert_matches!(err, RawSimulatePayloadError::Json(_));
    }

    #[test]
    fn applies_limits_to_parsed_payload() {
        let payload = SimulatePayload::<TransactionRequest>::default()
            .extend(SimBlock::default().call(Default::default()))
            .extend(SimBlock::default().call(Default::default()));
        let wrapped = RawSimulatePayload::from(payload);
        assert_matches!(
            wrapped.clone().parse(1, 2),
            Err(RawSimulatePayloadError::LimitExceeded(SimulatePayloadLimitExceeded::Blocks))
        );
        assert_matches!(
            wrapped.clone().parse(2, 1),
            Err(RawSimulatePayloadError::LimitExceeded(SimulatePayloadLimitExceeded::Calls))
        );
        assert_eq!(wrapped.parse(2, 2).unwrap().block_state_calls.len(), 2);
    }
}
