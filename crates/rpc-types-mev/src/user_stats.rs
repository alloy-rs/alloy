use crate::u256_numeric_string;
use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

/// Response for `flashbots_getUserStatsV2` represents stats for a searcher.
///
/// Note: this is V2: <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#flashbots_getuserstatsv2>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    /// Represents whether this searcher has a high enough reputation to be in the high priority
    /// queue.
    pub is_high_priority: bool,
    /// The total amount paid to validators over all time.
    #[serde(with = "u256_numeric_string")]
    pub all_time_validator_payments: U256,
    /// The total amount of gas simulated across all bundles submitted to Flashbots.
    /// This is the actual gas used in simulations, not gas limit.
    #[serde(with = "u256_numeric_string")]
    pub all_time_gas_simulated: U256,
    /// The total amount paid to validators the last 7 days.
    #[serde(with = "u256_numeric_string")]
    pub last_7d_validator_payments: U256,
    /// The total amount of gas simulated across all bundles submitted to Flashbots in the last 7
    /// days. This is the actual gas used in simulations, not gas limit.
    #[serde(with = "u256_numeric_string")]
    pub last_7d_gas_simulated: U256,
    /// The total amount paid to validators the last day.
    #[serde(with = "u256_numeric_string")]
    pub last_1d_validator_payments: U256,
    /// The total amount of gas simulated across all bundles submitted to Flashbots in the last
    /// day. This is the actual gas used in simulations, not gas limit.
    #[serde(with = "u256_numeric_string")]
    pub last_1d_gas_simulated: U256,
}
