//! Contains constants for [EIP-7892](https://github.com/ethereum/EIPs/tree/master/EIPS/eip-7892.md)

use crate::eip7840::BlobParams;
use alloc::{collections::BTreeMap, vec::Vec};

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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HardforkBlobParams {
    /// Configuration for blob-related calculations for the Cancun hardfork.
    pub cancun: Option<BlobParams>,
    /// Configuration for blob-related calculations for the Prague hardfork.
    pub prague: Option<BlobParams>,
    /// Time-based scheduled updates to blob parameters
    pub scheduled: Vec<(u64, BlobParams)>,
}

impl HardforkBlobParams {
    /// Returns the active blob parameters at the given timestamp.
    pub fn active_scheduled_params_at_timestamp(&self, timestamp: u64) -> Option<&BlobParams> {
        self.scheduled.iter().rev().find(|(ts, _)| timestamp >= *ts).map(|(_, params)| params)
    }

    /// Finds the active scheduled blob parameters for a given timestamp.
    pub fn from_schedule(schedule: BTreeMap<String, BlobParams>) -> Self {
        let mut cancun = None;
        let mut prague = None;
        let mut scheduled = Vec::new();

        for (key, params) in schedule {
            match key.as_str() {
                "cancun" => cancun = Some(params),
                "prague" => prague = Some(params),
                _ => {
                    if let Ok(timestamp) = key.parse::<u64>() {
                        scheduled.push((timestamp, params));
                    }
                }
            }
        }

        scheduled.sort_by_key(|(timestamp, _)| *timestamp);

        Self { cancun, prague, scheduled }
    }
}
