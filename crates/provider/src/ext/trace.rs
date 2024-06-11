//! This module extends the Ethereum JSON-RPC provider with the Trace namespace's RPC methods.
use crate::{Provider, RpcWithBlock};
use alloy_eips::BlockNumberOrTag;
use alloy_network::Network;
use alloy_primitives::TxHash;
use alloy_rpc_types_trace::parity::{LocalizedTransactionTrace, TraceResults, TraceType};
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
    fn trace_call<'a, 'b>(
        &self,
        request: &'a N::TransactionRequest,
        trace_type: &'b [TraceType],
    ) -> RpcWithBlock<T, (&'a N::TransactionRequest, &'b [TraceType]), TraceResults>;

    /// Traces multiple transactions on top of the same block, i.e. transaction `n` will be executed
    /// on top of the given block with all `n - 1` transaction applied first.
    ///
    /// Allows tracing dependent transactions.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    fn trace_call_many<'a>(
        &self,
        request: TraceCallList<'a, N>,
    ) -> RpcWithBlock<T, TraceCallList<'a, N>, TraceResults>;

    /// Parity trace transaction.
    async fn trace_transaction(
        &self,
        hash: TxHash,
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
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> TraceApi<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    fn trace_call<'a, 'b>(
        &self,
        request: &'a <N as Network>::TransactionRequest,
        trace_type: &'b [TraceType],
    ) -> RpcWithBlock<T, (&'a <N as Network>::TransactionRequest, &'b [TraceType]), TraceResults>
    {
        RpcWithBlock::new(self.weak_client(), "trace_call", (request, trace_type))
    }

    fn trace_call_many<'a>(
        &self,
        request: TraceCallList<'a, N>,
    ) -> RpcWithBlock<T, TraceCallList<'a, N>, TraceResults> {
        RpcWithBlock::new(self.weak_client(), "trace_callMany", request)
    }

    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_transaction", (hash,)).await
    }

    async fn trace_block(
        &self,
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_block", (block,)).await
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
