//! `trace_filter` types and support
use alloy_primitives::Address;
use alloy_serde::num::u64_opt_via_ruint;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Trace filter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TraceFilter {
    /// From block
    #[serde(with = "u64_opt_via_ruint")]
    pub from_block: Option<u64>,
    /// To block
    #[serde(with = "u64_opt_via_ruint")]
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

/// Helper type for matching `from` and `to` addresses. Empty sets match all addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceFilterMatcher {
    mode: TraceFilterMode,
    from_addresses: HashSet<Address>,
    to_addresses: HashSet<Address>,
}

impl TraceFilterMatcher {
    /// Returns `true` if the given `from` and `to` addresses match this filter.
    pub fn matches(&self, from: Address, to: Option<Address>) -> bool {
        match (self.from_addresses.is_empty(), self.to_addresses.is_empty()) {
            (true, true) => true,
            (false, true) => self.from_addresses.contains(&from),
            (true, false) => to.map_or(false, |to_addr| self.to_addresses.contains(&to_addr)),
            (false, false) => match self.mode {
                TraceFilterMode::Union => {
                    self.from_addresses.contains(&from)
                        || to.map_or(false, |to_addr| self.to_addresses.contains(&to_addr))
                }
                TraceFilterMode::Intersection => {
                    self.from_addresses.contains(&from)
                        && to.map_or(false, |to_addr| self.to_addresses.contains(&to_addr))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_filter() {
        let s = r#"{"fromBlock":  "0x3","toBlock":  "0x5"}"#;
        let filter: TraceFilter = serde_json::from_str(s).unwrap();
        assert_eq!(filter.from_block, Some(3));
        assert_eq!(filter.to_block, Some(5));
    }

    #[test]
    fn test_filter_matcher_addresses_unspecified() {
        let test_addr_d8 = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse().unwrap();
        let test_addr_16 = "0x160f5f00288e9e1cc8655b327e081566e580a71d".parse().unwrap();
        let filter_json = json!({
            "fromBlock": "0x3",
            "toBlock": "0x5",
        });
        let filter: TraceFilter =
            serde_json::from_value(filter_json).expect("Failed to parse filter");
        let matcher = filter.matcher();
        assert!(matcher.matches(test_addr_d8, None));
        assert!(matcher.matches(test_addr_16, None));
        assert!(matcher.matches(test_addr_d8, Some(test_addr_16)));
        assert!(matcher.matches(test_addr_16, Some(test_addr_d8)));
    }

    #[test]
    fn test_filter_matcher_from_address() {
        let test_addr_d8 = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse().unwrap();
        let test_addr_16 = "0x160f5f00288e9e1cc8655b327e081566e580a71d".parse().unwrap();
        let filter_json = json!({
            "fromBlock": "0x3",
            "toBlock": "0x5",
            "fromAddress": [test_addr_d8]
        });
        let filter: TraceFilter = serde_json::from_value(filter_json).unwrap();
        let matcher = filter.matcher();
        assert!(matcher.matches(test_addr_d8, None));
        assert!(!matcher.matches(test_addr_16, None));
        assert!(matcher.matches(test_addr_d8, Some(test_addr_16)));
        assert!(!matcher.matches(test_addr_16, Some(test_addr_d8)));
    }

    #[test]
    fn test_filter_matcher_to_address() {
        let test_addr_d8 = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse().unwrap();
        let test_addr_16 = "0x160f5f00288e9e1cc8655b327e081566e580a71d".parse().unwrap();
        let filter_json = json!({
            "fromBlock": "0x3",
            "toBlock": "0x5",
            "toAddress": [test_addr_d8],
        });
        let filter: TraceFilter = serde_json::from_value(filter_json).unwrap();
        let matcher = filter.matcher();
        assert!(matcher.matches(test_addr_16, Some(test_addr_d8)));
        assert!(!matcher.matches(test_addr_16, None));
        assert!(!matcher.matches(test_addr_d8, Some(test_addr_16)));
    }

    #[test]
    fn test_filter_matcher_both_addresses_union() {
        let test_addr_d8 = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse().unwrap();
        let test_addr_16 = "0x160f5f00288e9e1cc8655b327e081566e580a71d".parse().unwrap();
        let filter_json = json!({
            "fromBlock": "0x3",
            "toBlock": "0x5",
            "fromAddress": [test_addr_16],
            "toAddress": [test_addr_d8],
        });
        let filter: TraceFilter = serde_json::from_value(filter_json).unwrap();
        let matcher = filter.matcher();
        assert!(matcher.matches(test_addr_16, Some(test_addr_d8)));
        assert!(matcher.matches(test_addr_16, None));
        assert!(matcher.matches(test_addr_d8, Some(test_addr_d8)));
        assert!(!matcher.matches(test_addr_d8, Some(test_addr_16)));
    }

    #[test]
    fn test_filter_matcher_both_addresses_intersection() {
        let test_addr_d8 = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse().unwrap();
        let test_addr_16 = "0x160f5f00288e9e1cc8655b327e081566e580a71d".parse().unwrap();
        let filter_json = json!({
            "fromBlock": "0x3",
            "toBlock": "0x5",
            "fromAddress": [test_addr_16],
            "toAddress": [test_addr_d8],
            "mode": "intersection",
        });
        let filter: TraceFilter = serde_json::from_value(filter_json).unwrap();
        let matcher = filter.matcher();
        assert!(matcher.matches(test_addr_16, Some(test_addr_d8)));
        assert!(!matcher.matches(test_addr_16, None));
        assert!(!matcher.matches(test_addr_d8, Some(test_addr_d8)));
        assert!(!matcher.matches(test_addr_d8, Some(test_addr_16)));
    }
}
