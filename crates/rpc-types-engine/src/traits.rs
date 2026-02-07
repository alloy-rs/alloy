//! Traits for execution payload and payload attributes abstractions.

use alloc::vec::Vec;
use alloy_eips::{eip1898::BlockWithParent, eip4895::Withdrawal, BlockNumHash};
use alloy_primitives::{Bytes, B256};
use core::fmt::Debug;

/// Basic attributes required to initiate payload construction.
///
/// Defines minimal parameters needed to build a new execution payload.
pub trait PayloadAttributes:
    serde::de::DeserializeOwned + serde::Serialize + Debug + Clone + Send + Sync + 'static
{
    /// Returns the timestamp for the new payload.
    fn timestamp(&self) -> u64;

    /// Returns the withdrawals to be included in the payload.
    ///
    /// `Some` for post-Shanghai blocks, `None` for earlier blocks.
    fn withdrawals(&self) -> Option<&Vec<Withdrawal>>;

    /// Returns the parent beacon block root.
    ///
    /// `Some` for post-merge blocks, `None` for pre-merge blocks.
    fn parent_beacon_block_root(&self) -> Option<B256>;
}

impl PayloadAttributes for crate::PayloadAttributes {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        self.withdrawals.as_ref()
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.parent_beacon_block_root
    }
}

/// Represents the core data structure of an execution payload.
///
/// Contains all necessary information to execute and validate a block, including
/// headers, transactions, and consensus fields. Provides a unified interface
/// regardless of protocol version.
pub trait ExecutionPayload:
    serde::Serialize + serde::de::DeserializeOwned + Debug + Clone + Send + Sync + 'static
{
    /// Returns the hash of this block's parent.
    fn parent_hash(&self) -> B256;

    /// Returns this block's hash.
    fn block_hash(&self) -> B256;

    /// Returns this block's number (height).
    fn block_number(&self) -> u64;

    /// Returns this block's number hash.
    fn num_hash(&self) -> BlockNumHash {
        BlockNumHash::new(self.block_number(), self.block_hash())
    }

    /// Returns a [`BlockWithParent`] for this block.
    fn block_with_parent(&self) -> BlockWithParent {
        BlockWithParent::new(self.parent_hash(), self.num_hash())
    }

    /// Returns the withdrawals included in this payload.
    ///
    /// Returns `None` for pre-Shanghai blocks.
    fn withdrawals(&self) -> Option<&Vec<Withdrawal>>;

    /// Returns the access list included in this payload.
    ///
    /// Returns `None` for pre-Amsterdam blocks.
    fn block_access_list(&self) -> Option<&Bytes>;

    /// Returns the beacon block root associated with this payload.
    ///
    /// Returns `None` for pre-merge payloads.
    fn parent_beacon_block_root(&self) -> Option<B256>;

    /// Returns this block's timestamp (seconds since Unix epoch).
    fn timestamp(&self) -> u64;

    /// Returns the total gas consumed by all transactions in this block.
    fn gas_used(&self) -> u64;

    /// Returns the number of transactions in the payload.
    fn transaction_count(&self) -> usize;
}

impl ExecutionPayload for crate::ExecutionData {
    fn parent_hash(&self) -> B256 {
        self.payload.parent_hash()
    }

    fn block_hash(&self) -> B256 {
        self.payload.block_hash()
    }

    fn block_number(&self) -> u64 {
        self.payload.block_number()
    }

    fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        self.payload.withdrawals()
    }

    fn block_access_list(&self) -> Option<&Bytes> {
        None
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.sidecar.parent_beacon_block_root()
    }

    fn timestamp(&self) -> u64 {
        self.payload.timestamp()
    }

    fn gas_used(&self) -> u64 {
        self.payload.as_v1().gas_used
    }

    fn transaction_count(&self) -> usize {
        self.payload.as_v1().transactions.len()
    }
}
