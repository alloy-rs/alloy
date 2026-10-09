//! Block heartbeat and pending transaction watcher.

use crate::{blocks::Paused, Provider, RootProvider};
use alloy_consensus::BlockHeader;
use alloy_json_rpc::RpcError;
use alloy_network::{BlockResponse, Network, ReceiptResponse};
use alloy_primitives::{
    map::{B256HashMap, B256HashSet},
    TxHash, U64,
};
use alloy_rpc_client::WeakClient;
use alloy_transport::{utils::Spawnable, TransportError};
use futures::{
    stream::{FusedStream, StreamExt},
    FutureExt, Stream,
};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    future::Future,
    sync::Arc,
    time::Duration,
};
use tokio::{
    select,
    sync::{mpsc, oneshot},
};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::{
    std::Instant,
    tokio::{sleep_until, timeout},
};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use {
    std::time::Instant,
    tokio::time::{sleep_until, timeout},
};

#[cfg(not(target_family = "wasm"))]
use futures::stream::BoxStream;
#[cfg(target_family = "wasm")]
use futures::stream::LocalBoxStream as BoxStream;

/// Errors which may occur when watching a pending transaction.
#[derive(Debug, thiserror::Error)]
pub enum PendingTransactionError {
    /// Failed to register pending transaction in heartbeat.
    #[error("failed to register pending transaction to watch")]
    FailedToRegister,

