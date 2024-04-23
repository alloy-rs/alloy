//! Block heartbeat and pending transaction watcher.

use crate::{Provider, RootProvider};
use alloy_json_rpc::RpcError;
use alloy_network::Network;
use alloy_primitives::B256;
use alloy_rpc_types::Block;
use alloy_transport::{utils::Spawnable, Transport, TransportErrorKind, TransportResult};
use futures::{stream::StreamExt, FutureExt, Stream};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    future::Future,
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::{mpsc, oneshot, watch},
};

/// A builder for configuring a pending transaction watcher.
///
/// # Examples
///
/// Send and wait for a transaction to be confirmed 2 times, with a timeout of 60 seconds:
///
/// ```no_run
/// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider, tx: alloy_rpc_types::transaction::TransactionRequest) -> Result<(), Box<dyn std::error::Error>> {
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
/// # async fn example<N: alloy_network::Network>(provider: impl alloy_provider::Provider, tx: alloy_rpc_types::transaction::TransactionRequest) -> Result<(), Box<dyn std::error::Error>> {
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
pub struct PendingTransactionBuilder<'a, T, N> {
    config: PendingTransactionConfig,
    provider: &'a RootProvider<T, N>,
}

impl<'a, T: Transport + Clone, N: Network> PendingTransactionBuilder<'a, T, N> {
    /// Creates a new pending transaction builder.
    pub const fn new(provider: &'a RootProvider<T, N>, tx_hash: B256) -> Self {
        Self::from_config(provider, PendingTransactionConfig::new(tx_hash))
    }

    /// Creates a new pending transaction builder from the given configuration.
    pub const fn from_config(
        provider: &'a RootProvider<T, N>,
        config: PendingTransactionConfig,
    ) -> Self {
        Self { config, provider }
    }

    /// Returns the inner configuration.
    pub const fn inner(&self) -> &PendingTransactionConfig {
        &self.config
    }

    /// Consumes this builder, returning the inner configuration.
    pub const fn into_inner(self) -> PendingTransactionConfig {
        self.config
    }

    /// Returns the provider.
    pub const fn provider(&self) -> &'a RootProvider<T, N> {
        self.provider
    }

    /// Consumes this builder, returning the provider and the configuration.
    pub const fn split(self) -> (&'a RootProvider<T, N>, PendingTransactionConfig) {
        (self.provider, self.config)
    }

    /// Returns the transaction hash.
    pub const fn tx_hash(&self) -> &B256 {
        self.config.tx_hash()
    }

    /// Sets the transaction hash.
    pub fn set_tx_hash(&mut self, tx_hash: B256) {
        self.config.set_tx_hash(tx_hash);
    }

    /// Sets the transaction hash.
    pub const fn with_tx_hash(mut self, tx_hash: B256) -> Self {
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
    pub fn set_required_confirmations(&mut self, confirmations: u64) {
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
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
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
    pub async fn register(self) -> TransportResult<PendingTransaction> {
        self.provider.watch_pending_transaction(self.config).await
    }

    /// Waits for the transaction to confirm with the given number of confirmations.
    ///
    /// See:
    /// - [`register`](Self::register): for registering the transaction without waiting for it to be
    ///   confirmed.
    /// - [`get_receipt`](Self::get_receipt) for fetching the receipt after the transaction has been
    ///   confirmed.
    pub async fn watch(self) -> TransportResult<B256> {
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
    pub async fn get_receipt(self) -> TransportResult<N::ReceiptResponse> {
        let hash = self.config.tx_hash;
        let mut pending_tx = self.provider.watch_pending_transaction(self.config).await?;

        // FIXME: this is a hotfix to prevent a race condition where the heartbeat would miss the
        // block the tx was mined in
        let mut interval =
            tokio::time::interval(Duration::from_millis(self.provider.client().poll_interval()));

        loop {
            let mut confirmed = false;

            select! {
                 _ = interval.tick() => {},
                 res = &mut pending_tx => {
                        let _ = res?;
                        confirmed = true;
                    }
            }

            // try to fetch the receipt
            let receipt = self.provider.get_transaction_receipt(hash).await?;
            if let Some(receipt) = receipt {
                return Ok(receipt);
            }

            if confirmed {
                return Err(RpcError::NullResp);
            }
        }
    }
}

/// Configuration for watching a pending transaction.
///
/// This type can be used to create a [`PendingTransactionBuilder`], but in general it is only used
/// internally.
#[must_use = "this type does nothing unless you call `with_provider`"]
#[derive(Clone, Debug)]
#[allow(missing_copy_implementations)]
pub struct PendingTransactionConfig {
    /// The transaction hash to watch for.
    tx_hash: B256,

    /// Require a number of confirmations.
    required_confirmations: u64,

    /// Optional timeout for the transaction.
    timeout: Option<Duration>,
}

impl PendingTransactionConfig {
    /// Create a new watch for a transaction.
    pub const fn new(tx_hash: B256) -> Self {
        Self { tx_hash, required_confirmations: 1, timeout: None }
    }

    /// Returns the transaction hash.
    pub const fn tx_hash(&self) -> &B256 {
        &self.tx_hash
    }

    /// Sets the transaction hash.
    pub fn set_tx_hash(&mut self, tx_hash: B256) {
        self.tx_hash = tx_hash;
    }

    /// Sets the transaction hash.
    pub const fn with_tx_hash(mut self, tx_hash: B256) -> Self {
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
    pub fn set_required_confirmations(&mut self, confirmations: u64) {
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
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    /// Sets the timeout.
    pub const fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Wraps this configuration with a provider to expose watching methods.
    pub const fn with_provider<T: Transport + Clone, N: Network>(
        self,
        provider: &RootProvider<T, N>,
    ) -> PendingTransactionBuilder<'_, T, N> {
        PendingTransactionBuilder::from_config(provider, self)
    }
}

struct TxWatcher {
    config: PendingTransactionConfig,
    tx: oneshot::Sender<()>,
}

impl TxWatcher {
    /// Notify the waiter.
    fn notify(self) {
        debug!(tx=%self.config.tx_hash, "notifying");
        let _ = self.tx.send(());
    }
}

/// Represents a transaction that is yet to be confirmed a specified number of times.
///
/// This struct is a future created by [`PendingTransactionBuilder`] that resolves to the
/// transaction hash once the underlying transaction has been confirmed the specified number of
/// times in the network.
pub struct PendingTransaction {
    /// The transaction hash.
    pub(crate) tx_hash: B256,
    /// The receiver for the notification.
    // TODO: send a receipt?
    pub(crate) rx: oneshot::Receiver<()>,
}

impl fmt::Debug for PendingTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingTransaction").field("tx_hash", &self.tx_hash).finish()
    }
}

impl PendingTransaction {
    /// Returns this transaction's hash.
    pub const fn tx_hash(&self) -> &B256 {
        &self.tx_hash
    }
}

impl Future for PendingTransaction {
    type Output = TransportResult<B256>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.rx
            .poll_unpin(cx)
            .map(|res| res.map(|()| self.tx_hash).map_err(|_| TransportErrorKind::backend_gone()))
    }
}

