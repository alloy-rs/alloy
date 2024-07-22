//! `trace_filter` types and support
use crate::parity::{
    Action, CallAction, CreateAction, CreateOutput, RewardAction, SelfdestructAction, TraceOutput,
    TransactionTrace,
};
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Trace filter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TraceFilter {
    /// From block
    #[serde(with = "alloy_serde::quantity::opt")]
    pub from_block: Option<u64>,
    /// To block
    #[serde(with = "alloy_serde::quantity::opt")]
    pub to_block: Option<u64>,
    /// From address
    #[serde(default)]
    pub from_address: Vec<Address>,
    /// To address
    #[serde(default)]
    pub to_address: Vec<Address>,
    /// How to apply `from_address` and `to_address` filters.
    #[serde(default)]
    pub mode: TraceFilterMode,
    /// Output offset
    pub after: Option<u64>,
    /// Output amount
    pub count: Option<u64>,
}

// === impl TraceFilter ===

impl TraceFilter {
    /// Sets the `from_block` field of the struct
    pub const fn from_block(mut self, block: u64) -> Self {
        self.from_block = Some(block);
        self
    }

    /// Sets the `to_block` field of the struct
    pub const fn to_block(mut self, block: u64) -> Self {
        self.to_block = Some(block);
        self
    }

    /// Sets the `from_address` field of the struct
    pub fn from_address(mut self, addresses: Vec<Address>) -> Self {
        self.from_address = addresses;
        self
    }

    /// Sets the `to_address` field of the struct
    pub fn to_address(mut self, addresses: Vec<Address>) -> Self {
        self.to_address = addresses;
        self
    }

    /// Sets the `after` field of the struct
    pub const fn after(mut self, after: u64) -> Self {
        self.after = Some(after);
        self
    }

    /// Sets the `count` field of the struct
    pub const fn count(mut self, count: u64) -> Self {
        self.count = Some(count);
        self
    }

    /// Sets the `from_address` field of the struct
    pub const fn mode(mut self, mode: TraceFilterMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns a `TraceFilterMatcher` for this filter.
    pub fn matcher(&self) -> TraceFilterMatcher {
        let from_addresses = self.from_address.iter().cloned().collect();
        let to_addresses = self.to_address.iter().cloned().collect();
        TraceFilterMatcher { mode: self.mode, from_addresses, to_addresses }
    }
}

/// How to apply `from_address` and `to_address` filters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TraceFilterMode {
    /// Return traces for transactions with matching `from` OR `to` addresses.
    #[default]
    Union,
    /// Only return traces for transactions with matching `from` _and_ `to` addresses.
    Intersection,
}

/// Address filter.
/// This is a set of addresses to match against.
/// An empty set matches all addresses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AddressFilter(HashSet<Address>);

impl AddressFilter {
    /// Creates a new `AddressFilter` from a list of addresses.
    pub fn new(addresses: Vec<Address>) -> Self {
        Self(addresses.into_iter().collect())
    }

    /// Returns `true` if the given address is in the filter.
    pub fn matches(&self, addr: &Address) -> bool {
        self.0.is_empty() || self.0.contains(addr)
    }

    /// Returns `true` if the filter is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Helper type for matching `from` and `to` addresses. Empty sets match all addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceFilterMatcher {
    mode: TraceFilterMode,
    from_addresses: AddressFilter,
    to_addresses: AddressFilter,
}

