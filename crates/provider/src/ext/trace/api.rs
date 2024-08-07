//! This module extends the Ethereum JSON-RPC provider with the Trace namespace's RPC methods.
use super::TraceRpcWithBlock;
use crate::Provider;
use alloy_eips::BlockNumberOrTag;
use alloy_network::Network;
use alloy_primitives::TxHash;
use alloy_rpc_types_eth::Index;
use alloy_rpc_types_trace::{
    filter::TraceFilter,
    parity::{LocalizedTransactionTrace, TraceResults, TraceResultsWithTransactionHash, TraceType},
};
use alloy_transport::{Transport, TransportResult};

/// List of trace calls for use with [`TraceApi::trace_call_many`]
pub type TraceCallList<'a, N> = &'a [(<N as Network>::TransactionRequest, Vec<TraceType>)];

/// Trace namespace rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait TraceApi<N, T>: Send + Sync
where
    N: Network,
    T: Transport + Clone,
{
    /// Executes the given transaction and returns a number of possible traces.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    fn trace_call(
        &self,
        request: N::TransactionRequest,
    ) -> TraceRpcWithBlock<T, N::TransactionRequest, TraceResults>;

    /// Traces multiple transactions on top of the same block, i.e. transaction `n` will be executed
    /// on top of the given block with all `n - 1` transaction applied first.
    ///
    /// Allows tracing dependent transactions.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    fn trace_call_many(
        &self,
        requests: Vec<N::TransactionRequest>,
    ) -> TraceRpcWithBlock<T, Vec<N::TransactionRequest>, Vec<TraceResults>>;

    /// Parity trace transaction.
    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    /// Traces of the transaction on the given positions
    ///
    /// # Note
    ///
    /// This function accepts single index and build list with it under the hood because
    /// trace_get method accepts list of indices but limits this list to len == 1.
    async fn trace_get(
        &self,
        hash: TxHash,
        index: usize,
    ) -> TransportResult<LocalizedTransactionTrace>;

    /// Trace the given raw transaction.
    fn trace_raw_transaction(&self, data: Vec<u8>) -> TraceRpcWithBlock<T, Vec<u8>, TraceResults>;

    /// Traces matching given filter.
    async fn trace_filter(
        &self,
        filter: TraceFilter,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    /// Trace all transactions in the given block.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    /// Replays a transaction.
    fn trace_replay_transaction(&self, hash: TxHash) -> TraceRpcWithBlock<T, TxHash, TraceResults>;

    /// Replays all transactions in the given block.
    fn trace_replay_block_transactions(
        &self,
        block: BlockNumberOrTag,
    ) -> TraceRpcWithBlock<T, BlockNumberOrTag, Vec<TraceResultsWithTransactionHash>>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> TraceApi<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    fn trace_call(
        &self,
        request: N::TransactionRequest,
    ) -> TraceRpcWithBlock<T, N::TransactionRequest, TraceResults> {
        TraceRpcWithBlock::new(self.weak_client(), "trace_call", request)
    }

    fn trace_call_many(
        &self,
        requests: Vec<N::TransactionRequest>,
    ) -> TraceRpcWithBlock<T, Vec<N::TransactionRequest>, Vec<TraceResults>> {
        TraceRpcWithBlock::new(self.weak_client(), "trace_callMany", requests)
    }

    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_transaction", (hash,)).await
    }

    async fn trace_get(
        &self,
        hash: TxHash,
        index: usize,
    ) -> TransportResult<LocalizedTransactionTrace> {
        // We are using `[index]` because API accepts a list, but only supports a single index
        self.client().request("trace_get", (hash, (Index::from(index),))).await
    }

    fn trace_raw_transaction(&self, data: Vec<u8>) -> TraceRpcWithBlock<T, Vec<u8>, TraceResults> {
        TraceRpcWithBlock::new(self.weak_client(), "trace_rawTransaction", data)
    }

    async fn trace_filter(
        &self,
        filter: TraceFilter,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_filter", (filter,)).await
    }

    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_block", (block,)).await
    }

    fn trace_replay_transaction(&self, hash: TxHash) -> TraceRpcWithBlock<T, TxHash, TraceResults> {
        TraceRpcWithBlock::new(self.weak_client(), "trace_replayTransaction", hash)
    }

    fn trace_replay_block_transactions(
        &self,
        block: BlockNumberOrTag,
    ) -> TraceRpcWithBlock<T, BlockNumberOrTag, Vec<TraceResultsWithTransactionHash>> {
        TraceRpcWithBlock::new(self.weak_client(), "trace_replayBlockTransactions", block)
    }
}

#[cfg(test)]
mod test {
    use crate::ProviderBuilder;

    use super::*;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[tokio::test]
    async fn test_trace_block() {
        init_tracing();
        let provider = ProviderBuilder::new().on_anvil();
        let traces = provider.trace_block(BlockNumberOrTag::Latest).await.unwrap();
        assert_eq!(traces.len(), 0);
    }
}
