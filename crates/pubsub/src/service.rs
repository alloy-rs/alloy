use crate::{
    handle::ConnectionHandle,
    ix::PubSubInstruction,
    managers::{InFlight, RequestManager, SubscriptionManager},
    PubSubConnect, PubSubFrontend, RawSubscription,
};
use alloy_json_rpc::{Id, PubSubItem, Request, Response, ResponsePayload, RpcError, SubId};
use alloy_primitives::B256;
use alloy_transport::{
    utils::{to_json_raw_value, Spawnable},
    TransportError, TransportErrorKind, TransportResult,
};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

const MAX_RECONNECT_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// The service contains the backend handle, a subscription manager, and the
/// configuration details required to reconnect.
#[derive(Debug)]
pub(crate) struct PubSubService<T> {
    /// The backend handle.
    pub(crate) handle: ConnectionHandle,

    /// The configuration details required to reconnect.
    pub(crate) connector: T,

    /// The inbound requests.
    pub(crate) reqs: mpsc::UnboundedReceiver<PubSubInstruction>,

    /// The subscription manager.
    pub(crate) subs: SubscriptionManager,

    /// The request manager.
    pub(crate) in_flights: RequestManager,
}

impl<T: PubSubConnect> PubSubService<T> {
    /// Create a new service from a connector.
    pub(crate) async fn connect(connector: T) -> TransportResult<PubSubFrontend> {
        let handle = connector.connect().await?;

        let (tx, reqs) = mpsc::unbounded_channel();
        let this = Self {
            handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: Default::default(),
        };
        this.spawn();
        Ok(PubSubFrontend::new(tx))
    }

    /// Reconnect by dropping the backend and creating a new one.
    async fn get_new_backend(&mut self) -> TransportResult<ConnectionHandle> {
        let mut handle = self.connector.try_reconnect().await?;
        std::mem::swap(&mut self.handle, &mut handle);
        Ok(handle)
    }

    /// Reconnect the backend, re-issue pending requests, and re-start active
    /// subscriptions.
    async fn reconnect(&mut self) -> TransportResult<()> {
        debug!("Reconnecting pubsub service backend");

        let mut old_handle = self.get_new_backend().await?;

        debug!("Draining old backend to_handle");

        // Drain the old backend
        while let Ok(item) = old_handle.from_socket.try_recv() {
            self.handle_item(item)?;
        }

        old_handle.shutdown();

        // Re-issue pending requests.
        debug!(count = self.in_flights.len(), "Reissuing pending requests");
        for (_, in_flight) in self.in_flights.iter() {
            let msg = in_flight.request.serialized().to_owned();
            self.dispatch_request(msg)?;
        }

        // Re-subscribe to all active subscriptions
        debug!(count = self.subs.len(), "Re-starting active subscriptions");

        // Drop all server IDs. We'll re-insert them as we get responses.
        self.subs.drop_server_ids();

        // Dispatch all subscription requests.
        for (_, sub) in self.subs.iter() {
            let req = sub.request().to_owned();
            let (in_flight, _) = InFlight::new(req.clone(), sub.tx.receiver_count());
            self.in_flights.insert(in_flight);

            let msg = req.into_serialized();
            self.dispatch_request(msg)?;
        }

        Ok(())
    }

    /// Dispatch a request to the socket.
    fn dispatch_request(&self, brv: Box<RawValue>) -> TransportResult<()> {
        self.handle.to_socket.send(brv).map(drop).map_err(|_| TransportErrorKind::backend_gone())
    }

    /// Service a request.
    fn service_request(&mut self, in_flight: InFlight) -> TransportResult<()> {
        let brv = in_flight.request();

        self.dispatch_request(brv.serialized().to_owned())?;
        self.in_flights.insert(in_flight);

        Ok(())
    }

    /// Service a GetSub instruction.
    ///
    /// If the subscription exists, the waiter is sent `Some` broadcast receiver. If
    /// the subscription does not exist, the waiter is sent `None`.
    fn service_get_sub(&self, local_id: B256, tx: oneshot::Sender<Option<RawSubscription>>) {
        let _ = tx.send(self.subs.get_subscription(local_id));
    }

    /// Service an unsubscribe instruction.
    fn service_unsubscribe(&mut self, local_id: B256) -> TransportResult<()> {
        if let Some(server_id) = self.subs.server_id_for(&local_id) {
            // TODO: ideally we can send this with an unused id
            let req = Request::new("eth_unsubscribe", Id::Number(1), [server_id]);
            let brv = req.serialize().expect("no ser error").take_request();

            self.dispatch_request(brv)?;
        }
        self.subs.remove_sub(local_id);
        Ok(())
    }

