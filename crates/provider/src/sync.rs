//! Types for synchronous transaction submission with receipt retrieval.

use alloy_network::Network;
use alloy_primitives::{Bytes, TxHash};
use alloy_transport::TransportError;
use std::future::Future;

use crate::heart::PendingTransactionError;

#[cfg(not(target_family = "wasm"))]
/// Future type for SendTransactionSync on non-wasm targets.
pub(crate) type SendTransactionSyncFuture<N> = std::pin::Pin<
    Box<
        dyn Future<Output = Result<<N as Network>::ReceiptResponse, SendTransactionSyncError>>
            + Send,
    >,
>;

#[cfg(target_family = "wasm")]
/// Future type for SendTransactionSync on wasm targets.
pub(crate) type SendTransactionSyncFuture<N> = std::pin::Pin<
    Box<dyn Future<Output = Result<<N as Network>::ReceiptResponse, SendTransactionSyncError>>>,
>;

/// A synchronous transaction sender that returns transaction receipt immediately.
///
/// This future combines transaction submission and receipt fetching into a single operation,
/// reducing the number of RPC calls and providing immediate access to transaction metadata.
#[must_use = "this type does nothing unless you await it"]
pub struct SendTransactionSync<N: Network> {
    /// The raw transaction bytes that were sent.
    raw: Bytes,
    /// The transaction hash.
    tx_hash: TxHash,
    /// The future that will resolve to the transaction receipt.
    fut: SendTransactionSyncFuture<N>,
}

impl<N: Network> SendTransactionSync<N> {
    /// Creates a new sync transaction sender.
    pub fn new(raw: Bytes, tx_hash: TxHash, fut: SendTransactionSyncFuture<N>) -> Self {
        Self { raw, tx_hash, fut }
    }

    /// Returns the raw transaction bytes.
    pub const fn raw(&self) -> &Bytes {
        &self.raw
    }

    /// Returns the transaction hash.
    #[doc(alias = "transaction_hash")]
    pub const fn tx_hash(&self) -> &TxHash {
        &self.tx_hash
    }
}

impl<N: Network> std::fmt::Debug for SendTransactionSync<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendTransactionSync")
            .field("raw", &self.raw)
            .field("tx_hash", &self.tx_hash)
            .finish_non_exhaustive()
    }
}

impl<N: Network> Future for SendTransactionSync<N> {
    type Output = Result<N::ReceiptResponse, SendTransactionSyncError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.fut.as_mut().poll(cx)
    }
}

/// Errors that may occur when using synchronous transaction sending.
#[derive(Debug, thiserror::Error)]
pub enum SendTransactionSyncError {
    /// The raw transaction bytes that failed to be sent.
    #[error("transaction submission failed")]
    SubmissionFailed {
        /// The raw transaction bytes.
        raw: Bytes,
        /// The underlying error.
        #[source]
        error: TransportError,
    },

    /// Receipt retrieval failed but transaction was submitted.
    #[error("receipt retrieval failed for transaction {tx_hash}")]
    ReceiptFailed {
        /// The transaction hash that was successfully submitted.
        tx_hash: TxHash,
        /// The raw transaction bytes.
        raw: Bytes,
        /// The underlying error.
        #[source]
        error: PendingTransactionError,
    },

    /// Transaction was submitted but failed validation or execution.
    #[error("transaction failed")]
    TransactionFailed {
        /// The transaction hash.
        tx_hash: TxHash,
        /// The raw transaction bytes.
        raw: Bytes,
        /// The transaction receipt containing the failure details.
        receipt: Option<alloy_rpc_types_eth::TransactionReceipt>,
    },
}

impl SendTransactionSyncError {
    /// Returns the raw transaction bytes if available.
    pub const fn raw(&self) -> Option<&Bytes> {
        match self {
            Self::SubmissionFailed { raw, .. }
            | Self::ReceiptFailed { raw, .. }
            | Self::TransactionFailed { raw, .. } => Some(raw),
        }
    }

    /// Returns the transaction hash if available.
    pub const fn tx_hash(&self) -> Option<&TxHash> {
        match self {
            Self::SubmissionFailed { .. } => None,
            Self::ReceiptFailed { tx_hash, .. } | Self::TransactionFailed { tx_hash, .. } => {
                Some(tx_hash)
            }
        }
    }
}
