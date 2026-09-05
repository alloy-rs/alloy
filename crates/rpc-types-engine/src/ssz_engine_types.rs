//! Experimental Engine API v2 REST-SSZ wire types.
//!
//! These types intentionally live apart from the legacy JSON-RPC Engine API types because their
//! SSZ encodings are not always wire-compatible. This module contains the shared endpoint
//! containers, fork-specific payload containers, blob containers, and payload-body containers.
//! Decoders enforce REST-SSZ schema bounds independently of transport request-size limits.
//! Encoders preserve the wire layout of the supplied values; callers constructing values directly
//! remain responsible for satisfying the schema bounds.

use crate::{
    BlobsBundleV1, BlobsBundleV2, ExecutionPayloadBodyV1 as LegacyExecutionPayloadBodyV1,
    ExecutionPayloadBodyV2 as LegacyExecutionPayloadBodyV2,
    ExecutionPayloadEnvelopeV2 as LegacyBuiltPayloadShanghai,
    ExecutionPayloadEnvelopeV4 as LegacyBuiltPayloadPrague,
    ExecutionPayloadEnvelopeV5 as LegacyBuiltPayloadOsaka,
    ExecutionPayloadEnvelopeV6 as LegacyBuiltPayloadAmsterdam, ExecutionPayloadFieldV2,
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, ExecutionPayloadV4,
    ForkchoiceState, ForkchoiceUpdated as LegacyForkchoice,
    PayloadAttributes as LegacyPayloadAttributes, PayloadId, PayloadStatus as LegacyPayloadStatus,
    PayloadStatusEnum,
};
use alloy_eips::{
    eip4844::{Blob, BlobAndProofV1, BlobAndProofV2, BlobCellsAndProofsV1, Bytes48},
    eip4895::Withdrawal,
    eip7594::Cell,
    eip7685::Requests,
};
use alloy_primitives::{Address, Bytes, B128, B256, U256};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

mod bounds;
use bounds::{blob_entries, blob_hashes, body_entries, body_hashes, cells, proofs};

/// Maximum number of blobs in a REST-SSZ request or response.
pub const MAX_BLOBS_REQUEST: usize = 128;
/// Maximum number of bodies in a REST-SSZ request or response.
pub const MAX_BODIES_REQUEST: usize = 32;
/// Maximum UTF-8 byte length of a payload validation error.
pub const MAX_ERROR_BYTES: usize = 1024;

mod status;
pub use status::*;
mod payload;
pub use payload::*;
mod bodies;
pub use bodies::*;
mod blobs;
pub use blobs::*;

#[cfg(test)]
mod tests;
