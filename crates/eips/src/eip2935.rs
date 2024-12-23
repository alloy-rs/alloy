//! Contains the history storage contract, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-2935](https://eips.ethereum.org/EIPS/eip-2935): Serve historical block hashes from state.

use alloy_primitives::{address, bytes, Address, Bytes};

/// The address for the EIP-2935 history storage contract.
pub const HISTORY_STORAGE_ADDRESS: Address = address!("0F792be4B0c0cb4DAE440Ef133E90C0eCD48CCCC");

/// The code for the EIP-2935 history storage contract.
pub static HISTORY_STORAGE_CODE: Bytes = bytes!("3373fffffffffffffffffffffffffffffffffffffffe14604657602036036042575f35600143038111604257611fff81430311604257611fff9006545f5260205ff35b5f5ffd5b5f35611fff60014303065500");
