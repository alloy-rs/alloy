//! Contains WithdrawalRequest types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7002](https://eips.ethereum.org/EIPS/eip-7002): Execution layer triggerable withdrawals

use alloy_primitives::{Address, FixedBytes};
use serde::{Deserialize, Serialize};

/// Represents an execution layer triggerable withdrawal request.
///
/// See [EIP-7002](https://eips.ethereum.org/EIPS/eip-7002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalRequest {
    /// Address of the source of the exit.
    pub source_address: Address,
    /// Validator public key.
    pub pubkey: FixedBytes<48>,
    /// Amount of withdrawn ether in gwei.
    #[serde(with = "alloy_serde::u64_via_ruint")]
    pub amount: u64,
}
