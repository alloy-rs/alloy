use crate::{u256_numeric_string, ConsideredByBuildersAt, SealedByBuildersAt};

use alloy_primitives::{TxHash, B256, U256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Response for `flashbots_getBundleStatsV2` represents stats for a single bundle
///
/// Note: this is V2: <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#flashbots_getbundlestatsv2>
///
/// Timestamp format: "2022-10-06T21:36:06.322Z"
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum BundleStats {
    /// The relayer has not yet seen the bundle.
    #[default]
    Unknown,
    /// The relayer has seen the bundle, but has not simulated it yet.
    Seen(StatsSeen),
    /// The relayer has seen the bundle and has simulated it.
    Simulated(StatsSimulated),
}

impl Serialize for BundleStats {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unknown => serde_json::json!({"isSimulated": false}).serialize(serializer),
            Self::Seen(stats) => stats.serialize(serializer),
            Self::Simulated(stats) => stats.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BundleStats {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = serde_json::Map::deserialize(deserializer)?;

        if map.get("receivedAt").is_none() {
            Ok(Self::Unknown)
        } else if map["isSimulated"] == false {
            StatsSeen::deserialize(serde_json::Value::Object(map))
                .map(BundleStats::Seen)
                .map_err(serde::de::Error::custom)
        } else {
            StatsSimulated::deserialize(serde_json::Value::Object(map))
                .map(BundleStats::Simulated)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// Response for `flashbots_getBundleStatsV2` represents stats for a single bundle
///
/// Note: this is V2: <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#flashbots_getbundlestatsv2>
///
/// Timestamp format: "2022-10-06T21:36:06.322Z
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSeen {
    /// boolean representing if this searcher has a high enough reputation to be in the high
    /// priority queue
    pub is_high_priority: bool,
    /// representing whether the bundle gets simulated. All other fields will be omitted except
    /// simulated field if API didn't receive bundle
    pub is_simulated: bool,
    /// time at which the bundle API received the bundle
    pub received_at: String,
}

/// Response for `flashbots_getBundleStatsV2` represents stats for a single bundle
///
/// Note: this is V2: <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#flashbots_getbundlestatsv2>
///
/// Timestamp format: "2022-10-06T21:36:06.322Z
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSimulated {
    /// boolean representing if this searcher has a high enough reputation to be in the high
    /// priority queue
    pub is_high_priority: bool,
    /// representing whether the bundle gets simulated. All other fields will be omitted except
    /// simulated field if API didn't receive bundle
    pub is_simulated: bool,
    /// time at which the bundle gets simulated
    pub simulated_at: String,
    /// time at which the bundle API received the bundle
    pub received_at: String,
    /// indicates time at which each builder selected the bundle to be included in the target
    /// block
    #[serde(default = "Vec::new")]
    pub considered_by_builders_at: Vec<ConsideredByBuildersAt>,
    /// indicates time at which each builder sealed a block containing the bundle
    #[serde(default = "Vec::new")]
    pub sealed_by_builders_at: Vec<SealedByBuildersAt>,
}

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

/// Request for builder-specific `*_getBundleStats` endpoints, e.g. Titan's
/// `titan_getBundleStats` or Bombora's `bombora_getBundleStats`.
///
/// Note: Quasar's `quasar_getBundleStats` expects the parameter key in snake case
/// (`bundle_hash`), which is accepted when deserializing but not produced when serializing.
///
/// See also:
/// - Titan: <https://docs.titanbuilder.xyz/bundle-tracing>
/// - Bombora: <https://bombora.build/docs/bombora-getBundleStats>
/// - Quasar: <https://docs.quasar.win/bundle-tracing>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBundleStatsRequest {
    /// The bundle hash of the bundle as returned by `eth_sendBundle`.
    #[serde(alias = "bundle_hash")]
    pub bundle_hash: B256,
}

/// Response for builder-specific `*_getBundleStats` endpoints.
///
/// This covers the shared response format of:
/// - Titan: `titan_getBundleStats` <https://docs.titanbuilder.xyz/bundle-tracing>
/// - Bombora: `bombora_getBundleStats` <https://bombora.build/docs/bombora-getBundleStats>
/// - Quasar: `quasar_getBundleStats` <https://docs.quasar.win/bundle-tracing>
///
/// Fields that are only reported by some builders are optional.
///
/// Note: this is distinct from the (deprecated) Flashbots `flashbots_getBundleStatsV2`
/// response, see [`BundleStats`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderBundleStats {
    /// Status of the bundle.
    pub status: BuilderBundleStatus,
    /// The builder payment observed on the bundle's entry simulation, in wei.
    ///
    /// Empty or absent when the bundle failed simulation.
    #[serde(default, with = "opt_u256_numeric_string", skip_serializing_if = "Option::is_none")]
    pub builder_payment: Option<U256>,
    /// The builder payment observed when the bundle was used in a build attempt, in wei.
    ///
    /// Populated when the bundle reached [`BuilderBundleStatus::IncludedInBlock`] or
    /// [`BuilderBundleStatus::Submitted`]. Not reported by Quasar.
    #[serde(default, with = "opt_u256_numeric_string", skip_serializing_if = "Option::is_none")]
    pub builder_payment_when_included: Option<U256>,
    /// Human readable reason when the bundle failed simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// PropAMM interaction status of the bundle. Only reported by Bombora.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pamm_status: Option<BundlePammStatus>,
    /// Hash of the transaction that caused the simulation failure. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverting_hash: Option<TxHash>,
    /// Unix timestamp when the builder received the bundle. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub received_at: Option<u64>,
    /// Unix timestamp when the bundle was simulated. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulated_at: Option<u64>,
    /// Unix timestamp when a block containing the bundle was submitted to a relay. Only
    /// reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<u64>,
    /// The block number the bundle was included in. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// The block number the bundle targeted. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_block: Option<u64>,
    /// The bundle's `minTimestamp`. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_timestamp: Option<u64>,
    /// The bundle's `maxTimestamp`. Only reported by Quasar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_timestamp: Option<u64>,
}

/// Status of a bundle as reported by builder `*_getBundleStats` endpoints, see
/// [`BuilderBundleStats`].
///
/// Statuses are serialized in PascalCase (e.g. `SimulationPass`) as reported by Titan and
/// Bombora; Quasar's camelCase variants (e.g. `simulationPass`) are accepted when
/// deserializing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuilderBundleStatus {
    /// The builder has no record of the bundle.
    #[serde(alias = "notFound")]
    NotFound,
    /// The bundle is pending processing.
    #[serde(alias = "pending")]
    Pending,
    /// The bundle was received but arrived too late to enter the bundle pool for its target
    /// block.
    #[serde(alias = "received")]
    Received,
    /// The bundle failed validation, e.g. invalid RLP, block number, nonce or chain id.
    #[serde(alias = "invalid")]
    Invalid,
    /// The bundle failed the top-of-block simulation, e.g. a non-whitelisted transaction
    /// reverted or the builder payment was not positive.
    #[serde(alias = "simulationFail")]
    SimulationFail,
    /// The bundle simulated successfully but was not selected for a block, e.g. because it
    /// arrived too late or the builder payment was insufficient.
    #[serde(alias = "simulationPass")]
    SimulationPass,
    /// The bundle was selected for a block by at least one sorting algorithm, but that block
    /// was not submitted to a relay.
    #[serde(alias = "includedInBlock")]
    IncludedInBlock,
    /// The bundle was included in at least one block that was submitted to a relay. This does
    /// not guarantee the block won the slot.
    #[serde(alias = "submitted")]
    Submitted,
}

/// PropAMM interaction status of a bundle as reported by Bombora's `bombora_getBundleStats`.
///
/// <https://bombora.build/docs/bombora-getBundleStats>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BundlePammStatus {
    /// The bundle swapped through a PropAMM pool.
    SwappedThroughPamm,
    /// The bundle was matched with a maker quote.
    MatchedWithQuote,
}

