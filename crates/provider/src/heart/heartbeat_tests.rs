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
        self.heartbeat.watch_tx(config, received_at_block).await.unwrap()
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
    let scenarios: [&[(u64, bool, bool)]; 3] = [
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
