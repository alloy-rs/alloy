//! Types used by tracing backends.

use alloy_primitives::TxHash;
use serde::{Deserialize, Serialize};

/// The result of a single transaction trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TraceResult<Ok, Err> {
    /// Untagged success variant
    Success {
        /// Trace results produced by the tracer
        result: Ok,
        /// transaction hash
        #[serde(skip_serializing_if = "Option::is_none", rename = "txHash")]
        tx_hash: Option<TxHash>,
    },
    /// Untagged error variant
    Error {
        /// Trace failure produced by the tracer
        error: Err,
        /// transaction hash
        #[serde(skip_serializing_if = "Option::is_none", rename = "txHash")]
        tx_hash: Option<TxHash>,
    },
}

impl<Ok, Err> TraceResult<Ok, Err> {
    /// Returns the hash of the transaction that was traced.
    pub const fn tx_hash(&self) -> Option<TxHash> {
        *match self {
            TraceResult::Success { tx_hash, .. } => tx_hash,
            TraceResult::Error { tx_hash, .. } => tx_hash,
        }
    }
}
