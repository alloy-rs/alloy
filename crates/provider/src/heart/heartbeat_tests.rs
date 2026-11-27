use super::*;
use alloy_network::Ethereum;
use alloy_rpc_types_eth::{Block, BlockTransactions};

struct Harness {
    blocks: futures::channel::mpsc::UnboundedSender<Block>,
    heartbeat: HeartbeatHandle,
    paused: Arc<Paused>,
}

impl Harness {
    fn new() -> Self {
        let (blocks, stream) = futures::channel::mpsc::unbounded();
        let paused = Arc::<Paused>::default();
        let heartbeat = Heartbeat::<Ethereum, _>::new(stream, paused.clone()).spawn();
        Self { blocks, heartbeat, paused }
    }

    fn block(&self, number: u64, txs: &[TxHash]) {
        let mut block: Block = Block::default();
        block.header.inner.number = number;
        block.transactions = BlockTransactions::Hashes(txs.to_vec());
        self.blocks.unbounded_send(block).unwrap();
    }

    async fn watch(
        &self,
        config: PendingTransactionConfig,
        received_at_block: Option<u64>,
    ) -> PendingTransaction {
        self.heartbeat
            .watch_tx(config, received_at_block.map(|number| BlockNumHash::new(number, B256::ZERO)))
            .await
            .unwrap()
    }
}

/// With the clock paused, this only returns once every other task is idle.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}

fn poll(pending: &mut PendingTransaction) -> Option<Result<TxHash, PendingTransactionError>> {
    pending.now_or_never()
}

const TX: TxHash = B256::with_last_byte(1);
const OTHER: TxHash = B256::with_last_byte(9);

#[tokio::test(start_paused = true)]
async fn confirms_after_required_confirmations() {
    // (required confirmations, block including the tx, block at which the watcher resolves)
    for (confirmations, mined_in, resolves_at) in [(1, 2, 2), (2, 2, 3), (3, 2, 4)] {
        let harness = Harness::new();
        let config = PendingTransactionConfig::new(TX).with_required_confirmations(confirmations);
        let mut pending = harness.watch(config, None).await;

        let mut resolved = None;
        for number in 1..=resolves_at {
            harness.block(number, if number == mined_in { &[TX] } else { &[] });
            settle().await;
            resolved = poll(&mut pending);
            assert_eq!(resolved.is_some(), number == resolves_at, "{confirmations}: {number}");
        }
        assert_eq!(resolved.unwrap().unwrap(), TX);
    }
}

#[tokio::test(start_paused = true)]
async fn already_mined_tx_counts_confirmations_from_its_block() {
    // (received_at_block, required confirmations, resolves before block 4)
    let cases = [
        (None, 1, true),
        (None, 2, true),
        (None, 3, false),
        (Some(2), 2, true),
        (Some(2), 3, false),
    ];
    for (received_at_block, confirmations, immediately) in cases {
        let harness = Harness::new();
        // Keep the heartbeat unpaused so it records the blocks.
        let _keepalive = harness.watch(PendingTransactionConfig::new(OTHER), None).await;
        harness.block(1, &[]);
        harness.block(2, &[TX]);
        harness.block(3, &[]);
        settle().await;

        let config = PendingTransactionConfig::new(TX).with_required_confirmations(confirmations);
        let mut pending = harness.watch(config, received_at_block).await;
        settle().await;
        let mut resolved = poll(&mut pending);
        assert_eq!(resolved.is_some(), immediately, "{received_at_block:?}, {confirmations}");

        if !immediately {
            harness.block(4, &[]);
            settle().await;
            resolved = poll(&mut pending);
        }
        assert_eq!(resolved.expect("not confirmed after block 4").unwrap(), TX);
    }
}

#[tokio::test(start_paused = true)]
async fn timeout_reaps_unconfirmed_tx() {
    let harness = Harness::new();
    let other = B256::with_last_byte(2);
    let config = PendingTransactionConfig::new(TX).with_timeout(Some(Duration::ZERO));
    let timed_out = harness.watch(config, None).await;
    let mut pending = harness.watch(PendingTransactionConfig::new(other), None).await;

    let res = tokio::time::timeout(Duration::from_secs(60), timed_out).await.expect("not reaped");
    assert!(
        matches!(res, Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))),
        "{res:?}"
    );

    settle().await;
    assert!(poll(&mut pending).is_none());
    harness.block(1, &[TX, other]);
    settle().await;
    assert_eq!(poll(&mut pending).unwrap().unwrap(), other);
}