    /// Underlying transport error.
    #[error(transparent)]
    TransportError(#[from] TransportError),

    /// Error occurred while getting response from the heartbeat.
    #[error(transparent)]
    Recv(#[from] oneshot::error::RecvError),

    /// Errors that may occur when watching a transaction.
    #[error(transparent)]
    TxWatcher(#[from] WatchTxError),
}

/// A builder for configuring a pending transaction watcher.
///
/// # Examples
///
/// Send and wait for a transaction to be confirmed 2 times, with a timeout of 60 seconds:
///
/// ```no_run
/// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider, tx: alloy_rpc_types_eth::transaction::TransactionRequest) -> Result<(), Box<dyn std::error::Error>> {
/// // Send a transaction, and configure the pending transaction.
/// let builder = provider.send_transaction(tx)
///     .await?
///     .with_required_confirmations(2)
///     .with_timeout(Some(std::time::Duration::from_secs(60)));
/// // Register the pending transaction with the provider.
/// let pending_tx = builder.register().await?;
/// // Wait for the transaction to be confirmed 2 times.
/// let tx_hash = pending_tx.await?;
/// # Ok(())
/// # }
/// ```
///
/// This can also be more concisely written using `watch`:
/// ```no_run
/// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider, tx: alloy_rpc_types_eth::transaction::TransactionRequest) -> Result<(), Box<dyn std::error::Error>> {
/// let tx_hash = provider.send_transaction(tx)
///     .await?
///     .with_required_confirmations(2)
///     .with_timeout(Some(std::time::Duration::from_secs(60)))
///     .watch()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "this type does nothing unless you call `register`, `watch` or `get_receipt`"]
#[derive(Debug)]
#[doc(alias = "PendingTxBuilder")]
pub struct PendingTransactionBuilder<N: Network> {
    config: PendingTransactionConfig,
    provider: RootProvider<N>,
}

impl<N: Network> PendingTransactionBuilder<N> {
    /// Creates a new pending transaction builder.
    pub const fn new(provider: RootProvider<N>, tx_hash: TxHash) -> Self {
        Self::from_config(provider, PendingTransactionConfig::new(tx_hash))
    }

    /// Creates a new pending transaction builder from the given configuration.
    pub const fn from_config(provider: RootProvider<N>, config: PendingTransactionConfig) -> Self {
        Self { config, provider }
    }

    /// Returns the inner configuration.
    pub const fn inner(&self) -> &PendingTransactionConfig {
        &self.config
    }

    /// Consumes this builder, returning the inner configuration.
    pub fn into_inner(self) -> PendingTransactionConfig {
        self.config
    }

    /// Returns the provider.
    pub const fn provider(&self) -> &RootProvider<N> {
        &self.provider
    }

    /// Consumes this builder, returning the provider and the configuration.
    pub fn split(self) -> (RootProvider<N>, PendingTransactionConfig) {
        (self.provider, self.config)
    }

    /// Calls a function with a reference to the value.
    pub fn inspect<F: FnOnce(&Self)>(self, f: F) -> Self {
        f(&self);
        self
    }

    /// Returns the transaction hash.
    #[doc(alias = "transaction_hash")]
    pub const fn tx_hash(&self) -> &TxHash {
        self.config.tx_hash()
    }

    /// Sets the transaction hash.
    #[doc(alias = "set_transaction_hash")]
    pub const fn set_tx_hash(&mut self, tx_hash: TxHash) {
        self.config.set_tx_hash(tx_hash);
    }

    /// Sets the transaction hash.
    #[doc(alias = "with_transaction_hash")]
    pub const fn with_tx_hash(mut self, tx_hash: TxHash) -> Self {
        self.config.tx_hash = tx_hash;
        self
    }

    /// Returns the number of confirmations to wait for.
    #[doc(alias = "confirmations")]
    pub const fn required_confirmations(&self) -> u64 {
        self.config.required_confirmations()
    }

    /// Sets the number of confirmations to wait for.
    #[doc(alias = "set_confirmations")]
    pub const fn set_required_confirmations(&mut self, confirmations: u64) {
        self.config.set_required_confirmations(confirmations);
    }

    /// Sets the number of confirmations to wait for.
    #[doc(alias = "with_confirmations")]
    pub const fn with_required_confirmations(mut self, confirmations: u64) -> Self {
        self.config.required_confirmations = confirmations;
        self
    }

    /// Returns the timeout.
    pub const fn timeout(&self) -> Option<Duration> {
        self.config.timeout()
    }

    /// Sets the timeout.
    pub const fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.config.set_timeout(timeout);
    }

    /// Sets the timeout.
    pub const fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Registers the watching configuration with the provider.
    ///
    /// This does not wait for the transaction to be confirmed, but returns a [`PendingTransaction`]
    /// that can be awaited at a later moment.
    ///
    /// See:
    /// - [`watch`](Self::watch) for watching the transaction without fetching the receipt.
    /// - [`get_receipt`](Self::get_receipt) for fetching the receipt after the transaction has been
    ///   confirmed.
    #[doc(alias = "build")]
    pub async fn register(self) -> Result<PendingTransaction, PendingTransactionError> {
        self.provider.watch_pending_transaction(self.config).await
    }

    /// Waits for the transaction to confirm with the given number of confirmations.
    ///
    /// See:
    /// - [`register`](Self::register): for registering the transaction without waiting for it to be
    ///   confirmed.
    /// - [`get_receipt`](Self::get_receipt) for fetching the receipt after the transaction has been
    ///   confirmed.
    pub async fn watch(self) -> Result<TxHash, PendingTransactionError> {
        self.register().await?.await
    }

    /// Waits for the transaction to confirm with the given number of confirmations, and
    /// then fetches its receipt.
    ///
    /// Note that this method will call `eth_getTransactionReceipt` on the [**root
    /// provider**](RootProvider), and not on a specific network provider. This means that any
    /// overrides or customizations made to the network provider will not be used.
    ///
    /// See:
    /// - [`register`](Self::register): for registering the transaction without waiting for it to be
    ///   confirmed.
    /// - [`watch`](Self::watch) for watching the transaction without fetching the receipt.
    pub async fn get_receipt(self) -> Result<N::ReceiptResponse, PendingTransactionError> {
        let hash = self.config.tx_hash;
        self.provider.watch_pending_transaction(self.config).await?.await?;
        self.provider.get_transaction_receipt(hash).await?.ok_or_else(|| RpcError::NullResp.into())
    }
}

/// Configuration for watching a pending transaction.
///
/// This type can be used to create a [`PendingTransactionBuilder`], but in general it is only used
/// internally.
#[must_use = "this type does nothing unless you call `with_provider`"]
#[derive(Clone, Debug)]
#[doc(alias = "PendingTxConfig", alias = "TxPendingConfig")]
pub struct PendingTransactionConfig {
    /// The transaction hash to watch for.
    #[doc(alias = "transaction_hash")]
    tx_hash: TxHash,

    /// Require a number of confirmations.
    required_confirmations: u64,

    /// Optional timeout for the transaction.
    timeout: Option<Duration>,
}

impl PendingTransactionConfig {
    /// Create a new watch for a transaction.
    pub const fn new(tx_hash: TxHash) -> Self {
        Self { tx_hash, required_confirmations: 1, timeout: None }
    }

    /// Returns the transaction hash.
    #[doc(alias = "transaction_hash")]
    pub const fn tx_hash(&self) -> &TxHash {
        &self.tx_hash
    }

    /// Sets the transaction hash.
    #[doc(alias = "set_transaction_hash")]
    pub const fn set_tx_hash(&mut self, tx_hash: TxHash) {
        self.tx_hash = tx_hash;
    }

    /// Sets the transaction hash.
    #[doc(alias = "with_transaction_hash")]
    pub const fn with_tx_hash(mut self, tx_hash: TxHash) -> Self {
        self.tx_hash = tx_hash;
        self
    }

    /// Returns the number of confirmations to wait for.
    #[doc(alias = "confirmations")]
    pub const fn required_confirmations(&self) -> u64 {
        self.required_confirmations
    }

    /// Sets the number of confirmations to wait for.
    #[doc(alias = "set_confirmations")]
    pub const fn set_required_confirmations(&mut self, confirmations: u64) {
        self.required_confirmations = confirmations;
    }

    /// Sets the number of confirmations to wait for.
    #[doc(alias = "with_confirmations")]
    pub const fn with_required_confirmations(mut self, confirmations: u64) -> Self {
        self.required_confirmations = confirmations;
        self
    }

    /// Returns the timeout.
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Sets the timeout.
    pub const fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    /// Sets the timeout.
    pub const fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Wraps this configuration with a provider to expose watching methods.
    pub const fn with_provider<N: Network>(
        self,
        provider: RootProvider<N>,
    ) -> PendingTransactionBuilder<N> {
        PendingTransactionBuilder::from_config(provider, self)
    }
}

impl From<TxHash> for PendingTransactionConfig {
    fn from(tx_hash: TxHash) -> Self {
        Self::new(tx_hash)
    }
}

/// Errors which may occur in heartbeat when watching a transaction.
#[derive(Debug, thiserror::Error)]
pub enum WatchTxError {
    /// Transaction was not confirmed after configured timeout.
    #[error("transaction was not confirmed within the timeout")]
    Timeout,
}

/// The type sent by the [`HeartbeatHandle`] to the [`Heartbeat`] background task.
#[doc(alias = "TransactionWatcher")]
struct TxWatcher {
    config: PendingTransactionConfig,
    /// The block at which the transaction was received. To be filled once known.
    /// Invariant: any confirmed transaction in `Heart` has this value set.
    received_at_block: Option<u64>,
    tx: oneshot::Sender<Result<(), WatchTxError>>,
    /// Each registration has its own deadline, including duplicate transaction hashes.
    deadline: Option<Instant>,
}

impl TxWatcher {
    /// Notify the waiter.
    fn notify(self, result: Result<(), WatchTxError>) {
        debug!(tx=%self.config.tx_hash, "notifying");
        let _ = self.tx.send(result);
    }
}

/// Represents a transaction that is yet to be confirmed a specified number of times.
///
/// This struct is a future created by [`PendingTransactionBuilder`] that resolves to the
/// transaction hash once the underlying transaction has been confirmed the specified number of
/// times in the network.
#[doc(alias = "PendingTx", alias = "TxPending")]
pub struct PendingTransaction {
    /// The transaction hash.
    #[doc(alias = "transaction_hash")]
    pub(crate) tx_hash: TxHash,
    /// The receiver for the notification.
    // TODO: send a receipt?
    pub(crate) rx: oneshot::Receiver<Result<(), WatchTxError>>,
}

impl fmt::Debug for PendingTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingTransaction").field("tx_hash", &self.tx_hash).finish()
    }
}

