#![allow(dead_code, unreachable_pub)] // TODO: remove
//! Block Hearbeat and Transaction Watcher

use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    time::Duration,
};

use alloy_primitives::{B256, U256};
use alloy_rpc_types::Block;
use alloy_transport::utils::Spawnable;
use futures::{stream::StreamExt, FutureExt, Stream};
use tokio::{
    select,
    sync::{mpsc, oneshot, watch},
};

/// A configuration object for watching for transaction confirmation.
pub struct WatchConfig {
    /// The transaction hash to watch for.
    tx_hash: B256,

    /// Require a number of confirmations.
    confirmations: u64,

    /// Optional timeout for the transaction.
    timeout: Option<Duration>,

    /// Notify the waiter.
    tx: oneshot::Sender<()>,
}

impl WatchConfig {
    /// Create a new watch for a transaction.
    pub fn new(tx_hash: B256, tx: oneshot::Sender<()>) -> Self {
        Self { tx_hash, confirmations: 0, timeout: Default::default(), tx }
    }

    /// Set the number of confirmations to wait for.
    pub fn with_confirmations(mut self, confirmations: u64) -> Self {
        self.confirmations = confirmations;
        self
    }

    /// Set the timeout for the transaction.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Notify the waiter.
    fn notify(self) {
        let _ = self.tx.send(());
    }
}

/// A pending transaction that can be awaited.
pub struct PendingTransaction {
    /// The transaction hash.
    tx_hash: B256,
    /// The receiver for the notification.
    // TODO: send a receipt?
    rx: oneshot::Receiver<()>,
}

impl PendingTransaction {
    pub fn tx_hash(&self) -> B256 {
        self.tx_hash
    }
}

impl Future for PendingTransaction {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.rx.poll_unpin(cx).map(|res| res.unwrap())
    }
}

/// A handle to the heartbeat task.
#[derive(Debug, Clone)]
pub struct HeartbeatHandle {
    tx: mpsc::Sender<WatchConfig>,
    latest: watch::Receiver<Block>,
}

impl HeartbeatHandle {
    /// Get a new watcher that always sees the latest block.
    pub fn latest(&self) -> watch::Receiver<Block> {
        self.latest.clone()
    }
}

// TODO: Parameterize with `Network`
/// A heartbeat task that receives blocks and watches for transactions.
pub(crate) struct Heartbeat<St> {
    /// The stream of incoming blocks to watch.
    stream: futures::stream::Fuse<St>,

    /// Transactions to watch for.
    unconfirmed: HashMap<B256, WatchConfig>,

    /// Ordered map of transactions waiting for confirmations.
    waiting_confs: BTreeMap<U256, WatchConfig>,

    /// Ordered map of transactions to reap at a certain time.
    reap_at: BTreeMap<std::time::Instant, B256>,
}

impl<St> Heartbeat<St>
where
    St: Stream<Item = Block> + Unpin + Send + 'static,
{
    /// Create a new heartbeat task.
    pub fn new(stream: St) -> Self {
        Self {
            stream: stream.fuse(),
            unconfirmed: Default::default(),
            waiting_confs: Default::default(),
            reap_at: Default::default(),
        }
    }
}

impl<St> Heartbeat<St> {
    /// Check if any transactions have enough confirmations to notify.
    fn check_confirmations(&mut self, latest: &watch::Sender<Block>) {
        if let Some(current_height) = { latest.borrow().header.number } {
            let to_keep = self.waiting_confs.split_off(&current_height);
            let to_notify = std::mem::replace(&mut self.waiting_confs, to_keep);

            for (_, watcher) in to_notify.into_iter() {
                watcher.notify();
            }
        }
    }

    /// Get the next time to reap a transaction. If no reaps, this is a very
    /// long time from now (i.e. will not be woken).
    fn next_reap(&self) -> std::time::Instant {
        self.reap_at
            .keys()
            .next()
            .copied()
            .unwrap_or_else(|| std::time::Instant::now() + Duration::from_secs(60_000))
    }

    /// Reap any timeout
    fn reap_timeouts(&mut self) {
        let now = std::time::Instant::now();
        let to_keep = self.reap_at.split_off(&now);
        let to_reap = std::mem::replace(&mut self.reap_at, to_keep);

        for tx_hash in to_reap.values() {
            self.unconfirmed.remove(tx_hash);
        }
    }

    /// Handle a watch instruction by adding it to the watch list, and
    /// potentially adding it to our `reap_at` list.
    fn handle_watch_ix(&mut self, to_watch: WatchConfig) {
        // start watching for the tx
        if let Some(timeout) = to_watch.timeout {
            self.reap_at.insert(std::time::Instant::now() + timeout, to_watch.tx_hash);
        }
        self.unconfirmed.insert(to_watch.tx_hash, to_watch);
    }

    /// Handle a new block by checking if any of the transactions we're
    /// watching are in it, and if so, notifying the watcher. Also updates
    /// the latest block.
    fn handle_new_block(&mut self, block: Block, latest: &watch::Sender<Block>) {
        // Blocks without numbers are ignored, as they're not part of the chain.
        let block_height = match block.header.number {
            Some(height) => height,
            None => return,
        };

        // check if we are watching for any of the txns in this block
        let to_check = block
            .transactions
            .hashes()
            .copied()
            .filter_map(|tx_hash| self.unconfirmed.remove(&tx_hash));

        // If confirmations is 1 or less, notify the watcher
        for watcher in to_check {
            if watcher.confirmations <= 1 {
                watcher.notify();
                continue;
            }
            // Otherwise add it to the waiting list
            self.waiting_confs.insert(block_height + U256::from(watcher.confirmations), watcher);
        }

        // update the latest block. We use `send_replace` here to ensure the
        // latest block is always up to date, even if no receivers exist
        // C.f.
        // https://docs.rs/tokio/latest/tokio/sync/watch/struct.Sender.html#method.send
        let _ = latest.send_replace(block);

        self.check_confirmations(latest);
    }
}

impl<St> Heartbeat<St>
where
    St: Stream<Item = Block> + Unpin + Send + 'static,
{
    /// Spawn the heartbeat task, returning a [`HeartbeatHandle`]
    pub(crate) fn spawn(mut self, from: Block) -> HeartbeatHandle {
        let (latest, latest_rx) = watch::channel(from);
        let (ix_tx, mut ixns) = mpsc::channel(16);

        let fut = async move {
            'shutdown: loop {
                {
                    // We bias the select so that we always handle new messages
                    // before checking blocks, and reap timeouts are last.
                    let next_reap = self.next_reap();
                    select! {
                        biased;

                        // Watch for new transactions.
                        ix_opt = ixns.recv() => {
                            match ix_opt {
                                Some(to_watch) => self.handle_watch_ix(to_watch),
                                None => break 'shutdown, // ix channel is closed
                            }
                        },

                        // Wake up to handle new blocks
                        block = self.stream.select_next_some() => {
                            self.handle_new_block(block, &latest);
                        },

                        // This arm ensures we always wake up to reap timeouts,
                        // even if there are no other events.
                        _ = tokio::time::sleep_until(next_reap.into()) => {},
                    }
                }

                // Always reap timeouts
                self.reap_timeouts();
            }
        };

        fut.spawn_task();
        HeartbeatHandle { tx: ix_tx, latest: latest_rx }
    }
}
