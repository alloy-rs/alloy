//! Deserialization of simulation payloads with caller-supplied allocation limits.

use super::{SimBlock, SimulatePayload};
use alloc::{string::String, vec::Vec};
use core::{fmt, marker::PhantomData};
use serde::{
    de::{DeserializeSeed, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

/// The simulation request limit exceeded during deserialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulatePayloadLimitExceeded {
    /// The request contained more explicit blocks than allowed.
    Blocks,
    /// The request contained more calls across its explicit blocks than allowed.
    Calls,
}

impl fmt::Display for SimulatePayloadLimitExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocks => formatter.write_str("too many blocks in simulation payload"),
            Self::Calls => formatter.write_str("too many calls in simulation payload"),
        }
    }
}

impl core::error::Error for SimulatePayloadLimitExceeded {}

/// A deserialization seed that bounds the number of explicit blocks and calls in a simulation.
///
/// Limits are supplied by the caller instead of being imposed on the ordinary
/// [`SimulatePayload`] deserializer. Excess elements are consumed without deserializing
/// [`SimBlock`] or `TxReq`, and the result identifies which limit was exceeded. The limits do
/// not bound the size of a single block override or transaction request.
///
/// # Example
///
/// ```
/// use alloy_rpc_types_eth::{simulate::SimulatePayloadSeed, TransactionRequest};
/// use serde::de::DeserializeSeed;
///
/// let input = r#"{"blockStateCalls":[{"calls":[{}]}]}"#;
/// let mut deserializer = serde_json::Deserializer::from_str(input);
/// let payload = SimulatePayloadSeed::<TransactionRequest>::new(256, 4096)
///     .deserialize(&mut deserializer)??;
/// deserializer.end()?;
/// assert_eq!(payload.block_state_calls[0].calls.len(), 1);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SimulatePayloadSeed<TxReq> {
    max_blocks: usize,
    max_calls: usize,
    marker: PhantomData<TxReq>,
}

impl<TxReq> SimulatePayloadSeed<TxReq> {
    /// Creates a seed with limits for explicit blocks and calls across all blocks.
    pub const fn new(max_blocks: usize, max_calls: usize) -> Self {
        Self { max_blocks, max_calls, marker: PhantomData }
    }
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for SimulatePayloadSeed<TxReq> {
    type Value = Result<SimulatePayload<TxReq>, SimulatePayloadLimitExceeded>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        const FIELDS: &[&str] =
            &["blockStateCalls", "traceTransfers", "validation", "returnFullTransactions"];
        deserializer.deserialize_struct(
            "SimulatePayload",
            FIELDS,
            PayloadVisitor {
                max_blocks: self.max_blocks,
                max_calls: self.max_calls,
                marker: self.marker,
            },
        )
    }
}

struct PayloadVisitor<TxReq> {
    max_blocks: usize,
    max_calls: usize,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for PayloadVisitor<TxReq> {
    type Value = Result<SimulatePayload<TxReq>, SimulatePayloadLimitExceeded>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a simulation payload")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut blocks = None;
        let mut trace_transfers = None;
        let mut validation = None;
        let mut return_full_transactions = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "blockStateCalls" => {
                    if blocks.is_some() {
                        return Err(A::Error::duplicate_field("blockStateCalls"));
                    }
                    blocks = Some(map.next_value_seed(BlocksSeed {
                        max_blocks: self.max_blocks,
                        max_calls: self.max_calls,
                        marker: self.marker,
                    })?);
                }
                "traceTransfers" => {
                    if trace_transfers.is_some() {
                        return Err(A::Error::duplicate_field("traceTransfers"));
                    }
                    trace_transfers = Some(map.next_value()?);
                }
                "validation" => {
                    if validation.is_some() {
                        return Err(A::Error::duplicate_field("validation"));
                    }
                    validation = Some(map.next_value()?);
                }
                "returnFullTransactions" => {
                    if return_full_transactions.is_some() {
                        return Err(A::Error::duplicate_field("returnFullTransactions"));
                    }
                    return_full_transactions = Some(map.next_value()?);
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        Ok(build_result(
            blocks.unwrap_or_default(),
            trace_transfers.unwrap_or_default(),
            validation.unwrap_or_default(),
            return_full_transactions.unwrap_or_default(),
        ))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let blocks = seq
            .next_element_seed(BlocksSeed {
                max_blocks: self.max_blocks,
                max_calls: self.max_calls,
                marker: self.marker,
            })?
            .unwrap_or_default();
        let trace_transfers = seq.next_element()?.unwrap_or_default();
        let validation = seq.next_element()?.unwrap_or_default();
        let return_full_transactions = seq.next_element()?.unwrap_or_default();
        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::invalid_length(5, &self));
        }
        Ok(build_result(blocks, trace_transfers, validation, return_full_transactions))
    }
}