impl PendingTransaction {
    /// Creates a ready pending transaction.
    pub fn ready(tx_hash: TxHash) -> Self {
        let (tx, rx) = oneshot::channel();
        tx.send(Ok(())).ok(); // Make sure that the receiver is notified already.
        Self { tx_hash, rx }
    }

    /// Returns this transaction's hash.
    #[doc(alias = "transaction_hash")]
    pub const fn tx_hash(&self) -> &TxHash {
        &self.tx_hash
    }
}

impl Future for PendingTransaction {
    type Output = Result<TxHash, PendingTransactionError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.rx.poll_unpin(cx).map(|res| {
            res??;
            Ok(self.tx_hash)
        })
    }
}

/// A handle to the heartbeat task.
#[derive(Clone, Debug)]
pub(crate) struct HeartbeatHandle {
    tx: mpsc::Sender<TxWatcher>,
}

impl HeartbeatHandle {
    /// Watch for a transaction to be confirmed with the given config.
    #[doc(alias = "watch_transaction")]
    pub(crate) async fn watch_tx(
        &self,
        config: PendingTransactionConfig,
        received_at_block: Option<u64>,
    ) -> Result<PendingTransaction, PendingTransactionConfig> {
        let (tx, rx) = oneshot::channel();
        let tx_hash = config.tx_hash;
        let deadline = config.timeout.map(|timeout| Instant::now() + timeout);
        match self.tx.send(TxWatcher { config, received_at_block, tx, deadline }).await {
            Ok(()) => Ok(PendingTransaction { tx_hash, rx }),
            Err(e) => Err(e.0.config),
        }
    }
}

