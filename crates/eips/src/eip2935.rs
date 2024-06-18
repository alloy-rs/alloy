//! Contains the history storage contract, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-2935](https://eips.ethereum.org/EIPS/eip-2935): Serve historical block hashes from state.

use alloy_primitives::{address, bytes, Address, Bytes};

/// The address for the EIP-2935 history storage contract.
pub const HISTORY_STORAGE_ADDRESS: Address = address!("0aae40965e6800cd9b1f4b05ff21581047e3f91e");

/// The code for the EIP-2935 history storage contract.
pub static HISTORY_STORAGE_CODE: Bytes = bytes!("3373fffffffffffffffffffffffffffffffffffffffe1460575767ffffffffffffffff5f3511605357600143035f3511604b575f35612000014311604b57611fff5f3516545f5260205ff35b5f5f5260205ff35b5f5ffd5b5f35611fff60014303165500");
