//! Contains Exit types, first introduced in the Prague hardfork: <https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md>
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110).

use alloy_primitives::{Address, FixedBytes};
use serde::{Deserialize, Serialize};

/// This structure maps onto the exit object
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitV1 {
    /// Address of the source of the exit.
    pub source_address: Address,
    /// Validator public key.
    pub pubkey: FixedBytes<48>,
    /// Amount of withdrawn ether in gwei.
    #[serde(with = "alloy_serde::u64_via_ruint")]
    pub amount: u64,
}