/// A receipt observed independently of the block stream.
struct ReceiptCheck {
    hash: TxHash,
    block: Option<u64>,
    height: Option<u64>,
}

type ReceiptChecks = futures::stream::Fuse<BoxStream<'static, ReceiptCheck>>;

/// A heartbeat task that receives blocks and watches for transactions.
pub(crate) struct Heartbeat<N, S> {
    /// The stream of incoming blocks to watch.
    stream: futures::stream::Fuse<S>,

    /// Lookbehind blocks in form of mapping block number -> vector of transaction hashes.
    past_blocks: VecDeque<(u64, B256HashSet)>,

    /// Transactions to watch for.
    unconfirmed: B256HashMap<Vec<TxWatcher>>,

    /// Ordered map of transactions waiting for confirmations.
    waiting_confs: BTreeMap<u64, Vec<TxWatcher>>,

    /// Earliest deadline across both watcher maps. Recomputed after reaping.
    next_timeout: Option<Instant>,

    /// Weak ownership avoids a cycle through the root provider and its heartbeat handle.
    client: WeakClient,

    /// Whether the heartbeat is currently paused.
    paused: Arc<Paused>,

    _network: std::marker::PhantomData<N>,
}

impl<N: Network, S: Stream<Item = N::BlockResponse> + Unpin + 'static> Heartbeat<N, S> {
    /// Create a new heartbeat task.
    pub(crate) fn new(stream: S, is_paused: Arc<Paused>, client: WeakClient) -> Self {
        Self {
            stream: stream.fuse(),
            past_blocks: Default::default(),
            unconfirmed: Default::default(),
            waiting_confs: Default::default(),
            client,
            next_timeout: None,
            paused: is_paused,
            _network: Default::default(),
        }
    }

    /// Check if any transactions have enough confirmations to notify.
    fn check_confirmations(&mut self, current_height: u64) {
        let to_keep = if current_height == u64::MAX {
            BTreeMap::new()
        } else {
            self.waiting_confs.split_off(&(current_height + 1))
        };
        let to_notify = std::mem::replace(&mut self.waiting_confs, to_keep);
        for watcher in to_notify.into_values().flatten() {
            watcher.notify(Ok(()));
        }
    }

    /// Get the next time to reap a transaction. If no reaps, this is a very
    /// long time from now (i.e. will not be woken).
    fn next_reap(&self) -> Instant {
        self.next_timeout.unwrap_or_else(|| Instant::now() + Duration::from_secs(60_000))
    }

    /// Reap timeouts even after inclusion, and stop watching cancelled futures.
    fn reap_timeouts(&mut self) {
        let now = Instant::now();
        self.next_timeout = None;
        for watchers in self.unconfirmed.values_mut().chain(self.waiting_confs.values_mut()) {
            for watcher in watchers.extract_if(.., |watcher| {
                watcher.tx.is_closed() || watcher.deadline.is_some_and(|deadline| deadline <= now)
            }) {
                watcher.notify(Err(WatchTxError::Timeout));
            }
            for deadline in watchers.iter().filter_map(|watcher| watcher.deadline) {
                self.next_timeout =
                    Some(self.next_timeout.map_or(deadline, |next| next.min(deadline)));
            }
        }
        self.unconfirmed.retain(|_, watchers| !watchers.is_empty());
        self.waiting_confs.retain(|_, watchers| !watchers.is_empty());
    }

    /// Reap transactions overridden by a chain gap (true reorg or resync after a pause).
    /// Accepts new chain height as an argument, and drops any subscriptions
    /// that were received in blocks affected by the reorg (e.g. >= new_height).
    fn move_reorg_to_unconfirmed(&mut self, new_height: u64) {
        for waiters in self.waiting_confs.values_mut() {
            *waiters = std::mem::take(waiters).into_iter().filter_map(|watcher| {
                if let Some(received_at_block) = watcher.received_at_block {
                    // All blocks after and _including_ the new height are reaped.
                    if received_at_block >= new_height {
                        let hash = watcher.config.tx_hash;
                        debug!(tx=%hash, %received_at_block, %new_height, "return to unconfirmed after chain gap");
                        let mut watcher = watcher;
                        watcher.received_at_block = None;
                        self.unconfirmed.entry(hash).or_default().push(watcher);
                        return None;
                    }
                }
                Some(watcher)
            }).collect();
        }
        self.waiting_confs.retain(|_, watchers| !watchers.is_empty());
    }

    /// Check if we have any pending transactions.
    fn has_pending_transactions(&self) -> bool {
        !self.unconfirmed.is_empty() || !self.waiting_confs.is_empty()
    }

    /// Update the pause state based on whether we have pending transactions.
    fn update_pause_state(&mut self) {
        let should_pause = !self.has_pending_transactions();
        if self.paused.is_paused() != should_pause {
            debug!(paused = should_pause, "updating heartbeat pause state");
            self.paused.set_paused(should_pause);
        }
    }

    /// Handle a watch instruction by adding it to the watch list, and
    /// preserving its individual timeout.
    fn handle_watch_ix(&mut self, to_watch: TxWatcher) {
        if let Some(deadline) = to_watch.deadline {
            self.next_timeout = Some(self.next_timeout.map_or(deadline, |next| next.min(deadline)));
        }
        // Start watching for the transaction.
        debug!(tx=%to_watch.config.tx_hash, "watching");
        trace!(?to_watch.config, ?to_watch.received_at_block);
        if let Some(received_at_block) = to_watch.received_at_block {
            // Transaction is already confirmed, we just need to wait for the required
            // confirmations.
            let confirmations = to_watch.config.required_confirmations;
            let confirmed_at = received_at_block.saturating_add(confirmations.saturating_sub(1));
            let current_height =
                self.past_blocks.back().map(|(h, _)| *h).unwrap_or(received_at_block);

            if confirmed_at <= current_height {
                to_watch.notify(Ok(()));
            } else {
                self.waiting_confs.entry(confirmed_at).or_default().push(to_watch);
            }
            return;
        }

        // Transaction may be confirmed already, check the lookbehind history first.
        // If so, insert it into the waiting list.
        for (block_height, txs) in self.past_blocks.iter().rev() {
            if txs.contains(&to_watch.config.tx_hash) {
                let confirmations = to_watch.config.required_confirmations;
                let confirmed_at = block_height.saturating_add(confirmations.saturating_sub(1));
                let current_height = self.past_blocks.back().map(|(h, _)| *h).unwrap();

                if confirmed_at <= current_height {
                    to_watch.notify(Ok(()));
                } else {
                    debug!(tx=%to_watch.config.tx_hash, %block_height, confirmations, "adding to waiting list");
                    // Ensure reorg handling can move this watcher back if needed.
                    let mut to_watch = to_watch;
                    if to_watch.received_at_block.is_none() {
                        to_watch.received_at_block = Some(*block_height);
                    }
                    self.waiting_confs.entry(confirmed_at).or_default().push(to_watch);
                }
                return;
            }
        }

        self.unconfirmed.entry(to_watch.config.tx_hash).or_default().push(to_watch);
    }

    fn add_to_waiting_list(&mut self, watcher: TxWatcher, block_height: u64) {
        let confirmations = watcher.config.required_confirmations;
        debug!(tx=%watcher.config.tx_hash, %block_height, confirmations, "adding to waiting list");
        self.waiting_confs
            .entry(block_height.saturating_add(confirmations.saturating_sub(1)))
            .or_default()
            .push(watcher);
    }

    /// Recheck each hash once per round, regardless of how many callers watch it.
    /// Receipt RPCs run concurrently with block processing and timeout handling.
    fn receipt_checks(&self) -> ReceiptChecks {
        let mut hashes = B256HashMap::<bool>::default();
        for watcher in self.unconfirmed.values().chain(self.waiting_confs.values()).flatten() {
            *hashes.entry(watcher.config.tx_hash).or_default() |=
                watcher.config.required_confirmations > 1;
        }
        let client = self.client.clone();
        let checks = futures::stream::iter(hashes).map(move |(hash, needs_height)| {
            let client = client.clone();
            Box::pin(async_stream::stream! {
                let Some(client) = client.upgrade() else { return };
                // Bound each RPC so a stalled endpoint cannot occupy a recovery slot forever.
                let receipt = match timeout(Duration::from_secs(30), client.request::<_, Option<N::ReceiptResponse>>(
                    "eth_getTransactionReceipt", (hash,),
                )).await {
                    Ok(Ok(Some(receipt))) => receipt,
                    Ok(Err(err)) => {
                        debug!(tx=%hash, %err, "failed to recheck receipt; retrying next round");
                        return;
                    }
                    _ => return,
                };
                let block = receipt.block_number();
                // Notify single-confirmation callers before querying the head, even if a
                // different caller watches the same hash with multiple confirmations.
                yield ReceiptCheck { hash, block, height: None };
                if needs_height && block.is_some() {
                    if let Ok(Ok(height)) = timeout(Duration::from_secs(30), client.request::<_, U64>("eth_blockNumber", ())).await {
                        yield ReceiptCheck { hash, block, height: Some(height.to()) };
                    }
                }
            })
        }).flatten_unordered(16);
        (Box::pin(checks) as BoxStream<'static, _>).fuse()
    }

    fn handle_receipt(&mut self, ReceiptCheck { hash, block, height }: ReceiptCheck) {
        let confirmed = |watcher: &mut TxWatcher| {
            let confirmations = watcher.config.required_confirmations;
            watcher.config.tx_hash == hash
                && (confirmations <= 1
                    || block.zip(height).is_some_and(|(block, height)| {
                        height >= block.saturating_add(confirmations.saturating_sub(1))
                    }))
        };
        if let Some(watchers) = self.unconfirmed.get_mut(&hash) {
            for watcher in watchers.extract_if(.., confirmed) {
                watcher.notify(Ok(()));
            }
            if watchers.is_empty() {
                self.unconfirmed.remove(&hash);
            }
        }
        if let Some(height) = height {
            for watchers in self.waiting_confs.range_mut(..=height).map(|(_, watchers)| watchers) {
                for watcher in watchers.extract_if(.., confirmed) {
                    watcher.notify(Ok(()));
                }
            }
            self.waiting_confs.retain(|_, watchers| !watchers.is_empty());
        }
    }

    /// Handle a new block by checking if any of the transactions we're
    /// watching are in it, and if so, notifying the watcher. Also updates
    /// the latest block.
    fn handle_new_block(&mut self, block: N::BlockResponse) {
        let block_height = block.header().as_ref().number();
        debug!(%block_height, "handling block");

        // Add the block the lookbehind.
        // The value is chosen arbitrarily to not have a huge memory footprint but still
        // catch most cases where user subscribes for an already mined transaction.
        // Note that we expect provider to check whether transaction is already mined
        // before subscribing, so here we only need to consider time before sending a notification
        // and processing it.
        const MAX_BLOCKS_TO_RETAIN: usize = 10;
        if self.past_blocks.len() >= MAX_BLOCKS_TO_RETAIN {
            self.past_blocks.pop_front();
        }
        if let Some((last_height, _)) = self.past_blocks.back().as_ref() {
            // Check that the chain is continuous.
            if *last_height + 1 != block_height {
                // Move all the transactions that were reset by the reorg to the unconfirmed list.
                // This can also happen if we unpaused the heartbeat after some time.
                debug!(block_height, last_height, "reorg/unpause detected");
                self.move_reorg_to_unconfirmed(block_height);
                // Remove past blocks that are now invalid.
                self.past_blocks.retain(|(h, _)| *h < block_height);
            }
        }
        self.past_blocks.push_back((block_height, block.transactions().hashes().collect()));

        // Check if we are watching for any of the transactions in this block.
        let to_check: Vec<_> = block
            .transactions()
            .hashes()
            .filter_map(|tx_hash| self.unconfirmed.remove(&tx_hash))
            .flatten()
            .collect();
        for mut watcher in to_check {
            // If `confirmations` is not more than 1 we can notify the watcher immediately.
            let confirmations = watcher.config.required_confirmations;
            if confirmations <= 1 {
                watcher.notify(Ok(()));
                continue;
            }
            // Otherwise add it to the waiting list.

            // Set the block at which the transaction was received.
            if let Some(set_block) = watcher.received_at_block {
                warn!(tx=%watcher.config.tx_hash, set_block=%set_block, new_block=%block_height, "received_at_block already set");
                // We don't override the set value.
            } else {
                watcher.received_at_block = Some(block_height);
            }
            self.add_to_waiting_list(watcher, block_height);
        }

        self.check_confirmations(block_height);
    }
}

