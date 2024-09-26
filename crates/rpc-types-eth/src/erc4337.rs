use crate::{Log, TransactionReceipt};
use alloc::vec::Vec;
use alloy_primitives::{
    map::{AddressHashMap, HashMap},
    Address, BlockNumber, Bytes, B256, U256,
};

/// Options for conditional raw transaction submissions.
// reference for the implementation <https://notes.ethereum.org/@yoav/SkaX2lS9j#>
// See also <https://pkg.go.dev/github.com/aK0nshin/go-ethereum/arbitrum_types#ConditionalOptions>
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ConditionalOptions {
    /// A map of account addresses to their expected storage states.
    /// Each account can have a specified storage root or explicit slot-value pairs.
    #[cfg_attr(feature = "serde", serde(default))]
    pub known_accounts: AddressHashMap<AccountStorage>,
    /// The minimal block number at which the transaction can be included.
    /// `None` indicates no minimum block number constraint.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub block_number_min: Option<BlockNumber>,
    /// The maximal block number at which the transaction can be included.
    /// `None` indicates no maximum block number constraint.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub block_number_max: Option<BlockNumber>,
    /// The minimal timestamp at which the transaction can be included.
    /// `None` indicates no minimum timestamp constraint.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub timestamp_min: Option<u64>,
    /// The maximal timestamp at which the transaction can be included.
    /// `None` indicates no maximum timestamp constraint.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub timestamp_max: Option<u64>,
}

/// Represents the expected state of an account for a transaction to be conditionally accepted.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum AccountStorage {
    /// Expected storage root hash of the account.
    RootHash(B256),
    /// Explicit storage slots and their expected values.
    Slots(HashMap<U256, B256>),
}

/// [`UserOperation`] in the spec: Entry Point V0.6
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserOperation {
    /// The address of the smart contract account
    pub sender: Address,
    /// Anti-replay protection; also used as the salt for first-time account creation
    pub nonce: U256,
    /// Code used to deploy the account if not yet on-chain
    pub init_code: Bytes,
    /// Data that's passed to the sender for execution
    pub call_data: Bytes,
    /// Gas limit for execution phase
    pub call_gas_limit: U256,
    /// Gas limit for verification phase
    pub verification_gas_limit: U256,
    /// Gas to compensate the bundler
    pub pre_verification_gas: U256,
    /// Maximum fee per gas
    pub max_fee_per_gas: U256,
    /// Maximum priority fee per gas
    pub max_priority_fee_per_gas: U256,
    /// Paymaster Contract address and any extra data required for verification and execution
    /// (empty for self-sponsored transaction)
    pub paymaster_and_data: Bytes,
    /// Used to validate a UserOperation along with the nonce during verification
    pub signature: Bytes,
}

/// [`PackedUserOperation`] in the spec: Entry Point V0.7
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PackedUserOperation {
    /// The account making the operation.
    pub sender: Address,
    /// Prevents message replay attacks and serves as a randomizing element for initial user
    /// registration.
    pub nonce: U256,
    /// Deployer contract address: Required exclusively for deploying new accounts that don't yet
    /// exist on the blockchain.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub factory: Option<Address>,
    /// Factory data for the account creation process, applicable only when using a deployer
    /// contract.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub factory_data: Option<Bytes>,
    /// The call data.
    pub call_data: Bytes,
    /// The gas limit for the call.
    pub call_gas_limit: U256,
    /// The gas limit for the verification.
    pub verification_gas_limit: U256,
    /// Prepaid gas fee: Covers the bundler's costs for initial transaction validation and data
    /// transmission.
    pub pre_verification_gas: U256,
    /// The maximum fee per gas.
    pub max_fee_per_gas: U256,
    /// The maximum priority fee per gas.
    pub max_priority_fee_per_gas: U256,
    /// Paymaster contract address: Needed if a third party is covering transaction costs; left
    /// blank for self-funded accounts.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub paymaster: Option<Address>,
    /// The gas limit for the paymaster verification.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub paymaster_verification_gas_limit: Option<U256>,
    /// The gas limit for the paymaster post-operation.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub paymaster_post_op_gas_limit: Option<U256>,
    /// The paymaster data.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub paymaster_data: Option<Bytes>,
    /// The signature of the transaction.
    pub signature: Bytes,
}

/// Send User Operation
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SendUserOperation {
    /// User Operation
    EntryPointV06(UserOperation),
    /// Packed User Operation
    EntryPointV07(PackedUserOperation),
}

/// Response to sending a user operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SendUserOperationResponse {
    /// The hash of the user operation.
    pub user_op_hash: Bytes,
}

/// Represents the receipt of a user operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserOperationReceipt {
    /// The hash of the user operation.
    pub user_op_hash: Bytes,
    /// The entry point address for the user operation.
    pub entry_point: Address,
    /// The address of the sender of the user operation.
    pub sender: Address,
    /// The nonce of the user operation.
    pub nonce: U256,
    /// The address of the paymaster, if any.
    pub paymaster: Address,
    /// The actual gas cost incurred by the user operation.
    pub actual_gas_cost: U256,
    /// The actual gas used by the user operation.
    pub actual_gas_used: U256,
    /// Indicates whether the user operation was successful.
    pub success: bool,
    /// The reason for failure, if any.
    pub reason: Bytes,
    /// The logs generated by the user operation.
    pub logs: Vec<Log>,
    /// The transaction receipt of the user operation.
    pub receipt: TransactionReceipt,
}

/// Represents the gas estimation for a user operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserOperationGasEstimation {
    /// The gas limit for the pre-verification.
    pub pre_verification_gas: U256,
    /// The gas limit for the verification.
    pub verification_gas: U256,
    /// The gas limit for the paymaster verification.
    pub paymaster_verification_gas: U256,
    /// The gas limit for the call.
    pub call_gas_limit: U256,
}