    /// Service an instruction
    fn service_ix(&mut self, ix: PubSubInstruction) -> TransportResult<()> {
        trace!(?ix, "servicing instruction");
        match ix {
            PubSubInstruction::Request(in_flight) => self.service_request(in_flight),
            PubSubInstruction::GetSub(alias, tx) => {
                self.service_get_sub(alias, tx);
                Ok(())
            }
            PubSubInstruction::Unsubscribe(alias) => self.service_unsubscribe(alias),
        }
    }

    /// Handle an item from the backend.
    fn handle_item(&mut self, item: PubSubItem) -> TransportResult<()> {
        match item {
            PubSubItem::Response(resp) => match self.in_flights.handle_response(resp) {
                Some((server_id, in_flight)) => self.handle_sub_response(in_flight, server_id),
                None => Ok(()),
            },
            PubSubItem::Notification(notification) => {
                self.subs.notify(notification);
                Ok(())
            }
        }
    }

    /// Rewrite the subscription id and insert into the subscriptions manager
    fn handle_sub_response(
        &mut self,
        in_flight: InFlight,
        server_id: SubId,
    ) -> TransportResult<()> {
        let request = in_flight.request;
        let id = request.id().clone();

        let sub = self.subs.upsert(request, server_id, in_flight.channel_size);

        // Serialized B256 is always a valid serialized U256 too.
        let ser_alias = to_json_raw_value(sub.local_id())?;

        // We send back a success response with the new subscription ID.
        // We don't care if the channel is dead.
        let _ =
            in_flight.tx.send(Ok(Response { id, payload: ResponsePayload::Success(ser_alias) }));

        Ok(())
    }

    /// Attempt to reconnect with retries.
    ///
    /// Aborts immediately when a reconnect attempt returns a
    /// [`TransportErrorKind::NonRetryable`] error so deterministic backend
    /// failures (auth/protocol violations, malformed handshake, etc.) do not
    /// burn the full retry budget.
    async fn reconnect_with_retries(&mut self) -> TransportResult<()> {
        let mut retry_count = 0;
        let max_retries = self.handle.max_retries;
        let interval = self.handle.retry_interval;
        loop {
            match self.reconnect().await {
                Ok(()) => break Ok(()),
                Err(e) => {
                    if matches!(&e, RpcError::Transport(k) if k.is_non_retryable()) {
                        error!("Reconnect aborted (non-retryable), shutting down: {e}");
                        break Err(e);
                    }
                    retry_count += 1;
                    if retry_count >= max_retries {
                        error!("Reconnect failed after {max_retries} attempts, shutting down: {e}");
                        break Err(e);
                    }
                    let retry_interval = reconnect_retry_interval(interval, retry_count);
                    warn!(
                        "Reconnection attempt {retry_count}/{max_retries} failed: {e}. \
                         Retrying in {retry_interval:?}...",
                    );
                    sleep(retry_interval).await;
                }
            }
        }
    }

    /// Spawn the service.
    pub(crate) fn spawn(mut self) {
        let fut = async move {
            let result: TransportResult<()> = loop {
                // We bias the loop so that we always handle new messages before
                // reconnecting, and always reconnect before dispatching new
                // requests.
                tokio::select! {
                    biased;

                    item_opt = self.handle.from_socket.recv() => {
                        if let Some(item) = item_opt {
                            if let Err(e) = self.handle_item(item) {
                                break Err(e)
                            }
                        } else {
                            // The backend dropped its `to_frontend` sender.
                            // It may have also signaled a typed error via the
                            // `error` oneshot; drain it before reconnecting
                            // so a non-retryable error short-circuits the loop.
                            if let Ok(err) = self.handle.error.try_recv() {
                                if matches!(&err, RpcError::Transport(k) if k.is_non_retryable()) {
                                    error!(%err, "Pubsub service backend reported a non-retryable error, shutting down.");
                                    break Err(err)
                                }
                                error!(%err, "Pubsub service backend error.");
                            }
                            if let Err(e) = self.reconnect_with_retries().await {
                                break Err(e)
                            }
                        }
                    }

                    res = &mut self.handle.error => {
                        // The backend signaled a terminal error. The carried
                        // `TransportError` indicates whether it is recoverable.
                        // If the sender was dropped without a value, fall back
                        // to a generic backend-gone error.
                        let err = res.unwrap_or_else(|_| TransportErrorKind::backend_gone());
                        if matches!(&err, RpcError::Transport(k) if k.is_non_retryable()) {
                            error!(%err, "Pubsub service backend reported a non-retryable error, shutting down.");
                            break Err(err)
                        }
                        error!(%err, "Pubsub service backend error.");
                        if let Err(e) = self.reconnect_with_retries().await {
                            break Err(e)
                        }
                    }

                    req_opt = self.reqs.recv() => {
                        if let Some(req) = req_opt {
                            if let Err(err) = self.service_ix(req) {
                                if err
                                    .as_transport_err()
                                    .is_some_and(TransportErrorKind::is_backend_gone)
                                {
                                    if let Err(e) = self.reconnect_with_retries().await {
                                        break Err(e)
                                    }
                                } else {
                                    break Err(err)
                                }
                            }
                        } else {
                            info!("Pubsub service request channel closed. Shutting down.");
                           break Ok(())
                        }
                    }
                }
            };

            if let Err(err) = result {
                error!(%err, "pubsub service reconnection error");
                self.cleanup_on_shutdown(err);
            }
        };
        fut.spawn_task();
    }

