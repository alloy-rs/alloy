use alloy_consensus_any::AnyTxEnvelope;
use alloy_rpc_types_eth::{Block, Transaction};
use alloy_serde::WithOtherFields;

/// A catch-all header type for handling headers on multiple networks.
pub type AnyRpcHeader = alloy_rpc_types_eth::Header<alloy_consensus_any::AnyHeader>;

/// A catch-all block type for handling blocks on multiple networks.
pub type AnyRpcBlock =
    WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>;