fn build_result<TxReq>(
    (block_state_calls, exceeded): (Vec<SimBlock<TxReq>>, Option<SimulatePayloadLimitExceeded>),
    trace_transfers: bool,
    validation: bool,
    return_full_transactions: bool,
) -> Result<SimulatePayload<TxReq>, SimulatePayloadLimitExceeded> {
    let payload = SimulatePayload {
        block_state_calls,
        trace_transfers,
        validation,
        return_full_transactions,
    };
    match exceeded {
        Some(limit) => Err(limit),
        None => Ok(payload),
    }
}

struct BlocksSeed<TxReq> {
    max_blocks: usize,
    max_calls: usize,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for BlocksSeed<TxReq> {
    type Value = (Vec<SimBlock<TxReq>>, Option<SimulatePayloadLimitExceeded>);

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(BlocksVisitor {
            max_blocks: self.max_blocks,
            max_calls: self.max_calls,
            marker: self.marker,
        })
    }
}

struct BlocksVisitor<TxReq> {
    max_blocks: usize,
    max_calls: usize,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for BlocksVisitor<TxReq> {
    type Value = (Vec<SimBlock<TxReq>>, Option<SimulatePayloadLimitExceeded>);

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array of simulation blocks")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut blocks = Vec::new();
        let mut remaining_calls = self.max_calls;
        let mut too_many_blocks = false;
        let mut too_many_calls = false;
        while let Some(block) = seq.next_element_seed(BlockSeed {
            allowed: blocks.len() < self.max_blocks,
            remaining_calls: &mut remaining_calls,
            too_many_blocks: &mut too_many_blocks,
            too_many_calls: &mut too_many_calls,
            marker: self.marker,
        })? {
            if let Some(block) = block {
                blocks.push(block);
            }
        }
        let exceeded = if too_many_blocks {
            Some(SimulatePayloadLimitExceeded::Blocks)
        } else if too_many_calls {
            Some(SimulatePayloadLimitExceeded::Calls)
        } else {
            None
        };
        Ok((blocks, exceeded))
    }
}

struct BlockSeed<'a, TxReq> {
    allowed: bool,
    remaining_calls: &'a mut usize,
    too_many_blocks: &'a mut bool,
    too_many_calls: &'a mut bool,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for BlockSeed<'_, TxReq> {
    type Value = Option<SimBlock<TxReq>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        if !self.allowed {
            *self.too_many_blocks = true;
            IgnoredAny::deserialize(deserializer)?;
            return Ok(None);
        }
        deserializer
            .deserialize_struct(
                "SimBlock",
                &["blockOverrides", "stateOverrides", "calls"],
                BlockVisitor {
                    remaining_calls: self.remaining_calls,
                    too_many_calls: self.too_many_calls,
                    marker: self.marker,
                },
            )
            .map(Some)
    }
}

struct BlockVisitor<'a, TxReq> {
    remaining_calls: &'a mut usize,
    too_many_calls: &'a mut bool,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for BlockVisitor<'_, TxReq> {
    type Value = SimBlock<TxReq>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a simulation block")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut block_overrides = None;
        let mut state_overrides = None;
        let mut calls = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "blockOverrides" => {
                    if block_overrides.is_some() {
                        return Err(A::Error::duplicate_field("blockOverrides"));
                    }
                    block_overrides = Some(map.next_value()?);
                }
                "stateOverrides" => {
                    if state_overrides.is_some() {
                        return Err(A::Error::duplicate_field("stateOverrides"));
                    }
                    state_overrides = Some(map.next_value()?);
                }
                "calls" => {
                    if calls.is_some() {
                        return Err(A::Error::duplicate_field("calls"));
                    }
                    calls = Some(map.next_value_seed(CallsSeed {
                        remaining_calls: self.remaining_calls,
                        too_many_calls: self.too_many_calls,
                        marker: self.marker,
                    })?);
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        Ok(SimBlock {
            block_overrides: block_overrides.unwrap_or(None),
            state_overrides: state_overrides.unwrap_or(None),
            calls: calls.unwrap_or_default(),
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let block_overrides = seq.next_element()?.unwrap_or_default();
        let state_overrides = seq.next_element()?.unwrap_or_default();
        let calls = seq
            .next_element_seed(CallsSeed {
                remaining_calls: self.remaining_calls,
                too_many_calls: self.too_many_calls,
                marker: self.marker,
            })?
            .unwrap_or_default();
        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::invalid_length(4, &self));
        }
        Ok(SimBlock { block_overrides, state_overrides, calls })
    }
}

