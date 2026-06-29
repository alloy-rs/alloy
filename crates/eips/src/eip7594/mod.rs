//! Types and constants for PeerDAS.
//!
//! See also [EIP-7594](https://eips.ethereum.org/EIPS/eip-7594): PeerDAS - Peer Data Availability Sampling
use crate::eip4844::{FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENT_BYTES};
use alloy_primitives::FixedBytes;

/// Number of field elements in a Reed-Solomon extended blob.
pub const FIELD_ELEMENTS_PER_EXT_BLOB: usize = FIELD_ELEMENTS_PER_BLOB as usize * 2;

/// Number of field elements in a cell.
pub const FIELD_ELEMENTS_PER_CELL: usize = 64;

/// The number of bytes in a cell.
pub const BYTES_PER_CELL: usize = FIELD_ELEMENTS_PER_CELL * FIELD_ELEMENT_BYTES as usize;

/// The number of cells in an extended blob.
pub const CELLS_PER_EXT_BLOB: usize = FIELD_ELEMENTS_PER_EXT_BLOB / FIELD_ELEMENTS_PER_CELL;

/// A wrapper version for EIP-7594 sidecar encoding.
pub const EIP_7594_WRAPPER_VERSION: u8 = 1;

/// Maximum number of blobs per transaction after Fusaka hardfork activation.
pub const MAX_BLOBS_PER_TX_FUSAKA: u64 = 6;

/// A commitment/proof serialized as 0x-prefixed hex string
pub type Cell = FixedBytes<BYTES_PER_CELL>;

mod rlp;
pub use rlp::*;

#[cfg(feature = "kzg-sidecar")]
mod sidecar;
#[cfg(feature = "kzg-sidecar")]
pub use sidecar::*;

#[cfg(all(feature = "kzg-sidecar", feature = "serde", feature = "serde-bincode-compat"))]
pub use sidecar::serde_bincode_compat;
