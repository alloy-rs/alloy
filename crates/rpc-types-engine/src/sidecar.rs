//! Contains helpers for dealing with additional parameters of `newPayload` requests.

use crate::{CancunPayloadFields, MaybeCancunPayloadFields};
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
    prague: Option<Requests>,
}

impl ExecutionPayloadSidecar {
    /// Returns a new empty instance (pre-cancun, v1, v2)
    pub const fn none() -> Self {
        Self { cancun: MaybeCancunPayloadFields::none(), prague: None }
    }

    /// Creates a new instance for cancun with the cancun fields for `engine_newPayloadV3`
    pub fn v3(cancun: CancunPayloadFields) -> Self {
        Self { cancun: cancun.into(), prague: None }
    }

    /// Creates a new instance post prague for `engine_newPayloadV4`
    pub fn v4(cancun: CancunPayloadFields, requests: Requests) -> Self {
        Self { cancun: cancun.into(), prague: Some(requests) }
    }

    /// Returns a reference to the [`CancunPayloadFields`].
    pub const fn cancun(&self) -> Option<&CancunPayloadFields> {
        self.cancun.as_ref()
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
    pub const fn requests(&self) -> Option<&Requests> {
        self.prague.as_ref()
    }
}
