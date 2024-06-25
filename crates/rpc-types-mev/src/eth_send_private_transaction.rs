use crate::common::{Privacy, Validity};
use alloy_primitives::Bytes;
use serde::{Deserialize, Serialize};

/// Request for `eth_sendPrivateTransaction`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateTransactionRequest {
    /// raw signed transaction
    pub tx: Bytes,
    /// Hex-encoded number string, optional. Highest block number in which the transaction should
    /// be included.
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub max_block_number: Option<u64>,
    /// Preferences for private transaction.
    #[serde(default, skip_serializing_if = "PrivateTransactionPreferences::is_empty")]
    pub preferences: PrivateTransactionPreferences,
}

/// Additional preferences for `eth_sendPrivateTransaction`
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct PrivateTransactionPreferences {
    /// Requirements for the bundle to be included in the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validity: Option<Validity>,
    /// Preferences on what data should be shared about the bundle and its transactions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy: Option<Privacy>,
}

impl PrivateTransactionPreferences {
    /// Returns true if the preferences are empty.
    pub const fn is_empty(&self) -> bool {
        self.validity.is_none() && self.privacy.is_none()
    }
}
