//! [EIP-8272] constants.
//!
//! [EIP-8272]: https://eips.ethereum.org/EIPS/eip-8272
//!
//! # Provisional runtime
//!
//! EIP-8272 currently leaves `RECENT_ROOT_CODE` as `TBD`. [`RECENT_ROOT_CODE`] and
//! [`RECENT_ROOT_CODE_HASH`] therefore pin a provisional 345-byte two-operation runtime,
//! not a normative EIP bytecode. Its write path follows the pending
//! [`ethereum/sys-asm#53`] candidate; the complete validator runtime remains provisional.
//!
//! [`ethereum/sys-asm#53`]: https://github.com/ethereum/sys-asm/pull/53

use alloy_primitives::{address, b256, bytes, Address, Bytes, B256};

/// The EIP-8272 recent-root predeploy address.
pub const RECENT_ROOT_ADDRESS: Address = address!("0x0000000000000000000000000000000000008272");

/// Number of slots retained by each recent-root source's ring buffer.
pub const RECENT_ROOT_LENGTH: u64 = 8192;

/// Largest permitted age, in slots, of a recent-root reference.
pub const RECENT_ROOT_USABLE_WINDOW: u64 = RECENT_ROOT_LENGTH - 1;

/// Maximum number of recent-root tuples in one verifier frame.
pub const MAX_RECENT_ROOT_REFERENCES: usize = 16;

/// Size, in bytes, of one `(source_id, slot, root)` tuple.
pub const RECENT_ROOT_TUPLE_BYTES: usize = 72;

/// Size, in bytes, of the `salt || root` write calldata.
pub const RECENT_ROOT_WRITE_CALLDATA_BYTES: usize = 64;

/// Domain used when deriving a committed recent-root entry hash.
pub const RECENT_ROOT_ENTRY_DOMAIN: B256 =
    b256!("8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc");

/// Domain used when deriving a recent-root storage key.
pub const RECENT_ROOT_STORAGE_DOMAIN: B256 =
    b256!("bdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa");

/// Provisional EIP-8272 recent-root runtime.
///
/// It dispatches by calldata length: 64-byte `salt || root` calls write a root, and one to
/// sixteen packed 72-byte tuples validate roots. See this module's documentation for why this
/// is not yet a normative EIP constant.
pub static RECENT_ROOT_CODE: Bytes = bytes!(
    "346100d357366040146100d9573680156100d35780610480106100d35780604890066100d35760007f8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc6000527fbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa6080525b80358060205260a052806020013560c01c4b818111156100d357819003611fff106100d3578060285260208235905281602801602090604837606860002090611fff1660a852813560a05260486080205414156100d35760480181811061007057005b60006000fd5b33600052602060006020376034600c20807f8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc6040524b606852606052602060206088376068604020817fbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa60a852611fff4b1660d05260c852604860a8205500"
);

/// Keccak-256 hash of the provisional [`RECENT_ROOT_CODE`].
pub const RECENT_ROOT_CODE_HASH: B256 =
    b256!("cd1cae00e1d37cf97195f9e716dfa1b9a804e36bb5d7726c4f2c50e2580275a5");

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::keccak256;

    #[test]
    fn provisional_runtime_is_pinned() {
        assert_eq!(RECENT_ROOT_CODE.len(), 345);
        assert_eq!(keccak256(RECENT_ROOT_CODE.as_ref()), RECENT_ROOT_CODE_HASH);
    }

    #[test]
    fn domains_match_their_preimages() {
        assert_eq!(keccak256(b"RECENT_ROOT_ENTRY"), RECENT_ROOT_ENTRY_DOMAIN);
        assert_eq!(keccak256(b"RECENT_ROOT_STORAGE"), RECENT_ROOT_STORAGE_DOMAIN);
    }
}
