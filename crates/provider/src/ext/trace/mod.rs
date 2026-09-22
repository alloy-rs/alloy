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
    ///
    /// A successful `null` response becomes `None`. RPC errors are propagated unchanged.
    async fn trace_transaction(
        &self,
        hash: TxHash,
    ) -> TransportResult<Option<Vec<LocalizedTransactionTrace>>>;

    /// Returns the transaction trace at the given `traceAddress` path.
    ///
    /// An empty path selects the root, `[0]` selects its first child, and `[0, 1]` selects that
    /// child's second child. A successful `null` response becomes `None`.
    ///
    /// Older nodes may interpret a single index as a flat position instead of a tree path.
    /// For a flat index, fetch [`Self::trace_transaction`] and index the returned traces locally.
    /// Nodes that return RPC errors for missing transactions or paths retain those errors.
    async fn trace_get(
        &self,
        hash: TxHash,
        indices: &[usize],
    ) -> TransportResult<Option<LocalizedTransactionTrace>>;

    /// Trace the given raw transaction.
    fn trace_raw_transaction<'a>(&self, data: &'a [u8]) -> TraceBuilder<&'a [u8], TraceResults>;

    /// Traces matching given filter.
    async fn trace_filter(
        &self,
        tracer: &TraceFilter,
    ) -> TransportResult<Vec<LocalizedTransactionTrace>>;

    /// Trace all transactions in the given block.
    ///
    /// A successful `null` response becomes `None`. Some nodes, including Reth, instead return
    /// an RPC error for missing blocks; those errors are propagated unchanged.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn trace_block(
        &self,
        block: BlockId,
    ) -> TransportResult<Option<Vec<LocalizedTransactionTrace>>>;

    /// Replays a transaction. Returns `None` when the node returns `null` for a missing
    /// transaction.
    ///
    /// Default trace type is [`TraceType::Trace`].
    /// The returned transaction hash is `None` on older nodes that omit it.
    /// RPC errors, including unavailable history, are propagated unchanged.
    fn trace_replay_transaction(
        &self,
        hash: TxHash,
    ) -> TraceBuilder<TxHash, Option<TraceResultsWithTransactionHash<Option<TxHash>>>>;

    /// Replays all transactions in the given block.
    ///
    /// A successful `null` response becomes `None`; an empty array becomes `Some(vec![])`.
    /// RPC errors, including missing blocks or unavailable history, are propagated unchanged.
    ///
    /// Default trace type is [`TraceType::Trace`].
    fn trace_replay_block_transactions(
        &self,
        block: BlockId,
    ) -> TraceBuilder<BlockId, Option<Vec<TraceResultsWithTransactionHash>>>;
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
    ) -> TransportResult<Option<Vec<LocalizedTransactionTrace>>> {
        self.client().request("trace_transaction", (hash,)).await
    }

    async fn trace_get(
        &self,
        hash: TxHash,
        indices: &[usize],
    ) -> TransportResult<Option<LocalizedTransactionTrace>> {
        let indices: Vec<Index> = indices.iter().copied().map(Index::from).collect();
        self.client().request("trace_get", (hash, indices)).await
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

    async fn trace_block(
        &self,
        block: BlockId,
    ) -> TransportResult<Option<Vec<LocalizedTransactionTrace>>> {
        self.client().request("trace_block", (block,)).await
    }

    fn trace_replay_transaction(
        &self,
        hash: TxHash,
    ) -> TraceBuilder<TxHash, Option<TraceResultsWithTransactionHash<Option<TxHash>>>> {
        TraceBuilder::new_rpc(self.client().request("trace_replayTransaction", hash))
    }

    fn trace_replay_block_transactions(
        &self,
        block: BlockId,
    ) -> TraceBuilder<BlockId, Option<Vec<TraceResultsWithTransactionHash>>> {
        TraceBuilder::new_rpc(self.client().request("trace_replayBlockTransactions", block))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ext::test::async_ci_only, ProviderBuilder};
    use alloy_eips::{BlockNumberOrTag, Encodable2718};
    use alloy_network::{EthereumWallet, NetworkTransactionBuilder, TransactionBuilder};
    use alloy_node_bindings::{utils::run_with_tempdir, Reth};
    use alloy_primitives::{address, U256};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_signer_local::PrivateKeySigner;
    use alloy_transport::mock::Asserter;

    #[tokio::test]
    async fn trace_block() {
        let provider = ProviderBuilder::new().connect_anvil();
        let traces =
            provider.trace_block(BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap().unwrap();
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

                let traces = result.unwrap().unwrap();
                if let Some(transaction_hash) = traces.transaction_hash {
                    assert_eq!(transaction_hash, hash);
                }
                let traces = traces.full_trace;
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

    #[tokio::test]
    async fn trace_get_serializes_tree_paths() {
        use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
        use alloy_network::Ethereum;
        use alloy_rpc_client::RpcClient;
        use alloy_transport::TransportFut;

        for path in [vec![], vec![0], vec![0, 1], vec![6, 0], vec![16]] {
            let expected_path = path.clone();
            let service = tower::service_fn(move |request: RequestPacket| {
                let expected_path = expected_path.clone();
                Box::pin(async move {
                    let RequestPacket::Single(request) = request else {
                        panic!("expected a single trace_get request");
                    };
                    assert_eq!(request.method(), "trace_get");
                    let params: serde_json::Value =
                        serde_json::from_str(request.params().unwrap().get()).unwrap();
                    let wire_path: Vec<_> =
                        expected_path.iter().map(|index| format!("0x{index:x}")).collect();
                    assert_eq!(params, serde_json::json!([TxHash::ZERO, wire_path]));

                    let trace = serde_json::json!({
                        "type": "call",
                        "action": {
                            "from": alloy_primitives::Address::ZERO,
                            "to": alloy_primitives::Address::ZERO,
                            "callType": "call", "gas": "0x0", "input": "0x", "value": "0x0"
                        },
                        "result": {"gasUsed": "0x0", "output": "0x"},
                        "subtraces": 0,
                        "traceAddress": expected_path
                    });
                    Ok(ResponsePacket::Single(Response {
                        id: request.id().clone(),
                        payload: ResponsePayload::Success(
                            serde_json::value::to_raw_value(&trace).unwrap(),
                        ),
                    }))
                }) as TransportFut<'static>
            });
            let provider = crate::RootProvider::<Ethereum>::new(RpcClient::new(service, true));
            let trace = provider.trace_get(TxHash::ZERO, &path).await.unwrap().unwrap();
            assert_eq!(trace.trace.trace_address, path);
        }
    }

    #[tokio::test]
    async fn trace_get_missing_tree_paths() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        for path in [vec![], vec![0], vec![6, 0]] {
            asserter.push_success(&serde_json::Value::Null);
            assert!(provider.trace_get(TxHash::ZERO, &path).await.unwrap().is_none());
        }
    }

    #[tokio::test]
    async fn trace_get_preserves_rpc_errors() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        for code in [-32601, -32001, 4444] {
            asserter.push_failure(alloy_json_rpc::ErrorPayload {
                code,
                message: "lookup failed".into(),
                data: None,
            });
            let error = provider.trace_get(TxHash::ZERO, &[0, 1]).await.unwrap_err();
            assert_eq!(error.as_error_resp().unwrap().code, code);
        }
    }

    #[tokio::test]
    async fn trace_replay_preserves_transaction_hash() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        let hash = TxHash::with_last_byte(1);
        let replay = serde_json::json!({
            "output": "0x1234", "trace": [], "stateDiff": null, "vmTrace": null,
            "transactionHash": hash
        });
        asserter.push_success(&replay);
        let result = provider.trace_replay_transaction(hash).await.unwrap().unwrap();
        assert_eq!(result.transaction_hash, Some(hash));
        assert_eq!(serde_json::to_value(result).unwrap(), replay);

        // Block replay items retain the required transaction hash.
        asserter.push_success(&serde_json::json!([replay]));
        let results =
            provider.trace_replay_block_transactions(BlockId::latest()).await.unwrap().unwrap();
        assert_eq!(results[0].transaction_hash, hash);
    }

    #[tokio::test]
    async fn trace_replay_handles_absence_and_legacy_nodes() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        asserter.push_success(&serde_json::Value::Null);
        assert!(provider.trace_replay_transaction(TxHash::ZERO).await.unwrap().is_none());
        let replay =
            serde_json::json!({"output": "0x", "trace": [], "stateDiff": null, "vmTrace": null});
        asserter.push_success(&replay);
        let result = provider.trace_replay_transaction(TxHash::ZERO).await.unwrap().unwrap();
        assert!(result.transaction_hash.is_none());
        assert_eq!(serde_json::to_value(result.full_trace).unwrap(), replay);
        for code in [-32601, -32001, 4444] {
            asserter.push_failure(alloy_json_rpc::ErrorPayload {
                code,
                message: "lookup failed".into(),
                data: None,
            });
            let error = provider.trace_replay_transaction(TxHash::ZERO).await.unwrap_err();
            assert_eq!(error.as_error_resp().unwrap().code, code);
        }
    }

    #[tokio::test]
    async fn trace_collections_distinguish_null_and_empty() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        for (wire, expected) in
            [(serde_json::Value::Null, None), (serde_json::json!([]), Some(vec![]))]
        {
            for _ in 0..3 {
                asserter.push_success(&wire);
            }
            assert_eq!(provider.trace_transaction(TxHash::ZERO).await.unwrap(), expected);
            assert_eq!(provider.trace_block(BlockId::latest()).await.unwrap(), expected);
            let replay = provider.trace_replay_block_transactions(BlockId::latest()).await.unwrap();
            assert_eq!(replay.map(|traces| traces.len()), expected.map(|traces| traces.len()));
        }
    }

    #[tokio::test]
    async fn trace_collections_preserve_errors() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        for code in [-32601, -32001, 4444] {
            for _ in 0..3 {
                asserter.push_failure(alloy_json_rpc::ErrorPayload {
                    code,
                    message: "lookup failed".into(),
                    data: None,
                });
            }
            let errors = [
                provider.trace_transaction(TxHash::ZERO).await.unwrap_err(),
                provider.trace_block(BlockId::latest()).await.unwrap_err(),
                provider.trace_replay_block_transactions(BlockId::latest()).await.unwrap_err(),
            ];
            for error in errors {
                assert_eq!(error.as_error_resp().unwrap().code, code);
            }
        }
    }
}
