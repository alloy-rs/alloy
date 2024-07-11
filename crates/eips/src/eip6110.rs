//! Contains Deposit types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110): Supply validator deposits on chain
//!
//! Provides validator deposits as a list of deposit operations added to the Execution Layer block.

use alloy_primitives::{address, Address, FixedBytes, B256};
use alloy_rlp::{RlpDecodable, RlpEncodable};

/// Mainnet deposit contract address.
pub const MAINNET_DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("00000000219ab540356cbb839cbe05303d7705fa");

/// This structure maps onto the deposit object from [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DepositRequest {
    /// Validator public key
    pub pubkey: FixedBytes<48>,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount of ether deposited in gwei
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub amount: u64,
    /// Deposit signature
    pub signature: FixedBytes<96>,
    /// Deposit index
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub index: u64,
}
