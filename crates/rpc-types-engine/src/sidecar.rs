//! Contains helpers for dealing with additional parameters of `newPayload` requests.

use crate::{
    CancunPayloadFields, MaybeCancunPayloadFields, MaybePraguePayloadFields, PraguePayloadFields,
};
use alloc::vec::Vec;
use alloy_eips::eip7685::Requests;
use alloy_primitives::B256;

/// Container type for all available additional `newPayload` request parameters that are not present
/// in the `ExecutionPayload` object itself.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionPayloadSidecar {
    /// Cancun request params introduced in `engine_newPayloadV3` that are not present in the
    /// `ExecutionPayload`.
    cancun: MaybeCancunPayloadFields,
    /// The EIP-7685 requests provided as additional request params to `engine_newPayloadV4` that
    /// are not present in the `ExecutionPayload`.
    prague: MaybePraguePayloadFields,
}

impl ExecutionPayloadSidecar {
    /// Returns a new empty instance (pre-cancun, v1, v2)
    pub const fn none() -> Self {
        Self { cancun: MaybeCancunPayloadFields::none(), prague: MaybePraguePayloadFields::none() }
    }

    /// Creates a new instance for cancun with the cancun fields for `engine_newPayloadV3`
    pub fn v3(cancun: CancunPayloadFields) -> Self {
        Self { cancun: cancun.into(), prague: MaybePraguePayloadFields::none() }
    }

    /// Creates a new instance post prague for `engine_newPayloadV4`
    pub fn v4(cancun: CancunPayloadFields, prague: PraguePayloadFields) -> Self {
        Self { cancun: cancun.into(), prague: prague.into() }
    }

    /// Returns a reference to the [`CancunPayloadFields`].
    pub const fn cancun(&self) -> Option<&CancunPayloadFields> {
        self.cancun.as_ref()
    }

    /// Returns a reference to the [`PraguePayloadFields`].
    pub const fn prague(&self) -> Option<&PraguePayloadFields> {
        self.prague.as_ref()
    }

    /// Returns the parent beacon block root, if any.
    pub fn parent_beacon_block_root(&self) -> Option<B256> {
        self.cancun.parent_beacon_block_root()
    }

    /// Returns the blob versioned hashes, if any.
    pub fn versioned_hashes(&self) -> Option<&Vec<B256>> {
        self.cancun.versioned_hashes()
    }

    /// Returns the EIP-7685 requests
    pub fn requests(&self) -> Option<&Requests> {
        self.prague.requests()
    }

    /// Returns the target blobs per block
    pub fn target_blobs_per_block(&self) -> Option<u64> {
        self.prague.target_blobs_per_block()
    }
}