    /// Notify pending requests when service shuts down after reconnect failures.
    fn cleanup_on_shutdown(&mut self, _err: TransportError) {
        debug!(count = self.in_flights.len(), "Cleaning up pending requests");
        for (_, in_flight) in self.in_flights.drain() {
            let _ = in_flight.tx.send(Err(TransportErrorKind::backend_gone()));
        }

        // Subscriptions will be notified via broadcast channel drop
        debug!(count = self.subs.len(), "Dropping subscriptions");
    }
}

/// Returns the capped exponential backoff interval for a reconnect retry.
///
/// The configured retry interval is used as the base delay. Retry counts are 1-based, so the first
/// failed attempt waits for the base interval, the second waits for twice the base interval, and so
/// on. The delay is capped at [`MAX_RECONNECT_RETRY_INTERVAL`], unless the configured base interval
/// is already higher, in which case the configured base interval is preserved.
fn reconnect_retry_interval(base_interval: Duration, retry_count: u32) -> Duration {
    let backoff_multiplier = 1u32.checked_shl(retry_count.saturating_sub(1)).unwrap_or(u32::MAX);
    let max_interval = base_interval.max(MAX_RECONNECT_RETRY_INTERVAL);

    base_interval.saturating_mul(backoff_multiplier).min(max_interval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionInterface;
    use alloy_json_rpc::Request;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    };
    use tokio::time::timeout;

    #[derive(Clone, Debug, Default)]
    struct MockConnect(Arc<Mutex<Option<ConnectionHandle>>>);

    impl PubSubConnect for MockConnect {
        fn is_local(&self) -> bool {
            true
        }

        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            Err(TransportErrorKind::custom_str("connect is not used in this test"))
        }

        async fn try_reconnect(&self) -> TransportResult<ConnectionHandle> {
            self.0
                .lock()
                .expect("poisoned mutex")
                .take()
                .ok_or_else(|| TransportErrorKind::custom_str("missing mock connection handle"))
        }
    }

    /// Mock connector that counts every `try_reconnect` invocation and
    /// optionally returns a queued [`ConnectionHandle`].
    #[derive(Clone, Debug, Default)]
    struct CountingConnect {
        handle: Arc<Mutex<Option<ConnectionHandle>>>,
        calls: Arc<AtomicUsize>,
    }

    impl CountingConnect {
        fn with_handle(handle: ConnectionHandle) -> Self {
            Self {
                handle: Arc::new(Mutex::new(Some(handle))),
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl PubSubConnect for CountingConnect {
        fn is_local(&self) -> bool {
            true
        }

        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            Err(TransportErrorKind::custom_str("connect is not used in this test"))
        }

        async fn try_reconnect(&self) -> TransportResult<ConnectionHandle> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.handle
                .lock()
                .expect("poisoned mutex")
                .take()
                .ok_or_else(|| TransportErrorKind::custom_str("no more handles"))
        }
    }

    /// Returns a non-retryable error and counts `try_reconnect` calls.
    #[derive(Clone, Debug, Default)]
    struct NonRetryableConnect(Arc<AtomicUsize>);

    impl PubSubConnect for NonRetryableConnect {
        fn is_local(&self) -> bool {
            true
        }

        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            Err(TransportErrorKind::non_retryable_str("non-retryable test failure"))
        }

        async fn try_reconnect(&self) -> TransportResult<ConnectionHandle> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(TransportErrorKind::non_retryable_str("non-retryable test failure"))
        }
    }

    #[test]
    fn reconnect_retry_interval_uses_capped_exponential_backoff() {
        let base = Duration::from_secs(1);

        assert_eq!(reconnect_retry_interval(base, 1), Duration::from_secs(1));
        assert_eq!(reconnect_retry_interval(base, 2), Duration::from_secs(2));
        assert_eq!(reconnect_retry_interval(base, 3), Duration::from_secs(4));
        assert_eq!(reconnect_retry_interval(base, 6), Duration::from_secs(30));
    }

    #[test]
    fn reconnect_retry_interval_uses_configured_base_interval() {
        let base = Duration::from_millis(1);

        assert_eq!(reconnect_retry_interval(base, 1), Duration::from_millis(1));
        assert_eq!(reconnect_retry_interval(base, 2), Duration::from_millis(2));
    }

    #[test]
    fn reconnect_retry_interval_does_not_shorten_base_above_cap() {
        let base = Duration::from_secs(60);

        assert_eq!(reconnect_retry_interval(base, 1), Duration::from_secs(60));
        assert_eq!(reconnect_retry_interval(base, 2), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn reconnects_after_request_dispatch_hits_backend_gone() {
        let (dead_handle, dead_interface) = ConnectionHandle::new();
        let ConnectionInterface { from_frontend, to_frontend, error, shutdown } = dead_interface;
        drop(from_frontend);
        let _keep_dead_backend_alive = (to_frontend, error, shutdown);

        let (reconnected_handle, mut reconnected_interface) = ConnectionHandle::new();
        let connector = MockConnect(Arc::new(Mutex::new(Some(reconnected_handle))));
        let (tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle: dead_handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        service.spawn();

        let first = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (in_flight, rx) = InFlight::new(first, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        timeout(Duration::from_secs(1), rx)
            .await
            .expect("failed request should resolve promptly")
            .expect_err("raced request should be dropped when the backend is gone");

        let second = Request::new("eth_chainId", Id::Number(2), ()).serialize().unwrap();
        let expected = second.serialized().get().to_owned();
        let (in_flight, _rx) = InFlight::new(second, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        let dispatched =
            timeout(Duration::from_secs(1), reconnected_interface.recv_from_frontend())
                .await
                .expect("request should be dispatched after reconnect")
                .expect("new backend should receive the request");
        assert_eq!(dispatched.get(), expected);
    }

    #[tokio::test]
    async fn non_retryable_reconnect_error_short_circuits_retry_loop() {
        let (dead_handle, dead_interface) = ConnectionHandle::new();
        let ConnectionInterface { from_frontend, to_frontend, error, shutdown } = dead_interface;
        drop(from_frontend);
        let _keep_dead_backend_alive = (to_frontend, error, shutdown);

        let connector = NonRetryableConnect::default();
        let counter = connector.0.clone();
        let (tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle: dead_handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        service.spawn();

        let req = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (in_flight, rx) = InFlight::new(req, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        timeout(Duration::from_secs(1), rx)
            .await
            .expect("non-retryable reconnect should resolve promptly")
            .expect_err("request should fail when backend is gone and reconnect aborts");

        // Exactly one attempt, not `max_retries`.
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn non_retryable_close_skips_reconnect_loop() {
        // Backend is alive but emits a non-retryable error via the typed
        // `close_with_transport_error` channel. The service must NOT call
        // `try_reconnect` at all.
        let (live_handle, live_interface) = ConnectionHandle::new();

        // Provide a fresh handle that the connector *could* return, so that
        // accidentally triggering `try_reconnect` would succeed and complete
        // the reconnect path. We assert the call count to prove it didn't.
        let (spare_handle, _spare_interface) = ConnectionHandle::new();
        let connector = CountingConnect::with_handle(spare_handle);
        let calls = connector.calls.clone();

        let (_tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle: live_handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        service.spawn();

        // Backend signals a deterministic, non-retryable failure.
        live_interface.close_with_transport_error(TransportErrorKind::non_retryable_str(
            "deterministic protocol failure",
        ));

        // Give the service a chance to act on the error.
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "non-retryable backend error must not trigger reconnect attempts"
        );
    }

    #[tokio::test]
    async fn default_close_with_error_still_reconnects() {
        // Sanity check: the legacy `close_with_error()` path (which sends
        // `BackendGone`) continues to trigger the reconnect loop.
        let (live_handle, live_interface) = ConnectionHandle::new();

        let (reconnected_handle, mut reconnected_interface) = ConnectionHandle::new();
        let connector = CountingConnect::with_handle(reconnected_handle);
        let calls = connector.calls.clone();

        let (tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle: live_handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        service.spawn();

        // Trigger the legacy close path.
        live_interface.close_with_error();

        // After reconnect, a freshly dispatched request must reach the new
        // backend.
        let req = Request::new("eth_chainId", Id::Number(1), ()).serialize().unwrap();
        let expected = req.serialized().get().to_owned();
        let (in_flight, _rx) = InFlight::new(req, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        let dispatched =
            timeout(Duration::from_secs(1), reconnected_interface.recv_from_frontend())
                .await
                .expect("request should be dispatched after reconnect")
                .expect("new backend should receive the request");
        assert_eq!(dispatched.get(), expected);

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "default close_with_error should trigger exactly one reconnect"
        );
    }
}