#[tokio::test(start_paused = true)]
async fn chain_gap_resets_txs_included_at_or_above_it() {
    // (block number, includes the tx, watcher resolved after this block), 3 confirmations
    let scenarios: [&[(u64, bool, bool)]; 4] = [
        &[(1, true, false), (2, false, false), (2, false, false), (3, false, true)],
        &[
            (1, false, false),
            (2, true, false),
            (3, false, false),
            (2, false, false),
            (3, true, false),
            (4, false, false),
            (5, false, true),
        ],
        &[(1, true, false), (5, false, true)],
        // Reincluded higher after a reorg, then reorged out again before the new deadline.
        &[
            (1, false, false),
            (2, true, false),
            (2, false, false),
            (3, false, false),
            (4, true, false),
            (4, false, false),
            (5, false, false),
            (6, false, false),
            (7, true, false),
            (8, false, false),
            (9, false, true),
        ],
    ];
    for steps in scenarios {
        let harness = Harness::new();
        let config = PendingTransactionConfig::new(TX).with_required_confirmations(3);
        let mut pending = harness.watch(config, None).await;

        for (i, &(number, includes_tx, resolves)) in steps.iter().enumerate() {
            harness.block(number, if includes_tx { &[TX] } else { &[] });
            settle().await;
            let resolved = poll(&mut pending);
            assert_eq!(resolved.is_some(), resolves, "{steps:?}: step {i}");
            if let Some(res) = resolved {
                assert_eq!(res.unwrap(), TX);
            }
        }
    }
}

#[tokio::test(start_paused = true)]
async fn lookbehind_keeps_last_ten_canonical_blocks() {
    // (blocks as (number, includes the tx), tx found when registered afterwards)
    let cases: [(Vec<(u64, bool)>, bool); 4] = [
        ((1..=11).map(|n| (n, n == 2)).collect(), true),
        ((1..=11).map(|n| (n, n == 1)).collect(), false),
        (vec![(1, true), (2, false), (3, false), (2, false)], true),
        (vec![(1, false), (2, true), (3, false), (2, false)], false),
    ];
    for (blocks, found) in cases {
        let harness = Harness::new();
        // Keep the heartbeat unpaused so it records the blocks.
        let _keepalive = harness.watch(PendingTransactionConfig::new(OTHER), None).await;
        for &(number, includes_tx) in &blocks {
            harness.block(number, if includes_tx { &[TX] } else { &[] });
        }
        settle().await;

        let mut pending = harness.watch(PendingTransactionConfig::new(TX), None).await;
        settle().await;
        assert_eq!(poll(&mut pending).is_some(), found, "{blocks:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn ignores_blocks_while_paused() {
    let harness = Harness::new();
    let mut pending = harness.watch(PendingTransactionConfig::new(OTHER), None).await;
    harness.block(1, &[OTHER]);
    settle().await;
    assert!(poll(&mut pending).is_some());
    assert!(harness.paused.is_paused());

    // A block the stream had buffered before pausing can be reorged out, so it must not be
    // matched later.
    harness.block(2, &[TX]);
    settle().await;
    let config = PendingTransactionConfig::new(TX).with_timeout(Some(Duration::ZERO));
    let res = harness.watch(config, None).await.await;
    assert!(
        matches!(res, Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))),
        "{res:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn pauses_while_nothing_is_watched() {
    let harness = Harness::new();
    settle().await;
    assert!(harness.paused.is_paused());

    let mut pending = harness.watch(PendingTransactionConfig::new(TX), None).await;
    settle().await;
    assert!(!harness.paused.is_paused());

    harness.block(1, &[TX]);
    settle().await;
    assert_eq!(poll(&mut pending).unwrap().unwrap(), TX);
    assert!(harness.paused.is_paused());
}

#[tokio::test(start_paused = true)]
async fn second_watcher_for_same_tx_does_not_replace_first() {
    let harness = Harness::new();
    let first = harness.watch(PendingTransactionConfig::new(TX), None).await;
    let second = harness.watch(PendingTransactionConfig::new(TX), None).await;

    harness.block(1, &[TX]);
    assert_eq!(first.await.unwrap(), TX);
    assert_eq!(second.await.unwrap(), TX);
}

#[tokio::test(start_paused = true)]
async fn chain_gap_keeps_every_watcher_for_same_tx() {
    let harness = Harness::new();
    // Keep the heartbeat unpaused so it records block 1.
    let _keepalive = harness.watch(PendingTransactionConfig::new(OTHER), None).await;
    harness.block(1, &[TX]);
    settle().await;
    let config = PendingTransactionConfig::new(TX).with_required_confirmations(3);
    let first = harness.watch(config.clone(), Some(1)).await;
    let second = harness.watch(config, Some(1)).await;

    harness.block(1, &[]);
    for number in 2..=4 {
        harness.block(number, if number == 2 { &[TX] } else { &[] });
    }
    assert_eq!(first.await.unwrap(), TX);
    assert_eq!(second.await.unwrap(), TX);
}

#[tokio::test(start_paused = true)]
async fn timeout_reaps_only_the_watcher_that_set_it() {
    let harness = Harness::new();
    let config = PendingTransactionConfig::new(TX);
    let mut pending = harness.watch(config.clone(), None).await;
    let timed_out = harness.watch(config.with_timeout(Some(Duration::ZERO)), None).await;

    let res = tokio::time::timeout(Duration::from_secs(60), timed_out).await.expect("not reaped");
    assert!(
        matches!(res, Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))),
        "{res:?}"
    );

    settle().await;
    let res = poll(&mut pending);
    assert!(res.is_none(), "{res:?}");
    harness.block(1, &[TX]);
    assert_eq!(pending.await.unwrap(), TX);
}

