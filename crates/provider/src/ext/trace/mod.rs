//! This module extends the Ethereum JSON-RPC provider with the Trace namespace's RPC methods.
use crate::Provider;
use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::TxHash;
use alloy_rpc_types_eth::Index;
use alloy_rpc_types_trace::{
    filter::TraceFilter,
    parity::{LocalizedTransactionTrace, TraceResults, TraceResultsWithTransactionHash, TraceType},
};
use alloy_transport::TransportResult;

mod with_block;
pub use with_block::{TraceBuilder, TraceParams};

/// List of trace calls for use with [`TraceApi::trace_call_many`]
pub type TraceCallList<'a, N> = &'a [(<N as Network>::TransactionRequest, &'a [TraceType])];

/// Trace namespace rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait TraceApi<N>: Send + Sync
where
    N: Network,
{
    /// Executes the given transaction and returns a number of possible traces.
    ///
    /// Default trace type is [`TraceType::Trace`].
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    fn trace_call<'a>(
        &self,
        request: &'a N::TransactionRequest,
    ) -> TraceBuilder<&'a N::TransactionRequest, TraceResults>;

    /// Traces multiple transactions on top of the same block, i.e. transaction `n` will be executed
    /// on top of the given block with all `n - 1` transaction applied first.
    ///
    /// Allows tracing dependent transactions.
    ///
    /// If [`BlockId`] is unset the default at which calls will be executed is [`BlockId::pending`].
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    fn trace_call_many<'a>(
        &self,
        request: TraceCallList<'a, N>,
    ) -> TraceBuilder<TraceCallList<'a, N>, Vec<TraceResults>>;

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
    fn trace_raw_transaction<'a>(&self, data: &'a [u8]) -> TraceBuilder<&'a [u8], TraceResults>;

    /// Traces matching given filter.
    async fn trace_filter(
        &self,
        tracer: &TraceFilter,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    /// Trace all transactions in the given block.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn trace_block(&self, block: BlockId) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    /// Replays a transaction.
    ///
    /// Default trace type is [`TraceType::Trace`].
    fn trace_replay_transaction(&self, hash: TxHash) -> TraceBuilder<TxHash, TraceResults>;

    /// Replays all transactions in the given block.
    ///
    /// Default trace type is [`TraceType::Trace`].
    fn trace_replay_block_transactions(
        &self,
        block: BlockId,
    ) -> TraceBuilder<BlockId, Vec<TraceResultsWithTransactionHash>>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> TraceApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    fn trace_call<'a>(
        &self,
        request: &'a <N as Network>::TransactionRequest,
    ) -> TraceBuilder<&'a <N as Network>::TransactionRequest, TraceResults> {
        TraceBuilder::new_rpc(self.client().request("trace_call", request)).pending()
    }

    fn trace_call_many<'a>(
        &self,
        request: TraceCallList<'a, N>,
    ) -> TraceBuilder<TraceCallList<'a, N>, Vec<TraceResults>> {
        TraceBuilder::new_rpc(self.client().request("trace_callMany", request)).pending()
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

    fn trace_raw_transaction<'a>(&self, data: &'a [u8]) -> TraceBuilder<&'a [u8], TraceResults> {
        TraceBuilder::new_rpc(self.client().request("trace_rawTransaction", data))
    }

    async fn trace_filter(
        &self,
        tracer: &TraceFilter,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_filter", (tracer,)).await
    }

    async fn trace_block(&self, block: BlockId) -> TransportResult<Vec<LocalizedTransactionTrace>> {
        self.client().request("trace_block", (block,)).await
    }

    fn trace_replay_transaction(&self, hash: TxHash) -> TraceBuilder<TxHash, TraceResults> {
        TraceBuilder::new_rpc(self.client().request("trace_replayTransaction", hash))
    }

    fn trace_replay_block_transactions(
        &self,
        block: BlockId,
    ) -> TraceBuilder<BlockId, Vec<TraceResultsWithTransactionHash>> {
        TraceBuilder::new_rpc(self.client().request("trace_replayBlockTransactions", block))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ext::test::async_ci_only, ProviderBuilder};
    use alloy_eips::{BlockNumberOrTag, Encodable2718};
    use alloy_network::{EthereumWallet, TransactionBuilder};
    use alloy_node_bindings::{utils::run_with_tempdir, Reth};
    use alloy_primitives::{address, U256};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_signer_local::PrivateKeySigner;

    #[tokio::test]
    async fn trace_block() {
        let provider = ProviderBuilder::new().connect_anvil();
        let traces = provider.trace_block(BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();
        assert_eq!(traces.len(), 0);
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "no reth on windows")]
    async fn trace_call() {
        async_ci_only(|| async move {
            run_with_tempdir("reth-test-", |temp_dir| async move {
                let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir).spawn();
                let provider = ProviderBuilder::new().connect_http(reth.endpoint_url());

                let tx = TransactionRequest::default()
                    .with_from(address!("0000000000000000000000000000000000000123"))
                    .with_to(address!("0000000000000000000000000000000000000456"));

                let result = provider.trace_call(&tx).await;

                let traces = result.unwrap();
                similar_asserts::assert_eq!(
                    serde_json::to_string_pretty(&traces).unwrap().trim(),
                    r#"
{
  "output": "0x",
  "stateDiff": null,
  "trace": [
    {
      "type": "call",
      "action": {
        "from": "0x0000000000000000000000000000000000000123",
        "callType": "call",
        "gas": "0x2fa9e78",
        "input": "0x",
        "to": "0x0000000000000000000000000000000000000456",
        "value": "0x0"
      },
      "result": {
        "gasUsed": "0x0",
        "output": "0x"
      },
      "subtraces": 0,
      "traceAddress": []
    }
  ],
  "vmTrace": null
}
"#
                    .trim(),
                );
            })
            .await;
        })
        .await;
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "no reth on windows")]
    async fn trace_call_many() {
        async_ci_only(|| async move {
            run_with_tempdir("reth-test-", |temp_dir| async move {
                let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir).spawn();
                let provider = ProviderBuilder::new().connect_http(reth.endpoint_url());

                let tx1 = TransactionRequest::default()
                    .with_from(address!("0000000000000000000000000000000000000123"))
                    .with_to(address!("0000000000000000000000000000000000000456"));

                let tx2 = TransactionRequest::default()
                    .with_from(address!("0000000000000000000000000000000000000456"))
                    .with_to(address!("0000000000000000000000000000000000000789"));

                let result = provider
                    .trace_call_many(&[(tx1, &[TraceType::Trace]), (tx2, &[TraceType::Trace])])
                    .await;

                let traces = result.unwrap();
                similar_asserts::assert_eq!(
                    serde_json::to_string_pretty(&traces).unwrap().trim(),
                    r#"
[
  {
    "output": "0x",
    "stateDiff": null,
    "trace": [
      {
        "type": "call",
        "action": {
          "from": "0x0000000000000000000000000000000000000123",
          "callType": "call",
          "gas": "0x2fa9e78",
          "input": "0x",
          "to": "0x0000000000000000000000000000000000000456",
          "value": "0x0"
        },
        "result": {
          "gasUsed": "0x0",
          "output": "0x"
        },
        "subtraces": 0,
        "traceAddress": []
      }
    ],
    "vmTrace": null
  },
  {
    "output": "0x",
    "stateDiff": null,
    "trace": [
      {
        "type": "call",
        "action": {
          "from": "0x0000000000000000000000000000000000000456",
          "callType": "call",
          "gas": "0x2fa9e78",
          "input": "0x",
          "to": "0x0000000000000000000000000000000000000789",
          "value": "0x0"
        },
        "result": {
          "gasUsed": "0x0",
          "output": "0x"
        },
        "subtraces": 0,
        "traceAddress": []
      }
    ],
    "vmTrace": null
  }
]
"#
                    .trim()
                );
            })
            .await;
        })
        .await;
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "no reth on windows")]
    async fn test_replay_tx() {
        async_ci_only(|| async move {
            run_with_tempdir("reth-test-", |temp_dir| async move {
                let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir).spawn();
                let pk: PrivateKeySigner =
                    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                        .parse()
                        .unwrap();

                let wallet = EthereumWallet::new(pk);
                let provider =
                    ProviderBuilder::new().wallet(wallet).connect_http(reth.endpoint_url());

                let tx = TransactionRequest::default()
                    .with_from(address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"))
                    .value(U256::from(1000))
                    .with_to(address!("0000000000000000000000000000000000000456"));

                let res = provider.send_transaction(tx).await.unwrap();

                let receipt = res.get_receipt().await.unwrap();

                let hash = receipt.transaction_hash;

                let result = provider.trace_replay_transaction(hash).await;
                assert!(result.is_ok());

                let traces = result.unwrap();
                similar_asserts::assert_eq!(
                    serde_json::to_string_pretty(&traces).unwrap(),
                    r#"{
  "output": "0x",
  "stateDiff": null,
  "trace": [
    {
      "type": "call",
      "action": {
        "from": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
        "callType": "call",
        "gas": "0x0",
        "input": "0x",
        "to": "0x0000000000000000000000000000000000000456",
        "value": "0x3e8"
      },
      "result": {
        "gasUsed": "0x0",
        "output": "0x"
      },
      "subtraces": 0,
      "traceAddress": []
    }
  ],
  "vmTrace": null
}"#
                );
            })
            .await;
        })
        .await;
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "no reth on windows")]
    async fn trace_raw_tx() {
        async_ci_only(|| async move {
            run_with_tempdir("reth-test-", |temp_dir| async move {
                let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir).spawn();
                let pk: PrivateKeySigner =
                    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                        .parse()
                        .unwrap();

                let provider = ProviderBuilder::new().connect_http(reth.endpoint_url());

                let tx = TransactionRequest::default()
                    .with_from(address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"))
                    .gas_limit(21000)
                    .nonce(0)
                    .value(U256::from(1000))
                    .with_chain_id(provider.get_chain_id().await.unwrap())
                    .with_to(address!("0000000000000000000000000000000000000456"))
                    .with_max_priority_fee_per_gas(1_000_000_000)
                    .with_max_fee_per_gas(20_000_000_000);

                let wallet = EthereumWallet::new(pk);

                let raw = tx.build(&wallet).await.unwrap().encoded_2718();

                let result = provider.trace_raw_transaction(&raw).await;

                let traces = result.unwrap();

                similar_asserts::assert_eq!(
                    serde_json::to_string_pretty(&traces).unwrap(),
                    r#"{
  "output": "0x",
  "stateDiff": null,
  "trace": [
    {
      "type": "call",
      "action": {
        "from": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
        "callType": "call",
        "gas": "0x0",
        "input": "0x",
        "to": "0x0000000000000000000000000000000000000456",
        "value": "0x3e8"
      },
      "result": {
        "gasUsed": "0x0",
        "output": "0x"
      },
      "subtraces": 0,
      "traceAddress": []
    }
  ],
  "vmTrace": null
}"#
                );
            })
            .await;
        })
        .await;
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "no reth on windows")]
    async fn trace_replay_block_transactions() {
        async_ci_only(|| async move {
            run_with_tempdir("reth-test-", |temp_dir| async move {
                let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir).spawn();
                let pk: PrivateKeySigner =
                    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                        .parse()
                        .unwrap();

                let wallet = EthereumWallet::new(pk);
                let provider =
                    ProviderBuilder::new().wallet(wallet).connect_http(reth.endpoint_url());

                let tx = TransactionRequest::default()
                    .with_from(address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"))
                    .value(U256::from(1000))
                    .with_to(address!("0000000000000000000000000000000000000456"));

                let res = provider.send_transaction(tx).await.unwrap();

                let receipt = res.get_receipt().await.unwrap();

                let block_num = receipt.block_number.unwrap();

                let result =
                    provider.trace_replay_block_transactions(BlockId::number(block_num)).await;
                assert!(result.is_ok());

                let traces = result.unwrap();
                similar_asserts::assert_eq!(
                    serde_json::to_string_pretty(&traces).unwrap().trim(),
                    r#"[
  {
    "output": "0x",
    "stateDiff": null,
    "trace": [
      {
        "type": "call",
        "action": {
          "from": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
          "callType": "call",
          "gas": "0x0",
          "input": "0x",
          "to": "0x0000000000000000000000000000000000000456",
          "value": "0x3e8"
        },
        "result": {
          "gasUsed": "0x0",
          "output": "0x"
        },
        "subtraces": 0,
        "traceAddress": []
      }
    ],
    "vmTrace": null,
    "transactionHash": "0x744426e308ba55f122913c74009be469da45153a941932d520aa959d8547da7b"
  }
]"#
                    .trim()
                );
            })
            .await;
        })
        .await;
    }
}