/// A handle to the heartbeat task.
#[derive(Clone, Debug)]
pub(crate) struct HeartbeatHandle {
    tx: mpsc::Sender<TxWatcher>,
    #[allow(dead_code)]
    latest: watch::Receiver<Option<Block>>,
}

impl HeartbeatHandle {
    /// Watch for a transaction to be confirmed with the given config.
    pub(crate) async fn watch_tx(
        &self,
        config: PendingTransactionConfig,
    ) -> Result<PendingTransaction, PendingTransactionConfig> {
        let (tx, rx) = oneshot::channel();
        let tx_hash = config.tx_hash;
        match self.tx.send(TxWatcher { config, tx }).await {
            Ok(()) => Ok(PendingTransaction { tx_hash, rx }),
            Err(e) => Err(e.0.config),
        }
    }

    /// Returns a watcher that always sees the latest block.
    #[allow(dead_code)]
    pub(crate) const fn latest(&self) -> &watch::Receiver<Option<Block>> {
        &self.latest
    }
}

// TODO: Parameterize with `Network`
/// A heartbeat task that receives blocks and watches for transactions.
pub(crate) struct Heartbeat<S> {
    /// The stream of incoming blocks to watch.
    stream: futures::stream::Fuse<S>,

    /// Transactions to watch for.
    unconfirmed: HashMap<B256, TxWatcher>,

    /// Ordered map of transactions waiting for confirmations.
    waiting_confs: BTreeMap<u64, Vec<TxWatcher>>,

    /// Ordered map of transactions to reap at a certain time.
    reap_at: BTreeMap<Instant, B256>,
}

impl<S: Stream<Item = Block> + Unpin + 'static> Heartbeat<S> {
    /// Create a new heartbeat task.
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            unconfirmed: Default::default(),
            waiting_confs: Default::default(),
            reap_at: Default::default(),
        }
    }
}

impl<S> Heartbeat<S> {
    /// Check if any transactions have enough confirmations to notify.
    fn check_confirmations(&mut self, current_height: u64) {
        let to_keep = self.waiting_confs.split_off(&(current_height + 1));
        let to_notify = std::mem::replace(&mut self.waiting_confs, to_keep);
        for watcher in to_notify.into_values().flatten() {
            watcher.notify();
        }
    }

