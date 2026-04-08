//! Testing namespace types for building a block in a single call.
//!
//! This follows the `testing_buildBlockV1` specification.

use crate::PayloadAttributes;
use alloc::vec::Vec;
use alloy_primitives::{Bytes, B256};

/// Capability string for `testing_buildBlockV1`.
pub const TESTING_BUILD_BLOCK_V1: &str = "testing_buildBlockV1";

/// Request payload for `testing_buildBlockV1`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TestingBuildBlockRequestV1 {
    /// Parent block hash of the block to build.
    pub parent_block_hash: B256,
    /// Payload attributes.
    pub payload_attributes: PayloadAttributes,
    /// Raw signed transactions to force-include in order.
    pub transactions: Vec<Bytes>,
    /// Optional extra data for the block header.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub extra_data: Option<Bytes>,
}
