use crate::{BlockLogs, Provider, ProviderBuilder};
use alloy_consensus::BlockHeader;
use alloy_eips::BlockNumberOrTag;
use alloy_network::BlockResponse as _;
use alloy_network_primitives::HeaderResponse;
use alloy_primitives::{B256, U64};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_eth::{Block, Filter, Log};
use alloy_transport::{
    layers::{RetryBackoffLayer, RetryPolicy},
    TransportError, TransportFut,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    task::Poll,
    time::Duration,
};

struct ChainState {
    blocks: HashMap<u64, Block>,
    logs: HashMap<B256, Vec<Log>>,
    head: u64,
    block_request_full: Vec<bool>,
    fail_logs: usize,
    reorg_after_log_success: Option<Vec<(Block, Vec<Log>)>>,
}

#[derive(Clone)]
pub(crate) struct MockChain {
    state: Arc<RwLock<ChainState>>,
}

impl MockChain {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ChainState {
                blocks: HashMap::new(),
                logs: HashMap::new(),
                head: 0,
                block_request_full: Vec::new(),
                fail_logs: 0,
                reorg_after_log_success: None,
            })),
        }
    }

    pub(crate) fn extend(&self, blocks: &[(Block, Vec<Log>)]) {
        let mut state = self.state.write().unwrap();
        for (block, logs) in blocks {
            let number = block.header.inner.number;
            state.logs.insert(block.header.hash, logs.clone());
            state.blocks.insert(number, block.clone());
            if number > state.head {
                state.head = number;
            }
        }
    }

    pub(crate) fn reorg(&self, blocks: &[(Block, Vec<Log>)]) {
        let mut state = self.state.write().unwrap();
        Self::apply_reorg(&mut state, blocks);
    }

    fn apply_reorg(state: &mut ChainState, blocks: &[(Block, Vec<Log>)]) {
        let min_height =
            blocks.iter().map(|(b, _)| b.header.inner.number).min().expect("reorg needs blocks");
        let removed_hashes: Vec<_> = state
            .blocks
            .iter()
            .filter_map(|(&height, block)| (height >= min_height).then_some(block.header.hash))
            .collect();
        state.blocks.retain(|&height, _| height < min_height);
        for hash in removed_hashes {
            state.logs.remove(&hash);
        }

        let mut max = state.head;
        for (block, logs) in blocks {
            let number = block.header.inner.number;
            state.logs.insert(block.header.hash, logs.clone());
            state.blocks.insert(number, block.clone());
            if number > max {
                max = number;
            }
        }
        state.head = max;
    }

    pub(crate) fn fail_next_logs(&self, count: usize) {
        self.state.write().unwrap().fail_logs += count;
    }

    pub(crate) fn reorg_after_next_log_success(&self, blocks: Vec<(Block, Vec<Log>)>) {
        self.state.write().unwrap().reorg_after_log_success = Some(blocks);
    }

    pub(crate) fn block_request_full_flags(&self) -> Vec<bool> {
        self.state.read().unwrap().block_request_full.clone()
    }

    pub(crate) fn provider(&self) -> impl Provider {
        let transport = MockChainTransport { chain: self.clone() };
        ProviderBuilder::new().connect_client(RpcClient::new(transport, true))
    }

    pub(crate) fn provider_with_retry(&self) -> impl Provider {
        #[derive(Clone, Debug)]
        struct AlwaysRetryPolicy;

        impl RetryPolicy for AlwaysRetryPolicy {
            fn should_retry(&self, _error: &TransportError) -> bool {
                true
            }

            fn backoff_hint(&self, _error: &TransportError) -> Option<Duration> {
                None
            }
        }

        let retry_layer = RetryBackoffLayer::new_with_policy(1, 0, 10_000, AlwaysRetryPolicy);
        let transport = MockChainTransport { chain: self.clone() };
        let client = RpcClient::builder().layer(retry_layer).transport(transport, true);
        ProviderBuilder::new().connect_client(client)
    }

    fn handle_request(&self, req: &alloy_json_rpc::SerializedRequest) -> alloy_json_rpc::Response {
        let mut state = self.state.write().unwrap();
        let payload = match req.method() {
            "eth_blockNumber" => {
                let raw = serde_json::to_string(&U64::from(state.head)).unwrap();
                alloy_json_rpc::ResponsePayload::Success(
                    serde_json::value::RawValue::from_string(raw).unwrap(),
                )
            }
            "eth_getBlockByNumber" => {
                let params = req.params().expect("eth_getBlockByNumber requires params");
                let (tag, full): (BlockNumberOrTag, bool) =
                    serde_json::from_str(params.get()).unwrap();
                state.block_request_full.push(full);
                let number = match tag {
                    BlockNumberOrTag::Number(n) => n,
                    BlockNumberOrTag::Latest => state.head,
                    _ => unimplemented!("unsupported block tag in MockChain: {tag:?}"),
                };
                let block = state.blocks.get(&number).cloned();
                let raw = serde_json::to_string(&block).unwrap();
                alloy_json_rpc::ResponsePayload::Success(
                    serde_json::value::RawValue::from_string(raw).unwrap(),
                )
            }
            "eth_getLogs" => {
                if state.fail_logs > 0 {
                    state.fail_logs -= 1;
                    alloy_json_rpc::ResponsePayload::internal_error_message(
                        "temporary log error".into(),
                    )
                } else {
                    let params = req.params().expect("eth_getLogs requires params");
                    let (filter,): (Filter,) = serde_json::from_str(params.get()).unwrap();
                    let hash = filter.get_block_hash().expect("logs are queried by block hash");
                    if let Some(logs) = state.logs.get(&hash).cloned() {
                        if let Some(blocks) = state.reorg_after_log_success.take() {
                            Self::apply_reorg(&mut state, &blocks);
                        }
                        let raw = serde_json::to_string(&logs).unwrap();
                        alloy_json_rpc::ResponsePayload::Success(
                            serde_json::value::RawValue::from_string(raw).unwrap(),
                        )
                    } else {
                        alloy_json_rpc::ResponsePayload::internal_error_message(
                            "block not found".into(),
                        )
                    }
                }
            }
            other => panic!("MockChain: unexpected RPC method `{other}`"),
        };
        alloy_json_rpc::Response { id: req.id().clone(), payload }
    }
}