    /// Get the next time to reap a transaction. If no reaps, this is a very
    /// long time from now (i.e. will not be woken).
    fn next_reap(&self) -> Instant {
        self.reap_at
            .first_key_value()
            .map(|(k, _)| *k)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(60_000))
    }

    /// Reap any timeout
    fn reap_timeouts(&mut self) {
        let now = Instant::now();
        let to_keep = self.reap_at.split_off(&now);
        let to_reap = std::mem::replace(&mut self.reap_at, to_keep);

        for tx_hash in to_reap.values() {
            if self.unconfirmed.remove(tx_hash).is_some() {
                debug!(tx=%tx_hash, "reaped");
            }
        }
    }

    /// Handle a watch instruction by adding it to the watch list, and
    /// potentially adding it to our `reap_at` list.
    fn handle_watch_ix(&mut self, to_watch: TxWatcher) {
        // Start watching for the transaction.
        debug!(tx=%to_watch.config.tx_hash, "watching");
        trace!(?to_watch.config);
        if let Some(timeout) = to_watch.config.timeout {
            self.reap_at.insert(Instant::now() + timeout, to_watch.config.tx_hash);
        }
        self.unconfirmed.insert(to_watch.config.tx_hash, to_watch);
    }

    /// Handle a new block by checking if any of the transactions we're
    /// watching are in it, and if so, notifying the watcher. Also updates
    /// the latest block.
    fn handle_new_block(&mut self, block: Block, latest: &watch::Sender<Option<Block>>) {
        // Blocks without numbers are ignored, as they're not part of the chain.
        let Some(block_height) = &block.header.number else { return };

        // Check if we are watching for any of the transactions in this block.
        let to_check =
            block.transactions.hashes().filter_map(|tx_hash| self.unconfirmed.remove(tx_hash));
        for watcher in to_check {
            // If `confirmations` is not more than 1 we can notify the watcher immediately.
            let confirmations = watcher.config.required_confirmations;
            if confirmations <= 1 {
                watcher.notify();
                continue;
            }
            // Otherwise add it to the waiting list.
            debug!(tx=%watcher.config.tx_hash, %block_height, confirmations, "adding to waiting list");
            self.waiting_confs.entry(*block_height + confirmations - 1).or_default().push(watcher);
        }

        self.check_confirmations(*block_height);

        // Update the latest block. We use `send_replace` here to ensure the
        // latest block is always up to date, even if no receivers exist.
        // C.f. https://docs.rs/tokio/latest/tokio/sync/watch/struct.Sender.html#method.send
        debug!(%block_height, "updating latest block");
        let _ = latest.send_replace(Some(block));
    }
}

#[cfg(target_arch = "wasm32")]
impl<S: Stream<Item = Block> + Unpin + 'static> Heartbeat<S> {
    /// Spawn the heartbeat task, returning a [`HeartbeatHandle`].
    pub(crate) fn spawn(self) -> HeartbeatHandle {
        let (latest, latest_rx) = watch::channel(None::<Block>);
        let (ix_tx, ixns) = mpsc::channel(16);

        self.into_future(latest, ixns).spawn_task();

        HeartbeatHandle { tx: ix_tx, latest: latest_rx }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<S: Stream<Item = Block> + Unpin + Send + 'static> Heartbeat<S> {
    /// Spawn the heartbeat task, returning a [`HeartbeatHandle`].
    pub(crate) fn spawn(self) -> HeartbeatHandle {
        let (latest, latest_rx) = watch::channel(None::<Block>);
        let (ix_tx, ixns) = mpsc::channel(16);

        self.into_future(latest, ixns).spawn_task();

        HeartbeatHandle { tx: ix_tx, latest: latest_rx }
    }
}

impl<S: Stream<Item = Block> + Unpin + 'static> Heartbeat<S> {
    async fn into_future(
        mut self,
        latest: watch::Sender<Option<Block>>,
        mut ixns: mpsc::Receiver<TxWatcher>,
    ) {
        'shutdown: loop {
            {
                let next_reap = self.next_reap();
                let sleep = std::pin::pin!(tokio::time::sleep_until(next_reap.into()));

                // We bias the select so that we always handle new messages
                // before checking blocks, and reap timeouts are last.
                select! {
                    biased;

                    // Watch for new transactions.
                    ix_opt = ixns.recv() => match ix_opt {
                        Some(to_watch) => self.handle_watch_ix(to_watch),
                        None => break 'shutdown, // ix channel is closed
                    },

                    // Wake up to handle new blocks.
                    Some(block) = self.stream.next() => {
                        self.handle_new_block(block, &latest);
                    },

                    // This arm ensures we always wake up to reap timeouts,
                    // even if there are no other events.
                    _ = sleep => {},
                }
            }

            // Always reap timeouts
            self.reap_timeouts();
        }
    }
}
