use super::*;
use alloy_network::Ethereum;
use alloy_primitives::B256;
use alloy_rpc_types_eth::Block;

fn watcher(
    hash: TxHash,
    confirmations: u64,
    received: Option<u64>,
    deadline: Option<Instant>,
) -> (TxWatcher, PendingTransaction) {
    let (tx, rx) = oneshot::channel();
    (
        TxWatcher {
            config: PendingTransactionConfig::new(hash).with_required_confirmations(confirmations),
            received_at_block: received,
            tx,
            deadline,
        },
        PendingTransaction { tx_hash: hash, rx },
    )
}

fn heart() -> Heartbeat<Ethereum, futures::stream::Pending<Block>> {
    let client = alloy_rpc_client::RpcClient::mocked(alloy_transport::mock::Asserter::new());
    Heartbeat::new(futures::stream::pending(), Arc::default(), client.get_weak())
}

fn block(number: u64, hashes: Vec<TxHash>) -> Block {
    let mut block: Block = Block::default();
    block.header.number = number;
    block.transactions = alloy_rpc_types_eth::BlockTransactions::Hashes(hashes);
    block
}

#[tokio::test]
async fn duplicate_watchers_keep_independent_confirmations_and_deadlines() {
    let mut heart = heart();
    let hash = B256::repeat_byte(1);
    let (first, first_rx) = watcher(hash, 1, None, Some(Instant::now()));
    let (second, mut second_rx) = watcher(hash, 3, None, None);
    heart.handle_watch_ix(first);
    heart.handle_watch_ix(second);
    heart.reap_timeouts();
    assert!(matches!(
        first_rx.await,
        Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))
    ));
    assert!((&mut second_rx).now_or_never().is_none());
    heart.handle_new_block(block(1, vec![hash]));
    heart.handle_new_block(block(2, vec![]));
    assert!((&mut second_rx).now_or_never().is_none());
    heart.handle_new_block(block(3, vec![]));
    assert_eq!(second_rx.await.unwrap(), hash);
    assert!(!heart.has_pending_transactions());
}

#[tokio::test]
async fn timeouts_cover_already_mined_and_observed_transactions() {
    for already_mined in [false, true] {
        let mut heart = heart();
        let hash = B256::repeat_byte(2);
        let (watcher, pending) = watcher(hash, 3, already_mined.then_some(1), Some(Instant::now()));
        heart.handle_watch_ix(watcher);
        if !already_mined {
            heart.handle_new_block(block(1, vec![hash]));
        }
        heart.reap_timeouts();
        assert!(matches!(
            pending.await,
            Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))
        ));
        heart.update_pause_state();
        assert!(heart.paused.is_paused());
    }
}

#[tokio::test]
async fn recovery_respects_confirmations_and_releases_heartbeat() {
    let mut heart = heart();
    let hash = B256::repeat_byte(3);
    let (one, one_rx) = watcher(hash, 1, None, None);
    let (three, mut three_rx) = watcher(hash, 3, None, None);
    heart.handle_watch_ix(one);
    heart.handle_watch_ix(three);
    // Receipt available while the block stream and block-number RPC are stale.
    heart.handle_receipt(ReceiptCheck { hash, block: Some(10), height: Some(9) });
    assert_eq!(one_rx.await.unwrap(), hash);
    heart.handle_receipt(ReceiptCheck { hash, block: Some(10), height: Some(11) });
    assert!((&mut three_rx).now_or_never().is_none());
    heart.handle_receipt(ReceiptCheck { hash, block: Some(10), height: Some(12) });
    assert_eq!(three_rx.await.unwrap(), hash);
    heart.update_pause_state();
    assert!(heart.paused.is_paused());
}

#[tokio::test]
async fn reorg_reinclusion_uses_new_height_and_cancellation_cleans_up() {
    let mut heart = heart();
    let hash = B256::repeat_byte(4);
    let (watcher, mut pending) = watcher(hash, 3, None, None);
    heart.handle_watch_ix(watcher);
    heart.handle_new_block(block(10, vec![hash]));
    heart.handle_new_block(block(9, vec![]));
    assert_eq!(heart.unconfirmed[&hash][0].received_at_block, None);
    heart.handle_new_block(block(10, vec![]));
    heart.handle_new_block(block(11, vec![hash]));
    heart.handle_new_block(block(12, vec![]));
    assert!((&mut pending).now_or_never().is_none());
    drop(pending);
    heart.reap_timeouts();
    heart.update_pause_state();
    assert!(heart.paused.is_paused());
}

