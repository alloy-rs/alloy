use serde::{Deserialize, Serialize};

/// Request for `eth_cancelBundle`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelBundleRequest {
    /// Bundle hash of the bundle to be canceled
    pub bundle_hash: String,
}
