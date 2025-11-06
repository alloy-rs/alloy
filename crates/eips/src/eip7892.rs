//! Contains constants and helper functions for [EIP-7892](https://github.com/ethereum/EIPs/tree/master/EIPS/eip-7892.md)

use crate::eip7840::BlobParams;
use alloc::vec::Vec;

/// Targeted blob count with BPO1 activation
pub const BPO1_TARGET_BLOBS_PER_BLOCK: u64 = 10;

/// Max blob count with BPO1 activation
pub const BPO1_MAX_BLOBS_PER_BLOCK: u64 = 15;

/// Update fraction for BPO1
pub const BPO1_BASE_UPDATE_FRACTION: u64 = 8346193;

/// Targeted blob count with BPO2 activation
pub const BPO2_TARGET_BLOBS_PER_BLOCK: u64 = 14;

/// Max blob count with BPO2 activation
pub const BPO2_MAX_BLOBS_PER_BLOCK: u64 = 21;

/// Update fraction for BPO2
pub const BPO2_BASE_UPDATE_FRACTION: u64 = 11684671;

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
    /// Configuration for blob-related calculations for the Osaka hardfork.
    pub osaka: BlobParams,
    /// Configuration for blob-related calculations for the Amsterdam hardfork.
    pub amsterdam: BlobParams,
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
            osaka: BlobParams::osaka(),
            amsterdam: BlobParams::amsterdam(),
            scheduled: Default::default(),
        }
    }

    /// Configures the scheduled [`BlobParams`] with timestamps.
    pub fn with_scheduled(
        mut self,
        scheduled: impl IntoIterator<Item = (u64, BlobParams)>,
    ) -> Self {
        self.scheduled = scheduled.into_iter().collect();
        self
    }

    /// Returns the highest active blob parameters at the given timestamp.
    ///
    /// Note: this does only scan the entries scheduled by timestamp and not cancun or prague.
    pub fn active_scheduled_params_at_timestamp(&self, timestamp: u64) -> Option<&BlobParams> {
        self.scheduled.iter().rev().find(|(ts, _)| timestamp >= *ts).map(|(_, params)| params)
    }

    /// Returns the configured Cancun [`BlobParams`].
    pub const fn cancun(&self) -> &BlobParams {
        &self.cancun
    }

    /// Returns the configured Prague [`BlobParams`].
    pub const fn prague(&self) -> &BlobParams {
        &self.prague
    }

    /// Returns the configured Osaka [`BlobParams`].
    pub const fn osaka(&self) -> &BlobParams {
        &self.osaka
    }
}

impl Default for BlobScheduleBlobParams {
    fn default() -> Self {
        Self::mainnet()
    }
}
