//! EIP-4844 specific constants.

/// Gas consumption of a single data blob.
pub const DATA_GAS_PER_BLOB: u64 = 131_072u64; // 32*4096 = 131072 == 2^17 == 0x20000
