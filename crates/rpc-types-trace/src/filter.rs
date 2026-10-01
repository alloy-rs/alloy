//! `trace_filter` types and support.

use crate::parity::{
    Action, CallAction, CreateAction, CreateOutput, RewardAction, SelfdestructAction, TraceOutput,
    TransactionTrace,
};
use alloy_primitives::{map::AddressHashSet, Address, BlockHash};
use alloy_rpc_types_eth::FilterBlockOption;
use serde::{Deserialize, Serialize};

/// Trace filter.
///
/// Selects blocks either by the `from_block`..=`to_block` range or by `block_hash`, which selects
/// exactly one block. See [`TraceFilter::block_option`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TraceFilter {
    /// From block
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub from_block: Option<u64>,
    /// To block
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub to_block: Option<u64>,
    /// From address
    #[serde(default, deserialize_with = "alloy_serde::null_as_default")]
    pub from_address: Vec<Address>,
    /// To address
    #[serde(default, deserialize_with = "alloy_serde::null_as_default")]
    pub to_address: Vec<Address>,
    /// How to apply `from_address` and `to_address` filters. Defaults to intersection.
    #[serde(default, deserialize_with = "alloy_serde::null_as_default")]
    pub mode: TraceFilterMode,
    /// Output offset
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<u64>,
    /// Output amount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    /// Selects exactly the block with this hash, like `blockHash` in `eth_getLogs` (EIP-234).
    ///
    /// Mutually exclusive with `from_block` and `to_block`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
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

    /// Sets the `mode` field of the struct
    pub const fn mode(mut self, mode: TraceFilterMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the `block_hash` field of the struct
    pub const fn block_hash(mut self, block_hash: BlockHash) -> Self {
        self.block_hash = Some(block_hash);
        self
    }

    /// Returns the blocks this filter selects.
    ///
    /// Returns an error if `block_hash` is combined with `from_block` or `to_block`.
    pub const fn block_option(&self) -> Result<TraceFilterBlockOption, TraceFilterBlockConflict> {
        match (self.block_hash, self.from_block, self.to_block) {
            (Some(hash), None, None) => Ok(TraceFilterBlockOption::AtBlockHash(hash)),
            (Some(_), _, _) => Err(TraceFilterBlockConflict),
            (None, from_block, to_block) => {
                Ok(TraceFilterBlockOption::Range { from_block, to_block })
            }
        }
    }

    /// Returns a `TraceFilterMatcher` for this filter.
    pub fn matcher(&self) -> TraceFilterMatcher {
        let from_addresses = self.from_address.iter().copied().collect();
        let to_addresses = self.to_address.iter().copied().collect();
        TraceFilterMatcher { mode: self.mode, from_addresses, to_addresses }
    }
}

/// The blocks selected by a [`TraceFilter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceFilterBlockOption {
    /// Blocks from `from_block` to `to_block`, inclusive.
    Range {
        /// From block
        from_block: Option<u64>,
        /// To block
        to_block: Option<u64>,
    },
    /// Exactly the block with this hash.
    AtBlockHash(BlockHash),
}

impl From<TraceFilterBlockOption> for FilterBlockOption {
    fn from(option: TraceFilterBlockOption) -> Self {
        match option {
            TraceFilterBlockOption::Range { from_block, to_block } => Self::Range {
                from_block: from_block.map(Into::into),
                to_block: to_block.map(Into::into),
            },
            TraceFilterBlockOption::AtBlockHash(hash) => Self::AtBlockHash(hash),
        }
    }
}

/// Error returned by [`TraceFilter::block_option`] when `block_hash` is combined with
/// `from_block` or `to_block`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, thiserror::Error)]
#[error("cannot specify both blockHash and fromBlock/toBlock, choose one or the other")]
pub struct TraceFilterBlockConflict;

/// How to apply `from_address` and `to_address` filters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TraceFilterMode {
    /// Return traces for transactions with matching `from` OR `to` addresses.
    Union,
    /// Only return traces for transactions with matching `from` _and_ `to` addresses.
    #[default]
    Intersection,
}

/// Address filter.
/// This is a set of addresses to match against.
/// An empty set matches all addresses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AddressFilter(pub AddressHashSet);

