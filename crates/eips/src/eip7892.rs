//! Contains constants and helper functions for [EIP-7892](https://github.com/ethereum/EIPs/tree/master/EIPS/eip-7892.md)

use crate::eip7840::BlobParams;
use alloc::{collections::BTreeMap, string::String, vec::Vec};

/// A scheduled blob parameter update entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlobScheduleEntry {
    /// Blob parameters for the Cancun hardfork
    Cancun(BlobParams),
    /// Blob parameters for the Prague hardfork
    Prague(BlobParams),
    /// Blob parameters that take effect at a specific timestamp
    TimestampUpdate(u64, BlobParams),
}

/// Blob parameters configuration for a chain, including scheduled updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobScheduleBlobParams {
    /// Configuration for blob-related calculations for the Cancun hardfork.
    pub cancun: BlobParams,
    /// Configuration for blob-related calculations for the Prague hardfork.
    pub prague: BlobParams,
    /// Time-based scheduled updates to blob parameters.
    ///
    /// These are ordered by activation timestamps in natural order.
    pub scheduled: Vec<(u64, BlobParams)>,
}

impl BlobScheduleBlobParams {
    /// Returns the blob schedule for the ethereum mainnet.
    pub fn mainnet() -> Self {
        Self {
            cancun: BlobParams::cancun(),
            prague: BlobParams::prague(),
            scheduled: Default::default(),
        }
    }

    /// Returns the highest active blob parameters at the given timestamp.
    ///
    /// Note: this does only scan the entries scheduled by timestamp and not cancun or prague.
    pub fn active_scheduled_params_at_timestamp(&self, timestamp: u64) -> Option<&BlobParams> {
        self.scheduled.iter().rev().find(|(ts, _)| timestamp >= *ts).map(|(_, params)| params)
    }

    /// Returns the configured cancun [`BlobParams`].
    pub const fn cancun(&self) -> &BlobParams {
        &self.cancun
    }

    /// Returns the configured prague [`BlobParams`].
    pub const fn prague(&self) -> &BlobParams {
        &self.prague
    }

    /// Finds the active scheduled blob parameters for a given timestamp.
    pub fn from_schedule(schedule: &BTreeMap<String, BlobParams>) -> Self {
        let mut cancun = None;
        let mut prague = None;
        let mut scheduled = Vec::new();

        for (key, params) in schedule {
            match key.as_str() {
                "cancun" => cancun = Some(*params),
                "prague" => prague = Some(*params),
                _ => {
                    if let Ok(timestamp) = key.parse::<u64>() {
                        scheduled.push((timestamp, *params));
                    }
                }
            }
        }

        scheduled.sort_by_key(|(timestamp, _)| *timestamp);

        Self {
            cancun: cancun.unwrap_or_else(BlobParams::cancun),
            prague: prague.unwrap_or_else(BlobParams::prague),
            scheduled,
        }
    }
}

impl Default for BlobScheduleBlobParams {
    fn default() -> Self {
        Self::mainnet()
    }
}
