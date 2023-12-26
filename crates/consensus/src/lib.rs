//! Consensus logic for the Ethereum blockchain.
//!
//! This crate contains constants, types, and functions for implementing
//! Ethereum EL consensus and communication. This includes headers, blocks,
//! transactions, eip2718 envelopes, eip2930, eip4844, and more. The types in
//! this crate implement many of the traits found in [alloy_network].

mod basefee;
pub use basefee::BaseFeeParams;

pub mod constants;

mod header;
pub use header::{Header, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod pure;
pub use pure::{calc_blob_gasprice, calc_excess_blob_gas, calc_next_block_base_fee};

mod receipt;

pub mod transaction;
pub use transaction::{
    AccessList, AccessListItem, TxEip1559, TxEip2930, TxEnvelope, TxKind, TxLegacy, TxType,
};
