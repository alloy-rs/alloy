//! Contains the history storage contract, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-2935](https://eips.ethereum.org/EIPS/eip-2935): Serve historical block hashes from state.

use alloy_primitives::{address, bytes, Address, Bytes};

/// The address for the EIP-2935 history storage contract.
pub const HISTORY_STORAGE_ADDRESS: Address = address!("25a219378dad9b3503c8268c9ca836a52427a4fb");

/// The code for the EIP-2935 history storage contract.
pub static HISTORY_STORAGE_CODE: Bytes = bytes!("60203611603157600143035f35116029575f356120000143116029576120005f3506545f5260205ff35b5f5f5260205ff35b5f5ffd00");