#[tokio::test(start_paused = true)]
async fn timeout_applies_while_waiting_for_confirmations() {
    for received_at_block in [None, Some(1)] {
        let harness = Harness::new();
        // Keep the heartbeat unpaused so it records block 1.
        let _keepalive = harness.watch(PendingTransactionConfig::new(OTHER), None).await;
        harness.block(1, &[TX]);
        settle().await;

        let config = PendingTransactionConfig::new(TX)
            .with_required_confirmations(3)
            .with_timeout(Some(Duration::ZERO));
        let pending = harness.watch(config, received_at_block).await;
        let res = tokio::time::timeout(Duration::from_secs(60), pending)
            .await
            .unwrap_or_else(|_| panic!("{received_at_block:?}: watcher never timed out"));
        assert!(
            matches!(res, Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))),
            "{received_at_block:?}: {res:?}"
        );
        harness.block(2, &[OTHER]);
        settle().await;
        assert!(harness.paused.is_paused(), "{received_at_block:?}");
    }
}

/// Watchers registered at the same instant (coarse clocks, e.g. browser timers) must each keep
/// their own timeout.
#[test]
fn timeouts_registered_at_the_same_instant_reap_every_watcher() {
    let (_blocks, stream) = futures::channel::mpsc::unbounded::<Block>();
    let mut heartbeat = Heartbeat::<Ethereum, _>::new(stream, Arc::default());
    let now = Instant::now();
    let mut receivers = Vec::new();
    for tx_hash in [B256::with_last_byte(1), B256::with_last_byte(2)] {
        let (tx, rx) = oneshot::channel();
        let config = PendingTransactionConfig::new(tx_hash).with_timeout(Some(Duration::ZERO));
        let watcher = TxWatcher {
            config,
            received_at_block: None,
            reap_at: None,
            check_past_blocks: true,
            tx,
        };
        heartbeat.handle_watch_ix_at(watcher, now);
        receivers.push(rx);
    }

    std::thread::sleep(Duration::from_millis(1));
    heartbeat.reap_timeouts();

    for mut rx in receivers {
        assert!(
            matches!(
                rx.try_recv(),
                Ok(Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout)))
            ),
            "watcher not reaped"
        );
    }
    assert!(!heartbeat.has_pending_transactions());
}