#[cfg(target_family = "wasm")]
impl<N: Network, S: Stream<Item = N::BlockResponse> + Unpin + 'static> Heartbeat<N, S> {
    /// Spawn the heartbeat task, returning a [`HeartbeatHandle`].
    pub(crate) fn spawn(self) -> HeartbeatHandle {
        let (task, handle) = self.consume();
        task.spawn_task();
        handle
    }
}

#[cfg(not(target_family = "wasm"))]
impl<N: Network, S: Stream<Item = N::BlockResponse> + Unpin + Send + 'static> Heartbeat<N, S> {
    /// Spawn the heartbeat task, returning a [`HeartbeatHandle`].
    pub(crate) fn spawn(self) -> HeartbeatHandle {
        let (task, handle) = self.consume();
        task.spawn_task();
        handle
    }
}

impl<N: Network, S: Stream<Item = N::BlockResponse> + Unpin + 'static> Heartbeat<N, S> {
    fn consume(self) -> (impl Future<Output = ()>, HeartbeatHandle) {
        let (ix_tx, ixns) = mpsc::channel(64);
        (self.into_future(ixns), HeartbeatHandle { tx: ix_tx })
    }

    async fn into_future(mut self, mut ixns: mpsc::Receiver<TxWatcher>) {
        // There is at most one bounded recovery round in flight. Dropping it when idle
        // also cancels outstanding requests. Keep no strong provider/client reference here.
        let mut checks: Option<ReceiptChecks> = None;
        let mut next_check = Instant::now();
        'shutdown: loop {
            if self.next_timeout.is_some_and(|deadline| deadline <= Instant::now()) {
                self.reap_timeouts();
            }
            self.update_pause_state();
            if !self.has_pending_transactions() {
                checks = None;
            }
            let wake_at = if self.has_pending_transactions() {
                self.next_reap().min(next_check)
            } else {
                self.next_reap()
            };
            let sleep = std::pin::pin!(sleep_until(wake_at.into()));
            select! {
                ix_opt = ixns.recv() => match ix_opt {
                    Some(to_watch) => self.handle_watch_ix(to_watch),
                    None => break 'shutdown,
                },
                Some(block) = self.stream.next() => {
                    // Discard in-flight receipt observations on a detected reorg.
                    if self.past_blocks.back().is_some_and(|(height, _)| block.header().as_ref().number() <= *height) {
                        checks = Some(self.receipt_checks());
                    }
                    self.handle_new_block(block);
                },
                Some(receipt) = async {
                    match checks.as_mut() {
                        Some(checks) => checks.next().await,
                        None => futures::future::pending().await,
                    }
                } => self.handle_receipt(receipt),
                _ = sleep => {
                    if Instant::now() >= next_check {
                        self.reap_timeouts();
                        if checks.as_ref().is_none_or(FusedStream::is_terminated) {
                            checks = Some(self.receipt_checks());
                        }
                        let poll_interval = self.client.upgrade().map(|client| client.poll_interval()).unwrap_or(Duration::from_secs(1));
                        next_check = Instant::now() + poll_interval.max(Duration::from_millis(1));
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests;
