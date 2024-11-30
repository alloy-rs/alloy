mod receipt;
pub use receipt::AnyTransactionReceipt;

mod request;
pub use request::AnyTransactionRequest;

use alloy_consensus_any::AnyTxEnvelope;
use alloy_rpc_types_eth::Transaction;
use alloy_serde::WithOtherFields;

/// A catch-all transaction type for handling transactions on multiple networks.
pub type AnyRpcTransaction = WithOtherFields<Transaction<AnyTxEnvelope>>;