#[derive(Clone)]
struct MockChainTransport {
    chain: MockChain,
}

impl tower::Service<alloy_json_rpc::RequestPacket> for MockChainTransport {
    type Response = alloy_json_rpc::ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: alloy_json_rpc::RequestPacket) -> Self::Future {
        let chain = self.chain.clone();
        Box::pin(async move {
            Ok(match req {
                alloy_json_rpc::RequestPacket::Single(req) => {
                    alloy_json_rpc::ResponsePacket::Single(chain.handle_request(&req))
                }
                alloy_json_rpc::RequestPacket::Batch(reqs) => {
                    alloy_json_rpc::ResponsePacket::Batch(
                        reqs.iter().map(|r| chain.handle_request(r)).collect(),
                    )
                }
            })
        })
    }
}

pub(crate) fn block(number: u64, hash_last_byte: u8, parent_hash_last_byte: u8) -> Block {
    let mut block: Block = Block::default();
    block.header.inner.number = number;
    block.header.hash = B256::with_last_byte(hash_last_byte);
    block.header.inner.parent_hash = B256::with_last_byte(parent_hash_last_byte);
    block
}

pub(crate) fn log(number: u64, hash_last_byte: u8, index: u64) -> Log {
    Log {
        block_hash: Some(B256::with_last_byte(hash_last_byte)),
        block_number: Some(number),
        log_index: Some(index),
        ..Default::default()
    }
}

pub(crate) fn assert_batch(
    block_logs: &BlockLogs<alloy_network::Ethereum>,
    number: u64,
    hash_last_byte: u8,
    removed: bool,
    log_count: usize,
) {
    let block_hash = B256::with_last_byte(hash_last_byte);
    assert_eq!(block_logs.block.header().number(), number);
    assert_eq!(block_logs.block.header().hash(), block_hash);
    assert_eq!(block_logs.logs.len(), log_count);

    for log in &block_logs.logs {
        assert_eq!(log.block_number, Some(number));
        assert_eq!(log.block_hash, Some(block_hash));
        assert_eq!(log.removed, removed);
    }
}
