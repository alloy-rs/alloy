//! Types for the PropAMM (proprietary AMM) taker APIs exposed by block builders.
//!
//! PropAMMs are on-chain pools whose prices are refreshed every block: market makers stream
//! signed quote updates directly to block builders, and the builder guarantees that the
//! maker's freshest quote lands immediately before the transaction that reads it.
//!
//! These types cover the public taker surface:
//! - the state override stream (`/ws/pamm_quote_stream`) and `titan_getPammStateOverrides`:
//!   [`PammQuoteStreamMessage`], [`PammStateOverrides`]
//! - the price level stream (`/ws/pamm_price_levels`) and `titan_getPammPriceLevels`:
//!   [`PammPriceLevelsSnapshot`]
//! - quote requests (`titan_getPammQuote`, `titan_getPammQuoteVenue`): [`PammQuote`]
//!
//! The maker side (`/ws/sendquoteupdate`) is a protobuf-over-WebSocket protocol and not
//! covered by these types. On-chain, quote updates target the Flashbots priority update
//! registry: <https://github.com/flashbots/priority-update-registry>
//!
//! References:
//! - Titan: <https://docs.titanbuilder.xyz/propamms>
//! - Bombora: <https://bombora.build/docs/propamm>
//! - BuilderNet: <https://buildernet.org/docs/api>

use alloy_primitives::{Address, U256};
use alloy_rpc_types_eth::state::StateOverride;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A message on the PropAMM state override stream (`/ws/pamm_quote_stream`).
///
/// Every message is a complete snapshot of the latest maker quotes for the upcoming block,
/// expressed as state overrides on the priority update registry. The overrides can be passed
/// directly to simulation tooling that supports state overrides (e.g. `eth_call`,
/// `eth_simulateV1`) to simulate routes against next-block prices.
///
/// See:
/// - Titan: <https://docs.titanbuilder.xyz/propamms/takers>
/// - Bombora: <https://bombora.build/docs/propamm-takers>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammQuoteStreamMessage {
    /// The consensus slot the quotes target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u64>,
    /// The block number the quotes target.
    #[serde(alias = "block_number")]
    pub block_number: u64,
    /// Unix timestamp in nanoseconds at which the snapshot was emitted.
    pub timestamp: u64,
    /// The latest quote per PropAMM, keyed by the PropAMM's stream address.
    #[serde(flatten)]
    pub pamms: BTreeMap<Address, PammQuoteStreamEntry>,
}

/// A single PropAMM entry of a [`PammQuoteStreamMessage`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammQuoteStreamEntry {
    /// The state override representing the maker's latest quote for this PropAMM.
    #[serde(alias = "state_override")]
    pub state_override: StateOverride,
}

/// Response for `titan_getPammStateOverrides`: the latest maker quotes as state overrides on
/// the priority update registry.
///
/// <https://docs.titanbuilder.xyz/propamms/takers>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammStateOverrides {
    /// The block number the overrides target.
    #[serde(with = "alloy_serde::quantity")]
    pub block_number: u64,
    /// The state overrides representing the latest maker quotes, merged over all PropAMMs.
    pub state_overrides: StateOverride,
}

/// A snapshot of PropAMM price levels, as returned by `titan_getPammPriceLevels` and streamed
/// on `/ws/pamm_price_levels`.
///
/// Every snapshot is complete; a newer snapshot supersedes all previous ones.
///
/// <https://docs.titanbuilder.xyz/propamms/takers>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammPriceLevelsSnapshot {
    /// The consensus slot the price levels target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u64>,
    /// The block number the price levels target.
    #[serde(alias = "block_number")]
    pub block_number: u64,
    /// Unix timestamp in nanoseconds at which the snapshot was emitted.
    pub timestamp: u64,
    /// The price levels per PropAMM.
    pub pamms: Vec<PammPriceLevels>,
}

/// Price levels of a single PropAMM, see [`PammPriceLevelsSnapshot`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammPriceLevels {
    /// The address of the PropAMM.
    pub pamm: Address,
    /// The price levels per token pair.
    pub pairs: Vec<PammPairPriceLevels>,
}

/// Price levels of a single token pair of a PropAMM, see [`PammPriceLevels`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammPairPriceLevels {
    /// The input token of the swap.
    pub token_in: Address,
    /// The output token of the swap.
    pub token_out: Address,
    /// The price levels, ordered by increasing input amount.
    pub order_book: Vec<PammPriceLevel>,
}

/// A single price level of a PropAMM order book, see [`PammPairPriceLevels`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammPriceLevel {
    /// The input amount of the level.
    pub amount_in: U256,
    /// The output amount received for the input amount.
    pub amount_out: U256,
    /// How the level was derived.
    pub variant: PammPriceLevelVariant,
}

/// How a [`PammPriceLevel`] was derived.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PammPriceLevelVariant {
    /// The level was simulated in the EVM with a synthesized taker transaction.
    #[default]
    Simulated,
    /// The level was linearly interpolated between simulated levels.
    Interpolated,
}