#[tokio::test]
async fn zero_confirmations_at_genesis_do_not_underflow() {
    for confirmations in [0, 1] {
        for known_receipt in [false, true] {
            let mut heart = heart();
            let hash = B256::repeat_byte(5);
            heart.handle_new_block(block(0, vec![hash]));
            let (watcher, pending) = watcher(hash, confirmations, known_receipt.then_some(0), None);
            heart.handle_watch_ix(watcher);
            assert_eq!(pending.now_or_never().unwrap().unwrap(), hash);
        }
    }
}

#[cfg(all(feature = "anvil-node", not(target_family = "wasm")))]
mod anvil {
    use super::*;
    use crate::{ext::AnvilApi, ProviderBuilder};
    use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
    use alloy_node_bindings::{Anvil, AnvilInstance};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_transport::{TransportErrorKind, TransportResult};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::sync::Semaphore;
    use tower::Service;

    struct Control {
        stale_head: AtomicBool,
        block_head: AtomicBool,
        head_gate: Semaphore,
        receipts: AtomicUsize,
        fail_receipts: AtomicBool,
        stall_receipts: AtomicBool,
    }

    fn provider(anvil: &AnvilInstance) -> (RootProvider, Arc<Control>) {
        let control = Arc::new(Control {
            stale_head: AtomicBool::new(false),
            block_head: AtomicBool::new(false),
            head_gate: Semaphore::new(0),
            receipts: AtomicUsize::new(0),
            fail_receipts: AtomicBool::new(false),
            stall_receipts: AtomicBool::new(false),
        });
        let state = control.clone();
        let http = alloy_transport_http::Http::new(anvil.endpoint_url());
        let transport = tower::service_fn(
            move |request: RequestPacket| -> alloy_transport::TransportFut<'static> {
                let mut http = http.clone();
                let state = state.clone();
                Box::pin(async move {
                    let RequestPacket::Single(ref single) = request else {
                        panic!("unexpected batch")
                    };
                    let receipt = single.method() == "eth_getTransactionReceipt";
                    if single.method() == "eth_blockNumber" {
                        if state.block_head.load(Ordering::SeqCst) {
                            state.head_gate.acquire().await.unwrap().forget();
                        }
                        if state.stale_head.load(Ordering::SeqCst) {
                            return Ok(ResponsePacket::Single(Response {
                                id: single.id().clone(),
                                payload: ResponsePayload::Success(
                                    serde_json::value::to_raw_value(&U64::ZERO).unwrap(),
                                ),
                            }));
                        }
                    }
                    if receipt && state.stall_receipts.load(Ordering::SeqCst) {
                        return futures::future::pending::<TransportResult<ResponsePacket>>().await;
                    }
                    if receipt && state.fail_receipts.load(Ordering::SeqCst) {
                        state.receipts.fetch_add(1, Ordering::SeqCst);
                        return Err(TransportErrorKind::custom_str(
                            "injected transient receipt failure",
                        ));
                    }
                    let response = http.call(request).await;
                    if receipt {
                        state.receipts.fetch_add(1, Ordering::SeqCst);
                    }
                    response
                })
            },
        );
        let client = RpcClient::builder().transport(transport, true);
        client.set_poll_interval(Duration::from_millis(25));
        (RootProvider::new(client), control)
    }

    async fn wait_for(mut condition: impl FnMut() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("condition did not become true");
    }

    async fn send(provider: &RootProvider, anvil: &AnvilInstance) -> TxHash {
        *provider
            .send_transaction(
                TransactionRequest::default()
                    .from(anvil.addresses()[0])
                    .to(anvil.addresses()[1])
                    .gas_limit(21_000)
                    .gas_price(20_000_000_000),
            )
            .await
            .unwrap()
            .tx_hash()
    }

    // Force the first block-number request to complete only after twenty blocks
    // have been mined. NewBlocks starts at tip-1, deterministically missing the tx.
    async fn missed_initial_block(api: u8) {
        let anvil = Anvil::new().spawn();
        let admin = RootProvider::<Ethereum>::new_http(anvil.endpoint_url());
        admin.anvil_set_auto_mine(false).await.unwrap();
        let (provider, control) = provider(&anvil);
        // Repeat with the same root provider to cover both startup and resuming after idle.
        for _ in 0..2 {
            let hash = send(&admin, &anvil).await;
            control.head_gate.forget_permits(control.head_gate.available_permits());
            control.block_head.store(true, Ordering::SeqCst);
            let before = control.receipts.load(Ordering::SeqCst);
            let builder = PendingTransactionBuilder::new(provider.clone(), hash)
                .with_required_confirmations(3);
            let task = tokio::spawn(async move {
                match api {
                    0 => builder.register().await.unwrap().await.unwrap(),
                    1 => builder.watch().await.unwrap(),
                    _ => builder.get_receipt().await.unwrap().transaction_hash,
                }
            });
            wait_for(|| control.receipts.load(Ordering::SeqCst) > before).await;
            admin.anvil_mine(Some(20), None).await.unwrap();
            control.block_head.store(false, Ordering::SeqCst);
            control.head_gate.add_permits(16);
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(5), task).await.unwrap().unwrap(),
                hash
            );
            // Successful receipt recovery must remove the heartbeat watcher, not leave it polling.
            tokio::time::sleep(Duration::from_millis(100)).await;
            let calls = control.receipts.load(Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(150)).await;
            assert_eq!(control.receipts.load(Ordering::SeqCst), calls);
        }
    }

    #[tokio::test]
    async fn register_recovers_missed_initial_block() {
        missed_initial_block(0).await;
    }
    #[tokio::test]
    async fn watch_recovers_missed_initial_block() {
        missed_initial_block(1).await;
    }
    #[tokio::test]
    async fn get_receipt_recovers_missed_initial_block() {
        missed_initial_block(2).await;
    }

    #[tokio::test]
    async fn stale_head_recovers_single_but_waits_for_multiple_confirmations() {
        let anvil = Anvil::new().spawn();
        let admin = RootProvider::<Ethereum>::new_http(anvil.endpoint_url());
        admin.anvil_set_auto_mine(false).await.unwrap();
        let hash = send(&admin, &anvil).await;
        let (provider, control) = provider(&anvil);
        control.stale_head.store(true, Ordering::SeqCst);
        let one = PendingTransactionBuilder::new(provider.clone(), hash).register().await.unwrap();
        let mut three = PendingTransactionBuilder::new(provider.clone(), hash)
            .with_required_confirmations(3)
            .register()
            .await
            .unwrap();
        admin.anvil_mine(Some(1), None).await.unwrap();
        assert_eq!(tokio::time::timeout(Duration::from_secs(5), one).await.unwrap().unwrap(), hash);
        assert!(tokio::time::timeout(Duration::from_millis(100), &mut three).await.is_err());
        control.stale_head.store(false, Ordering::SeqCst);
        admin.anvil_mine(Some(1), None).await.unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(100), &mut three).await.is_err());
        admin.anvil_mine(Some(1), None).await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), three).await.unwrap().unwrap(),
            hash
        );
    }

    #[tokio::test]
    async fn transient_receipt_errors_retry_and_stalled_rpcs_do_not_block_timeouts() {
        let anvil = Anvil::new().spawn();
        let admin = RootProvider::<Ethereum>::new_http(anvil.endpoint_url());
        admin.anvil_set_auto_mine(false).await.unwrap();
        let hash = send(&admin, &anvil).await;
        let (provider, control) = provider(&anvil);
        control.stale_head.store(true, Ordering::SeqCst);
        let pending =
            PendingTransactionBuilder::new(provider.clone(), hash).register().await.unwrap();
        control.fail_receipts.store(true, Ordering::SeqCst);
        let before = control.receipts.load(Ordering::SeqCst);
        admin.anvil_mine(Some(1), None).await.unwrap();
        wait_for(|| control.receipts.load(Ordering::SeqCst) >= before + 2).await;
        control.fail_receipts.store(false, Ordering::SeqCst);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending).await.unwrap().unwrap(),
            hash
        );

        let pending = PendingTransactionBuilder::new(provider.clone(), hash)
            .with_required_confirmations(100)
            .with_timeout(Some(Duration::from_millis(100)))
            .register()
            .await
            .unwrap();
        control.stall_receipts.store(true, Ordering::SeqCst);
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), pending).await.unwrap(),
            Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))
        ));
    }

    #[tokio::test]
    async fn stalled_head_does_not_delay_single_confirmation_for_duplicate_hash() {
        let anvil = Anvil::new().spawn();
        let admin = RootProvider::<Ethereum>::new_http(anvil.endpoint_url());
        admin.anvil_set_auto_mine(false).await.unwrap();
        let hash = send(&admin, &anvil).await;
        let (provider, control) = provider(&anvil);
        control.block_head.store(true, Ordering::SeqCst);
        let one = PendingTransactionBuilder::new(provider.clone(), hash).register().await.unwrap();
        let three = PendingTransactionBuilder::new(provider.clone(), hash)
            .with_required_confirmations(3)
            .with_timeout(Some(Duration::from_millis(250)))
            .register()
            .await
            .unwrap();
        admin.anvil_mine(Some(1), None).await.unwrap();
        assert_eq!(tokio::time::timeout(Duration::from_secs(2), one).await.unwrap().unwrap(), hash);
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), three).await.unwrap(),
            Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))
        ));
    }

    #[tokio::test]
    async fn already_mined_confirmation_wait_times_out() {
        let anvil = Anvil::new().spawn();
        let admin = RootProvider::<Ethereum>::new_http(anvil.endpoint_url());
        admin.anvil_set_auto_mine(false).await.unwrap();
        let hash = send(&admin, &anvil).await;
        admin.anvil_mine(Some(1), None).await.unwrap();
        assert!(admin.get_transaction_receipt(hash).await.unwrap().is_some());
        let (provider, _) = provider(&anvil);
        let pending = PendingTransactionBuilder::new(provider.clone(), hash)
            .with_required_confirmations(3)
            .with_timeout(Some(Duration::from_millis(100)));
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), pending.watch()).await.unwrap(),
            Err(PendingTransactionError::TxWatcher(WatchTxError::Timeout))
        ));
    }

    #[tokio::test]
    #[cfg(feature = "ws-base")]
    async fn websocket_receipt_recovery_without_block_notifications() {
        let anvil = Anvil::new().spawn();
        let provider = RootProvider::<Ethereum>::connect(&anvil.ws_endpoint()).await.unwrap();
        provider.client().set_poll_interval(Duration::from_millis(25));
        provider.anvil_set_auto_mine(false).await.unwrap();
        let hash = send(&provider, &anvil).await;
        let paused = Arc::new(Paused::default());
        // Model a missed subscription notification without depending on socket scheduling.
        let heart = Heartbeat::<Ethereum, _>::new(
            futures::stream::pending::<Block>(),
            paused.clone(),
            provider.weak_client(),
        )
        .spawn();
        let pending = heart
            .watch_tx(PendingTransactionConfig::new(hash).with_required_confirmations(3), None)
            .await
            .unwrap();
        provider.anvil_mine(Some(3), None).await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending).await.unwrap().unwrap(),
            hash
        );
        wait_for(|| paused.is_paused()).await;
    }

    #[tokio::test]
    async fn fast_blocks_slower_http_polling() {
        let provider = ProviderBuilder::new()
            .connect_anvil_with_wallet_and_config(|anvil| anvil.block_time_f64(0.1))
            .unwrap();
        provider.client().set_poll_interval(Duration::from_millis(500));
        for confirmations in [0, 1, 3] {
            let pending = provider
                .send_transaction(
                    TransactionRequest::default()
                        .to(alloy_primitives::Address::ZERO)
                        .value(alloy_primitives::U256::from(1)),
                )
                .await
                .unwrap()
                .with_required_confirmations(confirmations);
            let hash = *pending.tx_hash();
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(10), pending.watch())
                    .await
                    .unwrap()
                    .unwrap(),
                hash
            );
        }
    }
}