#[test]
fn does_not_pause_while_watches_are_queued() {
    let (_blocks, stream) = futures::channel::mpsc::unbounded::<Block>();
    let mut heartbeat = Heartbeat::<Ethereum, _>::new(stream, Arc::default());
    heartbeat.past_blocks.push_back((BlockNumHash::new(1, B256::ZERO), [TX].into_iter().collect()));
    let (ix_tx, ixns) = mpsc::channel(1);
    let (tx, _rx) = oneshot::channel();
    let config = PendingTransactionConfig::new(TX);
    ix_tx
        .try_send(TxWatcher {
            config,
            received_at_block: None,
            reap_at: None,
            check_past_blocks: true,
            tx,
        })
        .unwrap();

    heartbeat.update_pause_state(&ixns);
    assert!(!heartbeat.paused.is_paused());
    assert_eq!(heartbeat.past_blocks.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn receipt_seed_from_another_fork_does_not_confirm() {
    use crate::{PendingTransactionBuilder, ProviderBuilder};
    use alloy_primitives::U64;
    use alloy_transport::mock::Asserter;

    let asserter = Asserter::new();
    let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
    let receipt = chain_receipt(10, Some(11));
    asserter.push_success(&receipt);
    asserter.push_success(&U64::from(11));
    for number in 10..=11 {
        let mut block: Block = Block::default();
        block.header.inner.number = number;
        block.header.hash = B256::with_last_byte(number as u8 + 20);
        block.header.inner.parent_hash = B256::with_last_byte(number as u8 + 19);
        asserter.push_success(&Some(block));
    }

    let mut tip: Block = Block::default();
    tip.header.inner.number = 11;
    tip.header.hash = B256::with_last_byte(31);
    asserter.push_success(&Some(tip));
    let mut inclusion: Block = Block::default();
    inclusion.header.inner.number = 10;
    inclusion.header.hash = B256::with_last_byte(30);
    asserter.push_success(&Some(inclusion));
    asserter.push_success(&Option::<serde_json::Value>::None);

    // The receipt belongs to fork A, but the lazy heartbeat's very first samples are
    // continuous blocks from fork B, neither of which contains the transaction.
    let mut pending = PendingTransactionBuilder::new(provider.root().clone(), TX)
        .with_required_confirmations(2)
        .register()
        .await
        .unwrap();
    settle().await;
    assert!(poll(&mut pending).is_none(), "orphaned receipt falsely confirmed");
    assert!(asserter.read_q().is_empty(), "replacement fork was not sampled");
}

/// A method-aware RPC fixture, so background polling cannot consume validation responses.
struct CanonicalChain {
    blocks: BTreeMap<u64, Block>,
    tip: u64,
    initial_receipt: Option<serde_json::Value>,
    receipt: Option<serde_json::Value>,
    requests: Vec<(String, serde_json::Value)>,
    stall_validation: bool,
}

fn chain_block(number: u64, hash: u8, parent: u8, txs: &[TxHash]) -> Block {
    let mut block: Block = Block::default();
    block.header.inner.number = number;
    block.header.hash = B256::with_last_byte(hash);
    block.header.inner.parent_hash = B256::with_last_byte(parent);
    block.transactions = BlockTransactions::Hashes(txs.to_vec());
    block
}

fn chain_receipt(number: u64, hash: Option<u8>) -> serde_json::Value {
    serde_json::json!({
        "transactionHash": TX,
        "transactionIndex": "0x0",
        "blockHash": hash.map(B256::with_last_byte),
        "blockNumber": alloy_primitives::U64::from(number),
        "from": alloy_primitives::Address::ZERO,
        "to": null,
        "contractAddress": null,
        "gasUsed": "0x5208",
        "cumulativeGasUsed": "0x5208",
        "effectiveGasPrice": "0x1",
        "status": "0x1",
        "logs": [],
        "logsBloom": alloy_primitives::Bloom::ZERO,
        "type": "0x0"
    })
}

impl CanonicalChain {
    fn provider(self) -> (RootProvider, Arc<std::sync::Mutex<Self>>) {
        use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
        use alloy_rpc_client::RpcClient;
        use alloy_transport::TransportFut;
        let chain = Arc::new(std::sync::Mutex::new(self));
        let rpc_chain = chain.clone();
        let service = tower::service_fn(move |packet: RequestPacket| {
            let chain = rpc_chain.clone();
            Box::pin(async move {
                let RequestPacket::Single(request) = packet else { panic!("unexpected batch") };
                let params: serde_json::Value =
                    serde_json::from_str(request.params().unwrap().get()).unwrap();
                let response = {
                    let mut chain = chain.lock().unwrap();
                    chain.requests.push((request.method().to_owned(), params.clone()));
                    match request.method() {
                        "eth_getTransactionReceipt" => Some(
                            serde_json::to_value(
                                chain.initial_receipt.take().or_else(|| chain.receipt.clone()),
                            )
                            .unwrap(),
                        ),
                        "eth_blockNumber" => Some(
                            serde_json::to_value(alloy_primitives::U64::from(chain.tip)).unwrap(),
                        ),
                        "eth_getBlockByNumber" => {
                            let number: alloy_primitives::U64 =
                                serde_json::from_value(params[0].clone()).unwrap();
                            let repeated = chain
                                .requests
                                .iter()
                                .filter(|(method, previous)| {
                                    method == "eth_getBlockByNumber" && previous == &params
                                })
                                .count()
                                > 1;
                            if chain.stall_validation && repeated {
                                None
                            } else {
                                Some(serde_json::to_value(chain.blocks.get(&number.to())).unwrap())
                            }
                        }
                        other => panic!("unexpected RPC: {other}"),
                    }
                };
                let Some(response) = response else { return pending().await };
                Ok(ResponsePacket::Single(Response {
                    id: request.id().clone(),
                    payload: ResponsePayload::Success(
                        serde_json::value::to_raw_value(&response).unwrap(),
                    ),
                }))
            }) as TransportFut<'static>
        });
        let client = RpcClient::new(service, true);
        client.set_poll_interval(Duration::from_secs(1));
        (RootProvider::new(client), chain)
    }
}

#[tokio::test(start_paused = true)]
async fn receipt_seeded_public_watches_require_canonical_inclusion() {
    // Exercise both public entry points, missing hashes, hash mismatch, matching hashes with
    // missing membership, and an inclusion block outside the heartbeat's initial samples.
    for register in [false, true] {
        for (receipt_hash, canonical_hash, tip) in
            [(Some(11), 30, 11), (Some(11), 30, 14), (Some(30), 30, 11), (None, 30, 11)]
        {
            let blocks = (9..=tip)
                .map(|number| {
                    let hash = (canonical_hash as u64 + number - 10) as u8;
                    (number, chain_block(number, hash, hash - 1, &[]))
                })
                .collect();
            let (provider, chain) = CanonicalChain {
                blocks,
                tip,
                initial_receipt: Some(chain_receipt(10, receipt_hash)),
                receipt: None,
                requests: Vec::new(),
                stall_validation: false,
            }
            .provider();
            let builder =
                PendingTransactionBuilder::new(provider.clone(), TX).with_required_confirmations(2);
            let mut watch = Box::pin(async move {
                if register {
                    builder.register().await?.await
                } else {
                    builder.watch().await
                }
            });
            assert!(watch.as_mut().now_or_never().is_none());
            settle().await;
            assert!(
                watch.as_mut().now_or_never().is_none(),
                "orphaned receipt confirmed: {register}, {receipt_hash:?}, {tip}"
            );

            // Even after the old numeric deadline, it must remain pending.
            {
                let mut chain = chain.lock().unwrap();
                let number = tip + 1;
                let hash = (canonical_hash as u64 + number - 10) as u8;
                chain.blocks.insert(number, chain_block(number, hash, hash - 1, &[]));
                chain.tip = number;
            }
            tokio::time::advance(Duration::from_secs(1)).await;
            settle().await;
            assert!(watch.as_mut().now_or_never().is_none());

            // A later canonical inclusion starts its own confirmation count.
            let inclusion = tip + 2;
            let hash = canonical_hash + (inclusion - 10) as u8;
            {
                let mut chain = chain.lock().unwrap();
                chain.blocks.insert(inclusion, chain_block(inclusion, hash, hash - 1, &[TX]));
                chain.receipt = Some(chain_receipt(inclusion, Some(hash)));
                chain.tip = inclusion;
            }
            tokio::time::advance(Duration::from_secs(1)).await;
            settle().await;
            assert!(watch.as_mut().now_or_never().is_none());
            {
                let mut chain = chain.lock().unwrap();
                chain.blocks.insert(inclusion + 1, chain_block(inclusion + 1, hash + 1, hash, &[]));
                chain.tip = inclusion + 1;
            }
            tokio::time::advance(Duration::from_secs(1)).await;
            settle().await;
            assert_eq!(
                watch.as_mut().now_or_never().expect("canonical tx never confirmed").unwrap(),
                TX
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn receipt_seeded_watch_handles_same_height_replacement() {
    let (provider, chain) = CanonicalChain {
        blocks: [(9, chain_block(9, 9, 8, &[])), (10, chain_block(10, 10, 9, &[TX]))].into(),
        tip: 10,
        initial_receipt: Some(chain_receipt(10, Some(10))),
        receipt: Some(chain_receipt(10, Some(10))),
        requests: Vec::new(),
        stall_validation: false,
    }
    .provider();
    let mut pending = PendingTransactionBuilder::new(provider, TX)
        .with_required_confirmations(3)
        .register()
        .await
        .unwrap();
    settle().await;
    assert!(poll(&mut pending).is_none());
    {
        let mut chain = chain.lock().unwrap();
        chain.blocks.insert(10, chain_block(10, 20, 9, &[]));
        chain.receipt = None;
    }
    for number in 11..=13 {
        {
            let mut chain = chain.lock().unwrap();
            chain
                .blocks
                .insert(number, chain_block(number, number as u8 + 10, number as u8 + 9, &[]));
            chain.tip = number;
        }
        tokio::time::advance(Duration::from_secs(1)).await;
        settle().await;
        assert!(poll(&mut pending).is_none(), "same-height orphan confirmed at {number}");
    }
    for number in 14..=16 {
        {
            let mut chain = chain.lock().unwrap();
            chain.blocks.insert(
                number,
                chain_block(
                    number,
                    number as u8 + 10,
                    number as u8 + 9,
                    if number == 14 { &[TX] } else { &[] },
                ),
            );
            chain.tip = number;
        }
        tokio::time::advance(Duration::from_secs(1)).await;
        settle().await;
        let result = poll(&mut pending);
        assert_eq!(result.is_some(), number == 16);
        if let Some(result) = result {
            assert_eq!(result.unwrap(), TX);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn historical_receipt_confirmation_has_bounded_rpc_cost() {
    let (provider, chain) = CanonicalChain {
        blocks: [
            (2, chain_block(2, 2, 1, &[TX])),
            (24, chain_block(24, 24, 23, &[])),
            (25, chain_block(25, 25, 24, &[])),
        ]
        .into(),
        tip: 25,
        initial_receipt: Some(chain_receipt(2, Some(2))),
        receipt: Some(chain_receipt(2, Some(2))),
        requests: Vec::new(),
        stall_validation: false,
    }
    .provider();
    let mut pending = PendingTransactionBuilder::new(provider, TX)
        .with_required_confirmations(3)
        .register()
        .await
        .unwrap();
    settle().await;
    assert_eq!(poll(&mut pending).unwrap().unwrap(), TX);
    let chain = chain.lock().unwrap();
    let blocks: Vec<_> = chain
        .requests
        .iter()
        .filter(|(method, _)| method == "eth_getBlockByNumber")
        .map(|(_, params)| params[0].as_str().unwrap())
        .collect();
    // Two initial samples and exactly two canonical checks; no historical block scan.
    assert_eq!(blocks, ["0x18", "0x19", "0x18", "0x2"]);
}

#[tokio::test(start_paused = true)]
async fn validation_timeout_starts_before_the_task_is_polled() {
    let (provider, _chain) = CanonicalChain {
        blocks: [(10, chain_block(10, 10, 9, &[TX])), (11, chain_block(11, 11, 10, &[]))].into(),
        tip: 11,
        initial_receipt: None,
        receipt: None,
        requests: Vec::new(),
        stall_validation: false,
    }
    .provider();
    let (tx, rx) = oneshot::channel();
    tx.send(Ok(TxConfirmation {
        inclusion: BlockNumHash::new(10, B256::with_last_byte(10)),
        tip: BlockNumHash::new(11, B256::with_last_byte(11)),
    }))
    .unwrap();
    let config = PendingTransactionConfig::new(TX)
        .with_required_confirmations(2)
        .with_timeout(Some(Duration::from_secs(5)));
    let mut pending =
        PendingTransaction { tx_hash: TX, rx }.with_confirmation_check(provider, config);

    // Pass the deadline before the spawned validator gets its first poll.
    tokio::time::advance(Duration::from_secs(6)).await;
    settle().await;
    let result = poll(&mut pending);
    assert!(
        matches!(result, Some(Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout)))),
        "{result:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn single_confirmation_receipt_needs_no_canonical_rpcs() {
    let (provider, chain) = CanonicalChain {
        blocks: BTreeMap::new(),
        tip: 11,
        initial_receipt: Some(chain_receipt(10, Some(10))),
        receipt: None,
        requests: Vec::new(),
        stall_validation: false,
    }
    .provider();
    let pending = PendingTransactionBuilder::new(provider, TX).register().await.unwrap();
    assert_eq!(pending.await.unwrap(), TX);
    let chain = chain.lock().unwrap();
    let methods: Vec<_> = chain.requests.iter().map(|(method, _)| method.as_str()).collect();
    assert_eq!(methods, ["eth_getTransactionReceipt"]);
}

#[tokio::test(start_paused = true)]
async fn parent_hash_change_resets_confirmations_at_continuous_heights() {
    let harness = Harness::new();
    let config = PendingTransactionConfig::new(TX).with_required_confirmations(3);
    let mut pending = harness.watch(config, None).await;
    for (number, hash, parent, includes_tx) in [
        (1, 1, 0, true),
        (2, 12, 11, false),
        (3, 13, 12, false),
        (4, 14, 13, true),
        (5, 15, 14, false),
        (6, 16, 15, false),
    ] {
        harness
            .blocks
            .unbounded_send(chain_block(
                number,
                hash,
                parent,
                if includes_tx { &[TX] } else { &[] },
            ))
            .unwrap();
        settle().await;
        let result = poll(&mut pending);
        assert_eq!(result.is_some(), number == 6, "confirmed at block {number}");
        if let Some(result) = result {
            assert_eq!(result.unwrap(), TX);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn canonical_validation_obeys_original_timeout() {
    let (provider, chain) = CanonicalChain {
        blocks: [(10, chain_block(10, 10, 9, &[TX])), (11, chain_block(11, 11, 10, &[]))].into(),
        tip: 11,
        initial_receipt: Some(chain_receipt(10, Some(10))),
        receipt: Some(chain_receipt(10, Some(10))),
        requests: Vec::new(),
        stall_validation: true,
    }
    .provider();
    let mut pending = PendingTransactionBuilder::new(provider, TX)
        .with_required_confirmations(2)
        .with_timeout(Some(Duration::from_secs(5)))
        .register()
        .await
        .unwrap();
    settle().await;
    assert!(poll(&mut pending).is_none());
    assert_eq!(
        chain
            .lock()
            .unwrap()
            .requests
            .iter()
            .filter(|(method, _)| method == "eth_getBlockByNumber")
            .count(),
        3
    );
    tokio::time::advance(Duration::from_secs(5)).await;
    settle().await;
    let result = poll(&mut pending).expect("validation RPC bypassed the watch timeout");
    assert!(matches!(result, Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))));
}

#[tokio::test(start_paused = true)]
async fn validation_rejects_a_same_height_tip_replacement() {
    let (provider, chain) = CanonicalChain {
        blocks: [(10, chain_block(10, 20, 19, &[])), (11, chain_block(11, 21, 20, &[]))].into(),
        tip: 11,
        initial_receipt: None,
        receipt: None,
        requests: Vec::new(),
        stall_validation: false,
    }
    .provider();
    let (tx, rx) = oneshot::channel();
    let pending = PendingTransaction { tx_hash: TX, rx };
    let config = PendingTransactionConfig::new(TX).with_required_confirmations(2);
    let mut pending = pending.with_confirmation_check(provider, config);
    // The heartbeat reached its deadline on fork A just before the same-height replacement.
    tx.send(Ok(TxConfirmation {
        inclusion: BlockNumHash::new(10, B256::with_last_byte(10)),
        tip: BlockNumHash::new(11, B256::with_last_byte(11)),
    }))
    .unwrap();
    settle().await;
    assert!(poll(&mut pending).is_none());
    assert!(chain
        .lock()
        .unwrap()
        .requests
        .iter()
        .any(|(method, _)| method == "eth_getTransactionReceipt"));
    for number in 12..=13 {
        {
            let mut chain = chain.lock().unwrap();
            chain.blocks.insert(
                number,
                chain_block(
                    number,
                    number as u8 + 10,
                    number as u8 + 9,
                    if number == 12 { &[TX] } else { &[] },
                ),
            );
            chain.tip = number;
        }
        tokio::time::advance(Duration::from_secs(1)).await;
        settle().await;
        let result = poll(&mut pending);
        assert_eq!(result.is_some(), number == 13);
        if let Some(result) = result {
            assert_eq!(result.unwrap(), TX);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn dropping_pending_cancels_stalled_validation() {
    let (provider, chain) = CanonicalChain {
        blocks: [(10, chain_block(10, 10, 9, &[TX])), (11, chain_block(11, 11, 10, &[]))].into(),
        tip: 11,
        initial_receipt: Some(chain_receipt(10, Some(10))),
        receipt: Some(chain_receipt(10, Some(10))),
        requests: Vec::new(),
        stall_validation: true,
    }
    .provider();
    let weak = provider.weak_client();
    let pending = PendingTransactionBuilder::new(provider, TX)
        .with_required_confirmations(2)
        .register()
        .await
        .unwrap();
    settle().await;
    assert!(weak.upgrade().is_some());
    let requests = chain.lock().unwrap().requests.len();
    drop(pending);
    settle().await;
    assert!(weak.upgrade().is_none(), "cancelled validator retained the provider");
    assert_eq!(chain.lock().unwrap().requests.len(), requests);
}

#[tokio::test(start_paused = true)]
async fn reorg_reconciliation_does_not_renew_timeout() {
    let (provider, _chain) = CanonicalChain {
        blocks: [(9, chain_block(9, 19, 18, &[])), (10, chain_block(10, 20, 19, &[TX]))].into(),
        tip: 10,
        initial_receipt: None,
        receipt: Some(chain_receipt(10, Some(20))),
        requests: Vec::new(),
        stall_validation: false,
    }
    .provider();
    let (tx, rx) = oneshot::channel();
    let pending = PendingTransaction { tx_hash: TX, rx };
    let config = PendingTransactionConfig::new(TX)
        .with_required_confirmations(3)
        .with_timeout(Some(Duration::from_secs(5)));
    let mut pending = pending.with_confirmation_check(provider, config);
    settle().await;
    tokio::time::advance(Duration::from_secs(4)).await;
    tx.send(Ok(TxConfirmation {
        inclusion: BlockNumHash::new(10, B256::with_last_byte(10)),
        tip: BlockNumHash::new(12, B256::with_last_byte(12)),
    }))
    .unwrap();
    settle().await;
    assert!(poll(&mut pending).is_none());
    tokio::time::advance(Duration::from_secs(1)).await;
    settle().await;
    assert!(matches!(
        poll(&mut pending),
        Some(Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout)))
    ));
}

#[cfg(all(feature = "anvil-api", feature = "reqwest"))]
#[tokio::test]
async fn receipt_seeded_confirmation_over_http_and_ws() {
    use crate::{ext::AnvilApi, ProviderBuilder};
    use alloy_node_bindings::Anvil;
    use alloy_primitives::U256;
    use alloy_rpc_types_eth::TransactionRequest;

    for ws in [false, true] {
        if ws && !cfg!(feature = "ws-base") {
            continue;
        }
        let anvil = Anvil::new().spawn();
        let endpoint = if ws { anvil.ws_endpoint() } else { anvil.endpoint() };
        let provider = ProviderBuilder::new().connect(&endpoint).await.unwrap();
        provider.client().set_poll_interval(Duration::from_millis(10));
        let accounts = provider.get_accounts().await.unwrap();
        let builder = provider
            .send_transaction(
                TransactionRequest::default()
                    .from(accounts[0])
                    .to(accounts[1])
                    .value(U256::from(1)),
            )
            .await
            .unwrap();
        let tx_hash = *builder.tx_hash();
        tokio::time::timeout(Duration::from_secs(2), async {
            while provider.get_transaction_receipt(tx_hash).await.unwrap().is_none() {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("transaction was not mined");
        let mut pending = builder.with_required_confirmations(3).register().await.unwrap();

        assert!(tokio::time::timeout(Duration::from_millis(50), &mut pending).await.is_err());
        provider.anvil_mine(Some(1), None).await.unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(50), &mut pending).await.is_err());
        provider.anvil_mine(Some(1), None).await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), pending)
                .await
                .expect("canonical inclusion did not confirm")
                .unwrap(),
            tx_hash
        );
    }
}
