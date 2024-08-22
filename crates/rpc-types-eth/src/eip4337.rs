use alloy_primitives::{Address, BlockNumber, Bytes, B256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Options for conditional raw transaction submissions.
// reference for the implementation <https://notes.ethereum.org/@yoav/SkaX2lS9j#>
// See also <https://pkg.go.dev/github.com/aK0nshin/go-ethereum/arbitrum_types#ConditionalOptions>
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalOptions {
    /// A map of account addresses to their expected storage states.
    /// Each account can have a specified storage root or explicit slot-value pairs.
    #[serde(default)]
    pub known_accounts: HashMap<Address, AccountStorage>,
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

/// EIP-4337: User Operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserOperation {
    /// The account making the operation.
    pub sender: Address,
    /// Prevents message replay attacks and serves as a randomizing element for initial user registration.
    pub nonce: U256,
    /// Deployer contract address: Required exclusively for deploying new accounts that don't yet exist on the blockchain.
    pub factory: Address,
    /// Factory data for the account creation process, applicable only when using a deployer contract.
    pub factory_data: Bytes,
    /// The call data.
    pub call_data: Bytes,
    /// The gas limit for the call.
    pub call_gas_limit: U256,
    /// The gas limit for the verification.
    pub verification_gas_limit: U256,
    /// Prepaid gas fee: Covers the bundler's costs for initial transaction validation and data transmission.
    pub pre_verification_gas: U256,
    /// The maximum fee per gas.
    pub max_fee_per_gas: U256,
    /// The maximum priority fee per gas.
    pub max_priority_fee_per_gas: U256,
    /// Paymaster contract address: Needed if a third party is covering transaction costs; left blank for self-funded accounts.
    pub paymaster: Address,
    /// The gas limit for the paymaster verification.
    pub paymaster_verification_gas_limit: U256,
    /// The gas limit for the paymaster post-operation.
    pub paymaster_post_op_gas_limit: U256,
    /// The paymaster data.
    pub paymaster_data: Bytes,
    /// The signature of the transaction.
    pub signature: Bytes,
}

/// Response to sending a user operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendUserOperationResponse {
    /// The hash of the user operation.
    pub user_operation_hash: Bytes,
}
