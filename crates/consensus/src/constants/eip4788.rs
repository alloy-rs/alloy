use alloy_primitives::{address, Address};

/// The caller to be used when calling the EIP-4788 beacon roots contract at the beginning of the
/// block.
pub const SYSTEM_ADDRESS: Address = address!("fffffffffffffffffffffffffffffffffffffffe");
