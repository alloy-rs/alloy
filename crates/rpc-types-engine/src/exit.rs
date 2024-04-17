//! Contains Exit types, first introduced in the Prague hardfork: <https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md>
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110).

use alloy_primitives::{FixedBytes, B256};
use serde::{Deserialize, Serialize};

/// This structure maps onto the exit object
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitV1 {
    /// Validator public key
    pub pubkey: FixedBytes<48>,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount in GWEI
    #[serde(with = "alloy_serde::u64_hex")]
    pub amount: u64,
    /// Deposit signature
    pub signature: FixedBytes<96>,
    /// Deposit index
    #[serde(with = "alloy_serde::u64_hex")]
    pub index: u64,
}