impl TraceFilterMatcher {
    /// Returns `true` if the given `TransactionTrace` matches this filter.
    pub fn matches(&self, trace: &TransactionTrace) -> bool {
        let (from_matches, to_matches) = match trace.action {
            Action::Call(CallAction { from, to, .. }) => {
                (self.from_addresses.matches(&from), self.to_addresses.matches(&to))
            }
            Action::Create(CreateAction { from, .. }) => (
                self.from_addresses.matches(&from),
                match trace.result {
                    Some(TraceOutput::Create(CreateOutput { address: to, .. })) => {
                        self.to_addresses.matches(&to)
                    }
                    _ => self.to_addresses.is_empty(),
                },
            ),
            Action::Selfdestruct(SelfdestructAction { address, refund_address, .. }) => {
                (self.from_addresses.matches(&address), self.to_addresses.matches(&refund_address))
            }
            Action::Reward(RewardAction { author, .. }) => {
                (self.from_addresses.matches(&author), self.to_addresses.is_empty())
            }
        };

        match self.mode {
            TraceFilterMode::Union => from_matches || to_matches,
            TraceFilterMode::Intersection => from_matches && to_matches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::RewardType;
    use alloy_primitives::{Address, Bytes, TxHash, B256, U256, U64};
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_parse_filter() {
        let s = r#"{"fromBlock":  "0x3","toBlock":  "0x5"}"#;
        let filter: TraceFilter = serde_json::from_str(s).unwrap();
        assert_eq!(filter.from_block, Some(3));
        assert_eq!(filter.to_block, Some(5));
    }

    fn address(addr: &str) -> Address {
        addr.parse().unwrap()
    }

    fn create_trace(action: Action, result: Option<TraceOutput>) -> TransactionTrace {
        TransactionTrace { action, result, subtraces: 1, trace_address: vec![], error: None }
    }

    #[test]
    fn test_matches_call_action_union_mode() {
        let matcher = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: AddressFilter::new(vec![address(
                "0xc77820eef59629fc8d88154977bc8de8a1b2f4ae",
            )]),
            to_addresses: AddressFilter::new(vec![address(
                "0x4f4495243837681061c4743b74b3eedf548d56a5",
            )]),
        };

        let trace = create_trace(
            Action::Call(CallAction {
                from: address("0xc77820eef59629fc8d88154977bc8de8a1b2f4ae"),
                call_type: "call".to_string(),
                gas: U256::from_str("0x4a0d00").unwrap(),
                input: vec![0x12],
                to: address("0x4f4495243837681061c4743b74b3eedf548d56a5"),
                value: U256::zero(),
            }),
            Some(TraceOutput::Call(CallOutput {
                gas_used: U256::from_str("0x17d337").unwrap(),
                output: Bytes::new(),
            })),
        );

        assert!(matcher.matches(&trace));
    }

    #[test]
    fn test_matches_create_action_intersection_mode() {
        let matcher = TraceFilterMatcher {
            mode: TraceFilterMode::Intersection,
            from_addresses: AddressFilter::new(vec![address(
                "0xc77820eef59629fc8d88154977bc8de8a1b2f4ae",
            )]),
            to_addresses: AddressFilter::new(vec![address(
                "0x4f4495243837681061c4743b74b3eedf548d56a5",
            )]),
        };

        let trace = create_trace(
            Action::Create(CreateAction {
                from: address("0xc77820eef59629fc8d88154977bc8de8a1b2f4ae"),
                gas: U256::from_str("0x4a0d00").unwrap(),
                init: vec![],
                value: U256::zero(),
            }),
            Some(TraceOutput::Create(CreateOutput {
                code: Bytes::new(),
                address: address("0x4f4495243837681061c4743b74b3eedf548d56a5"),
                gas_used: 0,
            })),
        );

        assert!(matcher.matches(&trace));
    }

    #[test]
    fn test_matches_reward_action_no_matches() {
        let matcher = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: AddressFilter::new(vec![address(
                "0xc77820eef59629fc8d88154977bc8de8a1b2f4ae",
            )]),
            to_addresses: AddressFilter::new(vec![address(
                "0x4f4495243837681061c4743b74b3eedf548d56a5",
            )]),
        };

        let trace = create_trace(
            Action::Reward(RewardAction {
                reward_type: RewardType::Block,
                author: address("0x1234567890123456789012345678901234567890"),
                value: U256::from_str("0x1").unwrap(),
            }),
            None,
        );

        assert!(!matcher.matches(&trace));
    }

    #[test]
    fn test_matches_selfdestruct_action_partial_match() {
        let matcher = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: AddressFilter::new(vec![address(
                "0xc77820eef59629fc8d88154977bc8de8a1b2f4ae",
            )]),
            to_addresses: AddressFilter::new(vec![]),
        };

        let trace = create_trace(
            Action::Selfdestruct(SelfdestructAction {
                address: address("0xc77820eef59629fc8d88154977bc8de8a1b2f4ae"),
                refund_address: address("0x4f4495243837681061c4743b74b3eedf548d56a5"),
                balance: U256::from_str("0x1").unwrap(),
            }),
            None,
        );

        assert!(matcher.matches(&trace));
    }

    #[test]
    fn test_matches_selfdestruct_action_intersection_no_match() {
        let matcher = TraceFilterMatcher {
            mode: TraceFilterMode::Intersection,
            from_addresses: AddressFilter::new(vec![address(
                "0xc77820eef59629fc8d88154977bc8de8a1b2f4ae",
            )]),
            to_addresses: AddressFilter::new(vec![address(
                "0x4f4495243837681061c4743b74b3eedf548d56a5",
            )]),
        };

        let trace = create_trace(
            Action::Selfdestruct(SelfdestructAction {
                address: address("0xc77820eef59629fc8d88154977bc8de8a1b2f4ae"),
                refund_address: address("0x1234567890123456789012345678901234567890"),
                balance: U256::from_str("0x1").unwrap(),
            }),
            None,
        );

        assert!(!matcher.matches(&trace));
    }
}
