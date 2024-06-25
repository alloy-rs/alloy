use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

/// Request for `eth_cancelPrivateTransaction`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelPrivateTransactionRequest {
    /// Transaction hash of the transaction to be canceled
    pub tx_hash: B256,
}
