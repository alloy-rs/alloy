use std::collections::BTreeMap;

use alloy_consensus::Receipt;
use alloy_primitives::{Address, Bloom, B256, U256};
use alloy_rpc_types_engine::PayloadId;
use alloy_rpc_types_eth::Withdrawal;
use serde::{Deserialize, Serialize};

/// Represents a Flashblock, a real-time block-like structure emitted by the Base L2 chain.
///
/// A Flashblock provides a snapshot of a block’s effects before finalization,
/// allowing faster insight into state transitions, balance changes, and logs.
/// It includes a diff of the block’s execution and associated metadata.
///
/// See: [Base Flashblocks Documentation](https://docs.base.org/chain/flashblocks)
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlashBlock {
    /// The unique payload ID as assigned by the execution engine for this block.
    pub payload_id: PayloadId,
    /// A sequential index that identifies the order of this Flashblock.
    pub index: u64,
    /// The execution diff representing state transitions and transactions.
    pub diff: Diff,
    /// Additional metadata about the block such as receipts and balances.
    pub metadata: Metadata,
}

/// Represents the block-level state and transaction changes from a single Flashblock.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diff {
    /// The new state root after execution of the block.
    pub state_root: B256,
    /// The root hash of the receipts trie for the block.
    pub receipts_root: B256,
    /// Aggregated bloom filter for all logs generated in the block.
    pub logs_bloom: Bloom,
    /// Total gas used by all transactions in the block.
    pub gas_used: u64,
    /// The hash of the block this diff is describing.
    pub block_hash: B256,
    /// List of transaction hashes included in the block.
    pub transactions: Vec<B256>,
    /// Withdrawals included in the block (relevant for post-merge Ethereum withdrawals).
    pub withdrawals: Withdrawal,
}

/// Provides metadata about the block that may be useful for indexing or analysis.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    /// The number of the block in the L2 chain.
    pub block_number: u64,
    /// A map of addresses to their updated balances after the block execution.
    /// This represents balance changes due to transactions, rewards, or system transfers.
    pub new_account_balances: BTreeMap<Address, U256>,
    /// Execution receipts for all transactions in the block.
    /// Contains logs, gas usage, and other EVM-level metadata.
    pub receipts: Receipt,
}