impl FromIterator<Address> for AddressFilter {
    fn from_iter<I: IntoIterator<Item = Address>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<Vec<Address>> for AddressFilter {
    fn from(addrs: Vec<Address>) -> Self {
        Self::from_iter(addrs)
    }
}

impl AddressFilter {
    /// Returns `true` if the given address is in the filter or the filter address set is empty.
    pub fn matches(&self, addr: &Address) -> bool {
        self.is_empty() || self.0.contains(addr)
    }

    /// Returns `true` if the address set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// `TraceFilterMatcher` is a filter used for matching `TransactionTrace` based on it's action and
/// result (if available).
///
/// It allows filtering traces by their mode, from address set, and to address set, and empty
/// address set means match all addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceFilterMatcher {
    mode: TraceFilterMode,
    from_addresses: AddressFilter,
    to_addresses: AddressFilter,
}

impl TraceFilterMatcher {
    /// Returns `true` if the given `TransactionTrace` matches this filter.
    ///
    /// # Arguments
    ///
    /// - `trace`: A reference to a `TransactionTrace` to be evaluated against the filter.
    ///
    /// # Returns
    ///
    /// - `true` if the transaction trace matches the filter criteria; otherwise, `false`.
    ///
    /// # Behavior
    ///
    /// This function evaluates whether the `trace` matches based on its action type:
    /// - `Call`: Matches if either the `from` or `to` addresses in the call action match the
    ///   filter's address criteria.
    /// - `Create`: Matches if the `from` address in action matches, and the result's address (if
    ///   available) matches the filter's address criteria.
    /// - `Selfdestruct`: Matches if the `address` and `refund_address` matches the filter's address
    ///   criteria.
    /// - `Reward`: Matches if the `author` address matches the filter's `to_addresses` criteria.
    ///
    /// The overall result depends on the filter mode:
    /// - `Union` mode: The trace matches if either the `from` or `to` address matches. If either of
    ///   the from or to address set is empty, the trace matches only if the other address matches,
    ///   and if both are empty, the filter matches all traces.
    /// - `Intersection` mode: The trace matches only if both the `from` and `to` addresses match.
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
                (self.from_addresses.is_empty(), self.to_addresses.matches(&author))
            }
        };

        match self.mode {
            TraceFilterMode::Union => {
                if self.from_addresses.is_empty() {
                    to_matches
                } else if self.to_addresses.is_empty() {
                    from_matches
                } else {
                    from_matches || to_matches
                }
            }
            TraceFilterMode::Intersection => from_matches && to_matches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, B256, U256};
    use serde_json::json;
    use similar_asserts::assert_eq;

    #[test]
    fn test_parse_filter() {
        let s = r#"{"fromBlock":  "0x3","toBlock":  "0x5"}"#;
        let filter: TraceFilter = serde_json::from_str(s).unwrap();
        assert_eq!(filter.from_block, Some(3));
        assert_eq!(filter.to_block, Some(5));
    }

    #[test]
    fn default_filter_intersects_address_lists() {
        let from = Address::with_last_byte(1);
        let to = Address::with_last_byte(2);
        let other = Address::with_last_byte(3);
        let filter: TraceFilter = serde_json::from_value(json!({
            "fromAddress": [from, other],
            "toAddress": [to, other],
        }))
        .unwrap();
        assert_eq!(filter.mode, TraceFilterMode::Intersection);
        assert_eq!(TraceFilter::default().mode, TraceFilterMode::Intersection);

        for (sender, recipient, expected) in [
            (from, to, true),
            (other, other, true),
            (from, from, false),
            (to, to, false),
            (to, from, false),
        ] {
            let trace = TransactionTrace {
                action: Action::Call(CallAction {
                    from: sender,
                    to: recipient,
                    ..Default::default()
                }),
                ..Default::default()
            };
            assert_eq!(filter.matcher().matches(&trace), expected);
        }
    }

    #[test]
    fn filter_mode_is_explicit_and_validated() {
        let from = Address::with_last_byte(1);
        let to = Address::with_last_byte(2);
        let trace = TransactionTrace {
            action: Action::Call(CallAction { from, to: from, ..Default::default() }),
            ..Default::default()
        };
        for (mode, expected) in [("union", true), ("intersection", false)] {
            let filter: TraceFilter = serde_json::from_value(json!({
                "fromAddress": [from], "toAddress": [to], "mode": mode,
            }))
            .unwrap();
            assert_eq!(filter.matcher().matches(&trace), expected);
        }
        assert!(serde_json::from_value::<TraceFilter>(json!({ "mode": "unknown" })).is_err());
    }

    #[test]
    fn null_members_are_omitted() {
        let omitted = TraceFilter::default().from_block(2).to_block(5);
        let filter: TraceFilter = serde_json::from_value(json!({
            "fromBlock": "0x2",
            "toBlock": "0x5",
            "fromAddress": null,
            "toAddress": null,
            "mode": null,
            "after": null,
            "count": null,
            "blockHash": null,
        }))
        .unwrap();
        assert_eq!(filter, omitted);

        for member in [
            "fromBlock",
            "toBlock",
            "fromAddress",
            "toAddress",
            "mode",
            "after",
            "count",
            "blockHash",
        ] {
            let filter: TraceFilter = serde_json::from_value(json!({ member: null })).unwrap();
            assert_eq!(filter, TraceFilter::default(), "{member}");
        }
    }

    #[test]
    fn invalid_members_are_rejected() {
        for filter in [
            json!({ "mode": "unknown" }),
            json!({ "mode": 1 }),
            json!({ "fromAddress": Address::ZERO }),
            json!({ "toAddress": [null] }),
            json!({ "unknown": null }),
            json!({ "fromAddress": null, "unknown": null }),
            json!({ "blockHash": "0x1234" }),
            json!({ "blockHash": { "blockHash": B256::ZERO } }),
            json!({ "blockHash": 1 }),
        ] {
            assert!(serde_json::from_value::<TraceFilter>(filter.clone()).is_err(), "{filter}");
        }
    }

    #[test]
    fn block_hash_selects_block() {
        let hash = B256::with_last_byte(0xab);
        let filter: TraceFilter = serde_json::from_value(json!({
            "blockHash": hash,
            "fromAddress": [Address::ZERO],
            "after": 1,
            "count": 2,
        }))
        .unwrap();
        assert_eq!(
            filter,
            TraceFilter::default()
                .block_hash(hash)
                .from_address(vec![Address::ZERO])
                .after(1)
                .count(2)
        );
        assert_eq!(filter.block_option(), Ok(TraceFilterBlockOption::AtBlockHash(hash)));

        let json = serde_json::to_value(TraceFilter::default().block_hash(hash)).unwrap();
        assert_eq!(json["blockHash"], json!(hash));
        assert_eq!(serde_json::from_value::<TraceFilter>(json).unwrap().block_hash, Some(hash));
        assert!(serde_json::to_value(TraceFilter::default()).unwrap().get("blockHash").is_none());
    }

    #[test]
    fn block_hash_excludes_range_bounds() {
        let hash = B256::with_last_byte(0xab);
        for filter in [
            json!({ "blockHash": hash, "fromBlock": "0x2" }),
            json!({ "blockHash": hash, "toBlock": "0x2" }),
            json!({ "blockHash": hash, "fromBlock": "0x2", "toBlock": "0x2" }),
        ] {
            let filter: TraceFilter = serde_json::from_value(filter).unwrap();
            assert_eq!(filter.block_option(), Err(TraceFilterBlockConflict), "{filter:?}");
        }

        for (filter, expected) in [
            (
                json!({ "blockHash": hash, "fromBlock": null, "toBlock": null }),
                TraceFilterBlockOption::AtBlockHash(hash),
            ),
            (
                json!({ "blockHash": null, "fromBlock": "0x2", "toBlock": "0x5" }),
                TraceFilterBlockOption::Range { from_block: Some(2), to_block: Some(5) },
            ),
            (json!({}), TraceFilterBlockOption::Range { from_block: None, to_block: None }),
        ] {
            let filter: TraceFilter = serde_json::from_value(filter).unwrap();
            assert_eq!(filter.block_option(), Ok(expected));
        }
    }

    #[test]
    fn block_option_converts_to_filter_block_option() {
        let hash = B256::with_last_byte(0xab);
        assert_eq!(
            FilterBlockOption::from(TraceFilterBlockOption::AtBlockHash(hash)),
            FilterBlockOption::AtBlockHash(hash)
        );
        assert_eq!(
            FilterBlockOption::from(TraceFilterBlockOption::Range {
                from_block: Some(2),
                to_block: None
            }),
            FilterBlockOption::Range { from_block: Some(2u64.into()), to_block: None }
        );
    }

    #[test]
    fn duplicate_block_selectors_are_rejected() {
        let hash = format!("\"{}\"", B256::with_last_byte(0xab));
        for (member, value) in
            [("blockHash", hash.as_str()), ("fromBlock", "\"0x1\""), ("toBlock", "\"0x1\"")]
        {
            for (first, second) in [(value, value), ("null", value), (value, "null")] {
                let filter = format!(r#"{{"{member}":{first},"{member}":{second}}}"#);
                assert!(serde_json::from_str::<TraceFilter>(&filter).is_err(), "{filter}");
            }
        }
    }

    #[test]
    fn null_mode_intersects_address_lists() {
        let from = Address::with_last_byte(1);
        let to = Address::with_last_byte(2);
        let filter: TraceFilter = serde_json::from_value(json!({
            "fromAddress": [from], "toAddress": [to], "mode": null,
        }))
        .unwrap();
        assert_eq!(filter.mode, TraceFilterMode::Intersection);

        let trace = TransactionTrace {
            action: Action::Call(CallAction { from, to: from, ..Default::default() }),
            ..Default::default()
        };
        assert!(!filter.matcher().matches(&trace));
    }

    #[test]
    fn default_filter_empty_lists_are_unconstrained() {
        let from = Address::with_last_byte(1);
        let to = Address::with_last_byte(2);
        let trace = TransactionTrace {
            action: Action::Call(CallAction { from, to, ..Default::default() }),
            ..Default::default()
        };
        for (filter, expected) in [
            (json!({}), true),
            (json!({ "fromAddress": [], "toAddress": [] }), true),
            (json!({ "fromAddress": [], "toAddress": [to] }), true),
            (json!({ "fromAddress": [from], "toAddress": [] }), true),
            (json!({ "fromAddress": [], "toAddress": [from] }), false),
            (json!({ "fromAddress": [to], "toAddress": [] }), false),
            (json!({ "fromAddress": null, "toAddress": null }), true),
            (json!({ "fromAddress": null, "toAddress": [to] }), true),
            (json!({ "fromAddress": [from], "toAddress": null }), true),
            (json!({ "fromAddress": null, "toAddress": [from] }), false),
            (json!({ "fromAddress": [to], "toAddress": null }), false),
        ] {
            let filter: TraceFilter = serde_json::from_value(filter).unwrap();
            assert_eq!(filter.matcher().matches(&trace), expected);
        }
    }

    #[test]
    fn test_filter_matcher_addresses_unspecified() {
        let filter_json = json!({ "fromBlock": "0x3", "toBlock": "0x5" });
        let matcher = serde_json::from_value::<TraceFilter>(filter_json).unwrap().matcher();
        let s = r#"{
            "action": {
                "from": "0x66e29f0b6b1b07071f2fde4345d512386cb66f5f",
                "callType": "call",
                "gas": "0x10bfc",
                "input": "0x",
                "to": "0x160f5f00288e9e1cc8655b327e081566e580a71d",
                "value": "0x244b"
            },
            "error": "Reverted",
            "result": {
                "gasUsed": "0x9daf",
                "output": "0x"
            },
            "subtraces": 3,
            "traceAddress": [],
            "type": "call"
        }"#;
        let trace = serde_json::from_str::<TransactionTrace>(s).unwrap();

        assert!(matcher.matches(&trace));
    }

    #[test]
    fn test_filter_matcher() {
        let addr0 = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse().unwrap();
        let addr1 = "0x160f5f00288e9e1cc8655b327e081566e580a71d".parse().unwrap();
        let addr2 = "0x160f5f00288e9e1cc8655b327e081566e580a71f".parse().unwrap();

        let m0 = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: Default::default(),
            to_addresses: Default::default(),
        };

        let m1 = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: AddressFilter::from(vec![addr0]),
            to_addresses: Default::default(),
        };

        let m2 = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: AddressFilter::from(vec![]),
            to_addresses: AddressFilter::from(vec![addr1]),
        };

        let m3 = TraceFilterMatcher {
            mode: TraceFilterMode::Union,
            from_addresses: AddressFilter::from(vec![addr0]),
            to_addresses: AddressFilter::from(vec![addr1]),
        };

        let m4 = TraceFilterMatcher {
            mode: TraceFilterMode::Intersection,
            from_addresses: Default::default(),
            to_addresses: Default::default(),
        };

        let m5 = TraceFilterMatcher {
            mode: TraceFilterMode::Intersection,
            from_addresses: AddressFilter::from(vec![addr0]),
            to_addresses: Default::default(),
        };

        let m6 = TraceFilterMatcher {
            mode: TraceFilterMode::Intersection,
            from_addresses: Default::default(),
            to_addresses: AddressFilter::from(vec![addr1]),
        };

        let m7 = TraceFilterMatcher {
            mode: TraceFilterMode::Intersection,
            from_addresses: AddressFilter::from(vec![addr0]),
            to_addresses: AddressFilter::from(vec![addr1]),
        };

        // normal call 0
        let trace = TransactionTrace {
            action: Action::Call(CallAction { from: addr0, to: addr1, ..Default::default() }),
            ..Default::default()
        };
        assert!(m0.matches(&trace));
        assert!(m1.matches(&trace));
        assert!(m2.matches(&trace));
        assert!(m3.matches(&trace));
        assert!(m4.matches(&trace));
        assert!(m5.matches(&trace));
        assert!(m6.matches(&trace));
        assert!(m7.matches(&trace));

        // normal call 1
        let trace = TransactionTrace {
            action: Action::Call(CallAction { from: addr0, to: addr2, ..Default::default() }),
            ..Default::default()
        };
        assert!(m0.matches(&trace));
        assert!(m1.matches(&trace));
        assert!(!m2.matches(&trace));
        assert!(m3.matches(&trace));
        assert!(m4.matches(&trace));
        assert!(m5.matches(&trace));
        assert!(!m6.matches(&trace));
        assert!(!m7.matches(&trace));

        // create success
        let trace = TransactionTrace {
            action: Action::Create(CreateAction {
                from: addr0,
                gas: 10240,
                init: Bytes::new(),
                value: U256::from(0),
                ..Default::default()
            }),
            result: Some(TraceOutput::Create(CreateOutput {
                address: addr1,
                code: Bytes::new(),
                gas_used: 1025,
            })),
            ..Default::default()
        };
        assert!(m0.matches(&trace));
        assert!(m1.matches(&trace));
        assert!(m2.matches(&trace));
        assert!(m3.matches(&trace));
        assert!(m4.matches(&trace));
        assert!(m5.matches(&trace));
        assert!(m6.matches(&trace));
        assert!(m7.matches(&trace));

        // create failure
        let trace = TransactionTrace {
            action: Action::Create(CreateAction {
                from: addr0,
                gas: 100,
                init: Bytes::new(),
                value: U256::from(0),
                ..Default::default()
            }),
            error: Some("out of gas".into()),
            ..Default::default()
        };
        assert!(m0.matches(&trace));
        assert!(m1.matches(&trace));
        assert!(!m2.matches(&trace));
        assert!(m3.matches(&trace));
        assert!(m4.matches(&trace));
        assert!(m5.matches(&trace));
        assert!(!m6.matches(&trace));
        assert!(!m7.matches(&trace));

        // selfdestruct
        let trace = TransactionTrace {
            action: Action::Selfdestruct(SelfdestructAction {
                address: addr0,
                refund_address: addr1,
                balance: U256::from(0),
            }),
            ..Default::default()
        };
        assert!(m0.matches(&trace));
        assert!(m1.matches(&trace));
        assert!(m2.matches(&trace));
        assert!(m3.matches(&trace));
        assert!(m4.matches(&trace));
        assert!(m5.matches(&trace));
        assert!(m6.matches(&trace));
        assert!(m7.matches(&trace));

        // reward
        let trace = TransactionTrace {
            action: Action::Reward(RewardAction {
                author: addr0,
                reward_type: crate::parity::RewardType::Block,
                value: U256::from(0),
            }),
            ..Default::default()
        };
        assert!(m0.matches(&trace));
        assert!(!m1.matches(&trace));
        assert!(!m2.matches(&trace));
        assert!(!m3.matches(&trace));
        assert!(m4.matches(&trace));
        assert!(!m5.matches(&trace));
        assert!(!m6.matches(&trace));
        assert!(!m7.matches(&trace));
    }
}
