//! Constants for PeerDAS.
//!
//! See also [EIP-7594](https://eips.ethereum.org/EIPS/eip-7594): PeerDAS - Peer Data Availability Sampling

use crate::eip4844::FIELD_ELEMENTS_PER_BLOB;

/// Number of field elements in a Reed-Solomon extended blob.
pub const FIELD_ELEMENTS_PER_EXT_BLOB: u64 = FIELD_ELEMENTS_PER_BLOB * 2;

/// Number of field elements in a cell.
pub const FIELD_ELEMENTS_PER_CELL: u64 = 64;

/// The number of cells in an extended blob.
pub const CELLS_PER_EXT_BLOB: u64 = FIELD_ELEMENTS_PER_EXT_BLOB / FIELD_ELEMENTS_PER_CELL;
