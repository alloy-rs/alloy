use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// Options for conditional raw transaction submissions.
// reference for the implementation https://notes.ethereum.org/@yoav/SkaX2lS9j#

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalTxOptions {
    /// The minimal block number at which the transaction can be included.
    /// `None` indicates no minimum block number constraint.
    pub block_number_min: Option<U256>,
    /// The maximal block number at which the transaction can be included.
    /// `None` indicates no maximum block number constraint.
    pub block_number_max: Option<U256>,
    /// The minimal timestamp at which the transaction can be included.
    /// `None` indicates no minimum timestamp constraint.
    pub timestamp_min: Option<U256>,
    /// The maximal timestamp at which the transaction can be included.
    /// `None` indicates no maximum timestamp constraint.
    pub timestamp_max: Option<U256>,
    /// A map of account addresses to their expected storage states.
    /// Each account can have a specified storage root or explicit slot-value pairs.
    pub known_accounts: HashMap<Address, KnownAccountState>,
}
/// Represents the expected state of an account for a transaction to be conditionally accepted.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum KnownAccountState {
    /// Expected storage root hash of the account.
    StorageRoot(B256),
    /// Explicit storage slots and their expected values.
    Slots(HashMap<U256, B256>),
}
