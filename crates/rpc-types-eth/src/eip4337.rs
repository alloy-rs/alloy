use alloy_primitives::{Address, BlockNumber, B256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Options for conditional raw transaction submissions.
// reference for the implementation <https://notes.ethereum.org/@yoav/SkaX2lS9j#>
// See also <https://pkg.go.dev/github.com/aK0nshin/go-ethereum/arbitrum_types#ConditionalOptions>
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalOptions {
    /// The minimal block number at which the transaction can be included.
    /// `None` indicates no minimum block number constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_number_min: Option<BlockNumber>,
    /// The maximal block number at which the transaction can be included.
    /// `None` indicates no maximum block number constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_number_max: Option<BlockNumber>,
    /// The minimal timestamp at which the transaction can be included.
    /// `None` indicates no minimum timestamp constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_min: Option<u64>,
    /// The maximal timestamp at which the transaction can be included.
    /// `None` indicates no maximum timestamp constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_max: Option<u64>,
    /// A map of account addresses to their expected storage states.
    /// Each account can have a specified storage root or explicit slot-value pairs.
    #[serde(default)]
    pub known_accounts: HashMap<Address, AccountStorage>,
}

/// Represents the expected state of an account for a transaction to be conditionally accepted.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AccountStorage {
    /// Expected storage root hash of the account.
    RootHash(B256),
    /// Explicit storage slots and their expected values.
    Slots(HashMap<U256, B256>),
}