struct CallsSeed<'a, TxReq> {
    remaining_calls: &'a mut usize,
    too_many_calls: &'a mut bool,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for CallsSeed<'_, TxReq> {
    type Value = Vec<TxReq>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(CallsVisitor {
            remaining_calls: self.remaining_calls,
            too_many_calls: self.too_many_calls,
            marker: self.marker,
        })
    }
}

struct CallsVisitor<'a, TxReq> {
    remaining_calls: &'a mut usize,
    too_many_calls: &'a mut bool,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for CallsVisitor<'_, TxReq> {
    type Value = Vec<TxReq>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array of calls")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut calls = Vec::new();
        while let Some(call) = seq.next_element_seed(CallSeed {
            remaining_calls: self.remaining_calls,
            too_many_calls: self.too_many_calls,
            marker: self.marker,
        })? {
            if let Some(call) = call {
                calls.push(call);
            }
        }
        Ok(calls)
    }
}

struct CallSeed<'a, TxReq> {
    remaining_calls: &'a mut usize,
    too_many_calls: &'a mut bool,
    marker: PhantomData<TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for CallSeed<'_, TxReq> {
    type Value = Option<TxReq>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        if *self.remaining_calls == 0 {
            *self.too_many_calls = true;
            IgnoredAny::deserialize(deserializer)?;
            return Ok(None);
        }
        *self.remaining_calls -= 1;
        TxReq::deserialize(deserializer).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionRequest;

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
        max_calls: usize,
    ) -> Result<
        Result<SimulatePayload<TransactionRequest>, SimulatePayloadLimitExceeded>,
        serde_json::Error,
    > {
        let mut deserializer = serde_json::Deserializer::from_str(input);
        let result = SimulatePayloadSeed::<TransactionRequest>::new(max_blocks, max_calls)
            .deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(result)
    }

    #[test]
    fn accepts_exact_limits() {
        let parsed = parse(&payload_with_calls(&[2, 1]), 2, 3).unwrap().unwrap();
        assert_eq!(parsed.block_state_calls.len(), 2);
        assert_eq!(parsed.block_state_calls[0].calls.len(), 2);
        assert_eq!(parsed.block_state_calls[1].calls.len(), 1);
    }

    #[test]
    fn rejects_calls_across_blocks() {
        let result = parse(&payload_with_calls(&[2, 2]), 2, 3).unwrap();
        assert!(matches!(result, Err(SimulatePayloadLimitExceeded::Calls)));
    }

    #[test]
    fn skips_excess_call_without_deserializing_transaction() {
        let input = r#"{"blockStateCalls":[{"calls":[{},123]}]}"#;
        let result = parse(input, 1, 1).unwrap();
        assert!(matches!(result, Err(SimulatePayloadLimitExceeded::Calls)));
    }

    #[test]
    fn rejects_excess_blocks() {
        let result = parse(&payload_with_calls(&[0, 0, 0]), 2, 3).unwrap();
        assert!(matches!(result, Err(SimulatePayloadLimitExceeded::Blocks)));
    }

    #[test]
    fn skips_excess_block_without_deserializing_it() {
        let input = r#"{"blockStateCalls":[{"calls":[]},123]}"#;
        let result = parse(input, 1, 1).unwrap();
        assert!(matches!(result, Err(SimulatePayloadLimitExceeded::Blocks)));
    }

    #[test]
    fn preserves_regular_payload_fields() {
        let input = r#"{"blockStateCalls":[{"blockOverrides":null,"stateOverrides":null,"calls":[{}],"other":true}],"traceTransfers":true,"validation":true,"returnFullTransactions":true}"#;
        let expected = serde_json::from_str::<SimulatePayload<TransactionRequest>>(input).unwrap();
        let parsed = parse(input, 1, 1).unwrap().unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), serde_json::to_value(expected).unwrap());
    }

    #[test]
    fn rejects_duplicate_calls_field() {
        let input = r#"{"blockStateCalls":[{"calls":[],"calls":[]}]}"#;
        assert!(parse(input, 1, 1).unwrap_err().to_string().contains("duplicate field `calls`"));
    }

    #[test]
    fn regular_deserialization_has_no_added_limits() {
        let input = payload_with_calls(&[3]);
        let parsed = serde_json::from_str::<SimulatePayload<TransactionRequest>>(&input).unwrap();
        assert_eq!(parsed.block_state_calls[0].calls.len(), 3);
        assert!(matches!(parse(&input, 1, 2).unwrap(), Err(SimulatePayloadLimitExceeded::Calls)));
    }
}
