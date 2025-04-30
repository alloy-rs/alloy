//!

use crate::eip7840::BlobParams;
use alloc::vec::Vec;

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
    pub cancun: BlobParams,
    /// Configuration for blob-related calculations for the Prague hardfork.
    pub prague: BlobParams,
    /// Time-based scheduled updates to blob parameters
    pub scheduled: Vec<(u64, BlobParams)>,
}