/// A PropAMM quote as returned by `titan_getPammQuote` and `titan_getPammQuoteVenue`.
///
/// <https://docs.titanbuilder.xyz/propamms/takers>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PammQuote {
    /// The input token of the swap.
    pub token_in: Address,
    /// The output token of the swap.
    pub token_out: Address,
    /// The input amount of the swap.
    pub amount_in: U256,
    /// The output amount received for the input amount.
    pub amount_out: U256,
    /// The PropAMM the quote is for.
    pub pamm: Address,
    /// The router contract to route the swap through.
    pub router: Address,
    /// The block number the quote is valid for.
    pub block_number: u64,
    /// The consensus slot the quote is valid for.
    pub slot: u64,
    /// Unix timestamp in nanoseconds at which the quote was emitted.
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn can_serde_pamm_quote_stream_message() {
        let s = r#"{
            "slot": 12345678,
            "blockNumber": 23040121,
            "timestamp": 1753977388000000000,
            "0xbc60639345dfa607d73b74e88c2d54d8b8ad7cc3": {
                "stateOverride": {
                    "0xda7afeed01fe625cf15d187a19f94b45f00b8c5f": {
                        "stateDiff": {
                            "0x0000000000000000000000000000000000000000000000000000000000000001": "0x0000000000000000000000000000000000000000000000000000000000000002"
                        }
                    }
                }
            }
        }"#;
        let msg = serde_json::from_str::<PammQuoteStreamMessage>(s).unwrap();
        assert_eq!(msg.slot, Some(12345678));
        assert_eq!(msg.block_number, 23040121);
        assert_eq!(msg.timestamp, 1753977388000000000);
        let entry = &msg.pamms[&address!("0xbc60639345dfa607d73b74e88c2d54d8b8ad7cc3")];
        let account =
            &entry.state_override[&address!("0xda7afeed01fe625cf15d187a19f94b45f00b8c5f")];
        assert_eq!(account.state_diff.as_ref().unwrap().len(), 1);

        let value = serde_json::to_value(&msg).unwrap();
        let rt = serde_json::from_value::<PammQuoteStreamMessage>(value).unwrap();
        assert_eq!(rt, msg);
    }

    #[test]
    fn can_deserialize_pamm_state_overrides() {
        let s = r#"{
            "blockNumber": "0x16f3a10",
            "stateOverrides": {
                "0xda7afeed01fe625cf15d187a19f94b45f00b8c5f": {
                    "stateDiff": {
                        "0x0000000000000000000000000000000000000000000000000000000000000001": "0x0000000000000000000000000000000000000000000000000000000000000002"
                    }
                }
            }
        }"#;
        let overrides = serde_json::from_str::<PammStateOverrides>(s).unwrap();
        assert_eq!(overrides.block_number, 0x16f3a10);
        assert_eq!(overrides.state_overrides.len(), 1);
    }

    #[test]
    fn can_deserialize_pamm_price_levels() {
        let s = r#"{
            "slot": 12345678,
            "blockNumber": 23040121,
            "timestamp": 1753977388000000000,
            "pamms": [{
                "pamm": "0x5979458912f80b96d30d4220af8e2e4925a33320",
                "pairs": [{
                    "tokenIn": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
                    "tokenOut": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                    "orderBook": [
                        {"amountIn": "0xde0b6b3a7640000", "amountOut": "0xd3c21b", "variant": "Simulated"},
                        {"amountIn": "0x1bc16d674ec80000", "amountOut": "0x1a78436", "variant": "Interpolated"}
                    ]
                }]
            }]
        }"#;
        let snapshot = serde_json::from_str::<PammPriceLevelsSnapshot>(s).unwrap();
        assert_eq!(snapshot.pamms.len(), 1);
        let pair = &snapshot.pamms[0].pairs[0];
        assert_eq!(pair.order_book.len(), 2);
        assert_eq!(pair.order_book[0].variant, PammPriceLevelVariant::Simulated);
        assert_eq!(pair.order_book[1].variant, PammPriceLevelVariant::Interpolated);
    }

    #[test]
    fn can_deserialize_pamm_quote() {
        let s = r#"{
            "tokenIn": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            "tokenOut": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            "amountIn": "0xde0b6b3a7640000",
            "amountOut": "0xd3c21b",
            "pamm": "0x5979458912f80b96d30d4220af8e2e4925a33320",
            "router": "0x4ddf368080cd7946db5b459ad591c350158175e1",
            "blockNumber": 23040121,
            "slot": 12345678,
            "timestamp": 1753977388000000000
        }"#;
        let quote = serde_json::from_str::<PammQuote>(s).unwrap();
        assert_eq!(quote.router, address!("0x4ddf368080cd7946db5b459ad591c350158175e1"));
        assert_eq!(quote.amount_in, U256::from(1000000000000000000u64));
        assert_eq!(quote.block_number, 23040121);
    }
}