/// Serde helpers for optional wei amounts encoded as numeric strings, where an empty string
/// denotes absence.
mod opt_u256_numeric_string {
    use crate::u256_numeric_string;
    use alloy_primitives::U256;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(serde_json::Value::String(s)) if s.is_empty() => Ok(None),
            Some(val) => u256_numeric_string::deserialize(val).map(Some).map_err(de::Error::custom),
        }
    }

    pub(crate) fn serialize<S>(val: &Option<U256>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match val {
            Some(val) => u256_numeric_string::serialize(val, serializer),
            None => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    use crate::SealedByBuildersAt;

    #[test]
    fn can_deserialize_builder_bundle_stats() {
        // Bombora style: <https://bombora.build/docs/bombora-getBundleStats>
        let s = r#"{
            "status": "IncludedInBlock",
            "builderPayment": "13618000000000",
            "builderPaymentWhenIncluded": "13618000000000",
            "error": "",
            "pammStatus": null
        }"#;
        let stats = serde_json::from_str::<BuilderBundleStats>(s).unwrap();
        assert_eq!(stats.status, BuilderBundleStatus::IncludedInBlock);
        assert_eq!(stats.builder_payment, Some(U256::from(13618000000000u64)));
        assert_eq!(stats.builder_payment_when_included, Some(U256::from(13618000000000u64)));
        assert!(stats.pamm_status.is_none());

        // failed simulation reports an empty builder payment
        let s = r#"{
            "status": "SimulationFail",
            "builderPayment": "",
            "error": "BundleRevert. Reverting Hash: 0x1111111111111111111111111111111111111111111111111111111111111111",
            "pammStatus": "matchedWithQuote"
        }"#;
        let stats = serde_json::from_str::<BuilderBundleStats>(s).unwrap();
        assert_eq!(stats.status, BuilderBundleStatus::SimulationFail);
        assert_eq!(stats.builder_payment, None);
        assert_eq!(stats.pamm_status, Some(BundlePammStatus::MatchedWithQuote));

        // Quasar style: <https://docs.quasar.win/bundle-tracing>
        let s = r#"{
            "status": "submitted",
            "builderPayment": "12934799399930",
            "error": null,
            "revertingHash": null,
            "receivedAt": 1753977388,
            "simulatedAt": 1753977388,
            "submittedAt": 1753977409,
            "blockNumber": 23040121,
            "targetBlock": 23040121,
            "minTimestamp": null,
            "maxTimestamp": null
        }"#;
        let stats = serde_json::from_str::<BuilderBundleStats>(s).unwrap();
        assert_eq!(stats.status, BuilderBundleStatus::Submitted);
        assert_eq!(stats.builder_payment, Some(U256::from(12934799399930u64)));
        assert_eq!(stats.received_at, Some(1753977388));
        assert_eq!(stats.submitted_at, Some(1753977409));
        assert_eq!(stats.block_number, Some(23040121));
        assert_eq!(stats.target_block, Some(23040121));
        assert_eq!(stats.min_timestamp, None);
    }

    #[test]
    fn can_deserialize_get_bundle_stats_request() {
        let camel = r#"{"bundleHash": "0x1111111111111111111111111111111111111111111111111111111111111111"}"#;
        let snake = r#"{"bundle_hash": "0x1111111111111111111111111111111111111111111111111111111111111111"}"#;
        let a = serde_json::from_str::<GetBundleStatsRequest>(camel).unwrap();
        let b = serde_json::from_str::<GetBundleStatsRequest>(snake).unwrap();
        assert_eq!(a, b);
        assert!(serde_json::to_string(&a).unwrap().contains("bundleHash"));
    }

    #[test]
    fn can_serialize_deserialize_bundle_stats() {
        let fixtures = [
            (
                r#"{
                    "isSimulated": false
                }"#,
                BundleStats::Unknown,
            ),
            (
                r#"{
                    "isHighPriority": false,
                    "isSimulated": false,
                    "receivedAt": "476190476193"
                }"#,
                BundleStats::Seen(StatsSeen {
                    is_high_priority: false,
                    is_simulated: false,
                    received_at: "476190476193".to_string(),
                }),
            ),
            (
                r#"{
                    "isHighPriority": true,
                    "isSimulated": true,
                    "simulatedAt": "111",
                    "receivedAt": "222",
                    "consideredByBuildersAt":[],
                    "sealedByBuildersAt": [
                        {
                            "pubkey": "333",
                            "timestamp": "444"
                        },
                        {
                            "pubkey": "555",
                            "timestamp": "666"
                        }
                    ]
                }"#,
                BundleStats::Simulated(StatsSimulated {
                    is_high_priority: true,
                    is_simulated: true,
                    simulated_at: String::from("111"),
                    received_at: String::from("222"),
                    considered_by_builders_at: vec![],
                    sealed_by_builders_at: vec![
                        SealedByBuildersAt {
                            pubkey: String::from("333"),
                            timestamp: String::from("444"),
                        },
                        SealedByBuildersAt {
                            pubkey: String::from("555"),
                            timestamp: String::from("666"),
                        },
                    ],
                }),
            ),
        ];

        let strip_whitespaces =
            |input: &str| input.chars().filter(|&c| !c.is_whitespace()).collect::<String>();

        for (serialized, deserialized) in fixtures {
            // Check de-serialization
            let deserialized_expected = serde_json::from_str::<BundleStats>(serialized).unwrap();
            assert_eq!(deserialized, deserialized_expected);

            // Check serialization
            let serialized_expected = &serde_json::to_string(&deserialized).unwrap();
            assert_eq!(strip_whitespaces(serialized), strip_whitespaces(serialized_expected));
        }
    }
}
