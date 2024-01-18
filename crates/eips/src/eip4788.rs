//! [EIP-4788] constants.
//!
//! [EIP-4788]: https://eips.ethereum.org/EIPS/eip-4788

use alloy_primitives::{address, Address};

/// The caller to be used when calling the EIP-4788 beacon roots contract at the beginning of the
/// block.
pub const SYSTEM_ADDRESS: Address = address!("fffffffffffffffffffffffffffffffffffffffe");
