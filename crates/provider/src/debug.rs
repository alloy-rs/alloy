//! This module extends the Ethereum JSON-RPC provider with the Debug namespace's RPC methods.
use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{BlockNumber, TxHash, B256};
use alloy_rpc_types::{BlockNumberOrTag, TransactionRequest};
use alloy_rpc_types_trace::geth::{
    GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace,
};
use alloy_transport::{Transport, TransportResult};

/// Debug namespace rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait DebugApi<N, T>: Send + Sync {
    /// Reruns the transaction specified by the hash and returns the trace.
    ///
    /// It will replay any prior transactions to achieve the same state the transaction was executed
    /// in.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace>;

    /// Return a full stack trace of all invoked opcodes of all transaction that were included in
    /// this block.
    ///
    /// The parent of the block must be present or it will fail.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_block_by_hash(
        &self,
        block: B256,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<GethTrace>>;

    /// Same as `debug_trace_block_by_hash` but block is specified by number.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_block_by_number(
        &self,
        block: BlockNumber,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<GethTrace>>;

    /// Executes the given transaction without publishing it like `eth_call` and returns the trace
    /// of the execution.
    ///
    /// The transaction will be executed in the context of the given block number or tag.
    /// The state its run on is the state of the previous block.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    ///
    /// Not all nodes support this call.
    async fn debug_trace_call(
        &self,
        tx: TransactionRequest,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<GethTrace>;

    /// Same as `debug_trace_call` but it used to run and trace multiple transactions at once.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_call_many(
        &self,
        txs: Vec<TransactionRequest>,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<Vec<GethTrace>>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> DebugApi<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace> {
        self.client().request("debug_traceTransaction", (hash, trace_options)).await
    }

    async fn debug_trace_block_by_hash(
        &self,
        block: B256,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<GethTrace>> {
        self.client().request("debug_traceBlockByHash", (block, trace_options)).await
    }

    async fn debug_trace_block_by_number(
        &self,
        block: BlockNumber,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<GethTrace>> {
        self.client().request("debug_traceBlockByNumber", (block, trace_options)).await
    }

    async fn debug_trace_call(
        &self,
        tx: TransactionRequest,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<GethTrace> {
        self.client().request("debug_traceCall", (tx, block, trace_options)).await
    }

    async fn debug_trace_call_many(
        &self,
        txs: Vec<TransactionRequest>,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<Vec<GethTrace>> {
        self.client().request("debug_traceCallMany", (txs, block, trace_options)).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_network::TransactionBuilder;
    use alloy_primitives::U256;

    extern crate self as alloy_provider;

    // NOTE: We cannot import the test-utils crate here due to a circular dependency.
    include!("../../internal-test-utils/src/providers.rs");

    #[tokio::test]
    async fn test_debug_trace_transaction() {
        init_tracing();
        let (provider, anvil) = spawn_anvil();

        let from = anvil.addresses()[0];
        let to = anvil.addresses()[1];

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest::default()
            .from(from)
            .to(to)
            .value(U256::from(100))
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);
        let pending = provider.send_transaction(tx).await.unwrap();
        let receipt = pending.get_receipt().await.unwrap();

        let hash = receipt.transaction_hash;
        let trace_options = GethDebugTracingOptions::default();

        let trace = provider.debug_trace_transaction(hash, trace_options).await.unwrap();

        if let GethTrace::Default(trace) = trace {
            assert_eq!(trace.gas, 21000)
        }
    }

    #[tokio::test]
    async fn test_debug_trace_call() {
        init_tracing();
        let (provider, anvil) = spawn_anvil();

        let from = anvil.addresses()[0];

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest::default()
            .from(from)
            .with_input("0xdeadbeef")
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);

        let trace = provider
            .debug_trace_call(tx, BlockNumberOrTag::Latest, GethDebugTracingCallOptions::default())
            .await
            .unwrap();

        if let GethTrace::Default(trace) = trace {
            assert!(!trace.struct_logs.is_empty());
        }
    }
}
