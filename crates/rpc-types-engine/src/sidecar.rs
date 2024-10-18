//! Contains helpers for dealing with additional parameters of `newPayload` requests.

use crate::MaybeCancunPayloadFields;

/// Container type for all available additional `newPayload` request parameters which are not present in the `Execution`
#[derive(Debug, Clone, Default)]
pub struct ExecutionPayloadSidecar {
    /// Fields introduced in `engine_newPayloadV3` that are not present in the `ExecutionPayload` RPC
    cancun: MaybeCancunPayloadFields,
    /// The 7685
    prague: Option<()>

}