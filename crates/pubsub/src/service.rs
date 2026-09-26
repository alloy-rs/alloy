use crate::{
    handle::ConnectionHandle,
    ix::PubSubInstruction,
    managers::{InFlight, RequestManager, SubscriptionManager},
    time::{sleep_until, Instant},
    PubSubConnect, PubSubFrontend, RawSubscription, RecoveryBackoff,
};
use alloy_json_rpc::{Id, PubSubItem, Request, Response, ResponsePayload, RpcError, SubId};
use alloy_primitives::B256;
use alloy_transport::{
    utils::{to_json_raw_value, Spawnable},
    TransportErrorKind, TransportResult,
};
#[cfg(not(target_family = "wasm"))]
use futures::future::BoxFuture;
#[cfg(target_family = "wasm")]
use futures::future::LocalBoxFuture as BoxFuture;
use serde_json::value::RawValue;
use std::{future::pending, sync::Arc};
use tokio::sync::{mpsc, oneshot};

#[cfg(test)]
use tokio::time::{sleep, Duration};

enum ConnectionState {
    Connected,
    Waiting(Instant),
    Connecting(BoxFuture<'static, TransportResult<ConnectionHandle>>),
}

enum ReconnectEvent {
    Start,
    Complete(TransportResult<ConnectionHandle>),
}

/// The service contains the backend handle, a subscription manager, and the
/// configuration details required to reconnect.
#[derive(Debug)]
pub(crate) struct PubSubService<T> {
    /// The backend handle.
    pub(crate) handle: ConnectionHandle,

    /// The configuration details required to reconnect.
    pub(crate) connector: Arc<T>,

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
            connector: Arc::new(connector),
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: Default::default(),
        };
        this.spawn();
        Ok(PubSubFrontend::new(tx))
    }

    /// Install a new backend and restore live subscriptions.
    fn install_backend(&mut self, mut old_handle: ConnectionHandle) -> TransportResult<()> {
        std::mem::swap(&mut self.handle, &mut old_handle);

        debug!("Draining old backend to_handle");

        // Drain the old backend
        while let Ok(item) = old_handle.from_socket.try_recv() {
            self.handle_item(item)?;
        }

        old_handle.shutdown();

        self.in_flights.expire();

        // Re-issue pending subscription requests.
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
            let (mut in_flight, _) = InFlight::new(req.clone(), sub.tx.receiver_count());
            in_flight.subscription_replay = true;
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
        if !in_flight.is_live() {
            return Ok(());
        }
        let brv = in_flight.request().serialized().to_owned();
        self.in_flights.insert(in_flight);
        self.dispatch_request(brv)
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
            PubSubInstruction::Cancel(id, identity) => {
                self.in_flights.cancel(&id, &identity);
                Ok(())
            }
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

    fn disconnect(&mut self) -> TransportResult<()> {
        // Process every completed item before failing unresolved reads.
        while let Ok(item) = self.handle.from_socket.try_recv() {
            self.handle_item(item)?;
        }
        self.in_flights.disconnect();
        Ok(())
    }

    fn disconnected_instruction(&mut self, ix: PubSubInstruction) {
        match ix {
            PubSubInstruction::Request(request) if request.is_subscription() => {
                if request.is_live() {
                    self.in_flights.insert(request);
                }
            }
            PubSubInstruction::Request(request) => {
                let _ = request.tx.send(Err(TransportErrorKind::custom(std::io::Error::from(
                    std::io::ErrorKind::NotConnected,
                ))));
            }
            PubSubInstruction::Cancel(id, identity) => self.in_flights.cancel(&id, &identity),
            PubSubInstruction::GetSub(id, tx) => self.service_get_sub(id, tx),
            PubSubInstruction::Unsubscribe(id) => {
                self.subs.remove_sub(id);
            }
        }
    }

    fn fail_requests(&mut self, error: &alloy_transport::TransportError) {
        let detail = error.to_string();
        self.in_flights.fail_all(&detail);
        self.reqs.close();
        while let Ok(instruction) = self.reqs.try_recv() {
            if let PubSubInstruction::Request(request) = instruction {
                let _ = request.tx.send(Err(TransportErrorKind::non_retryable_str(&detail)));
            }
        }
    }

    /// Process cancellation, expiry, and provider closure even during a slow
    /// reconnect or its backoff. Only the provider retries read requests.
    pub(crate) fn spawn(mut self) {
        async move {
            let mut state = ConnectionState::Connected;
            let mut backoff = RecoveryBackoff::default();
            loop {
                self.in_flights.expire();
                let deadline = self.in_flights.next_deadline();
                let connected = matches!(state, ConnectionState::Connected);
                tokio::select! {
                    biased;
                    request = self.reqs.recv() => {
                        let Some(request) = request else { break; };
                        if connected {
                            if let Err(error) = self.service_ix(request) {
                                if !error.as_transport_err().is_some_and(TransportErrorKind::is_backend_gone) { break; }
                                if self.disconnect().is_err() { break; }
                                state = ConnectionState::Waiting(Instant::now() + backoff.next_delay());
                            }
                        } else {
                            self.disconnected_instruction(request);
                        }
                    }
                    () = async {
                        if let Some(deadline) = deadline { sleep_until(deadline).await; }
                        else { pending::<()>().await; }
                    } => { self.in_flights.expire(); }
                    item = self.handle.from_socket.recv(), if connected => {
                        if let Some(item) = item {
                            if self.handle_item(item).is_err() { break; }
                            backoff = RecoveryBackoff::default();
                        } else {
                            if let Ok(error @ RpcError::Transport(TransportErrorKind::NonRetryable(_))) = self.handle.error.try_recv() {
                                self.fail_requests(&error);
                                break;
                            }
                            if self.disconnect().is_err() { break; }
                            state = ConnectionState::Waiting(Instant::now() + backoff.next_delay());
                        }
                    }
                    error = &mut self.handle.error, if connected => {
                        if let Ok(error @ RpcError::Transport(TransportErrorKind::NonRetryable(_))) = error {
                            self.fail_requests(&error);
                            break;
                        }
                        if self.disconnect().is_err() { break; }
                        state = ConnectionState::Waiting(Instant::now() + backoff.next_delay());
                    }
                    event = async {
                        match &mut state {
                            ConnectionState::Connected => pending().await,
                            ConnectionState::Waiting(at) => { sleep_until(*at).await; ReconnectEvent::Start }
                            ConnectionState::Connecting(attempt) => ReconnectEvent::Complete(attempt.await),
                        }
                    }, if !connected => {
                        match event {
                            ReconnectEvent::Start => {
                                let connector = self.connector.clone();
                                state = ConnectionState::Connecting(Box::pin(async move { connector.try_reconnect().await }));
                            }
                            ReconnectEvent::Complete(Ok(handle)) => {
                                if self.install_backend(handle).is_err() {
                                    if self.disconnect().is_err() { break; }
                                    state = ConnectionState::Waiting(Instant::now() + backoff.next_delay());
                                } else {
                                    // A socket handshake alone does not prove recovery.
                                    // Reset backoff only after receiving a response or notification.
                                    state = ConnectionState::Connected;
                                }
                            }
                            ReconnectEvent::Complete(Err(error)) => {
                                if !transient_connect_error(&error) { self.fail_requests(&error); break; }
                                state = ConnectionState::Waiting(Instant::now() + backoff.next_delay());
                            }
                        }
                    }
                }
            }
            self.handle.shutdown();
        }.spawn_task();
    }
}

fn transient_connect_error(error: &alloy_transport::TransportError) -> bool {
    if let RpcError::Transport(TransportErrorKind::HttpError(error)) = error {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    if let RpcError::Transport(TransportErrorKind::BackendGone) = error {
        return true;
    }
    if let RpcError::Transport(TransportErrorKind::Custom(error)) = error {
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error.as_ref());
        while let Some(error) = source {
            if let Some(error) = error.downcast_ref::<std::io::Error>() {
                return matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::NotConnected
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::UnexpectedEof
                );
            }
            source = error.source();
        }
    }
    false
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
            connector: Arc::new(connector),
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
            .expect("manager must return the connection failure")
            .expect_err("raced request should fail when the backend is gone");

        let second =
            Request::new("eth_subscribe", Id::Number(2), ("newHeads",)).serialize().unwrap();
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
            connector: Arc::new(connector),
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
            .expect("manager must return the connection failure")
            .expect_err("request should fail when backend is gone and reconnect aborts");
        tx.closed().await;

        // A permanent failure must stop after one attempt.
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
            connector: Arc::new(connector),
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
            connector: Arc::new(connector),
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        service.spawn();

        // Trigger the legacy close path.
        live_interface.close_with_error();

        // After reconnect, a freshly dispatched request must reach the new
        // backend.
        let req = Request::new("eth_subscribe", Id::Number(1), ("newHeads",)).serialize().unwrap();
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

#[cfg(test)]
mod recovery_regressions {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::time::timeout;

    #[derive(Debug)]
    struct NextConnection(std::sync::Mutex<Option<ConnectionHandle>>);
    impl PubSubConnect for NextConnection {
        fn is_local(&self) -> bool {
            true
        }
        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            self.0.lock().unwrap().take().ok_or_else(TransportErrorKind::backend_gone)
        }
    }

    #[derive(Debug, Clone)]
    struct Refused(Arc<AtomicUsize>);

    impl PubSubConnect for Refused {
        fn is_local(&self) -> bool {
            true
        }
        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(TransportErrorKind::custom(std::io::Error::from(
                std::io::ErrorKind::ConnectionRefused,
            )))
        }
    }

    #[tokio::test]
    async fn dropping_frontend_stops_reconnect_work() {
        let (handle, interface) = ConnectionHandle::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let (tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle,
            connector: Arc::new(Refused(calls.clone())),
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        service.spawn();
        interface.close_with_error();
        timeout(Duration::from_secs(1), async {
            while calls.load(Ordering::SeqCst) == 0 {
                sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        drop(tx);
        sleep(Duration::from_millis(50)).await;
        let stopped = calls.load(Ordering::SeqCst);
        sleep(Duration::from_secs(4)).await;
        assert_eq!(calls.load(Ordering::SeqCst), stopped, "closed provider kept reconnecting");
    }

    #[tokio::test]
    async fn cancelled_request_does_not_replay_after_reconnect() {
        let (handle, interface) = ConnectionHandle::new();
        let (new_handle, mut new_interface) = ConnectionHandle::new();
        let connector = NextConnection(std::sync::Mutex::new(Some(new_handle)));
        let (tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService {
            handle,
            connector: Arc::new(connector),
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };
        let request =
            Request::new("eth_call", Id::Number(91), ("exact", "0x123")).serialize().unwrap();
        let (pending, rx) = InFlight::new(request, 16);
        service.in_flights.insert(pending);
        drop(rx);
        drop(interface);
        let new_handle = service.connector.try_reconnect().await.unwrap();
        service.install_backend(new_handle).unwrap();
        assert!(
            timeout(Duration::from_millis(100), new_interface.recv_from_frontend()).await.is_err(),
            "cancelled request was replayed"
        );
        assert_eq!(service.in_flights.len(), 0);
        drop(tx);
    }
}

#[cfg(test)]
mod unstable_connection_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct ClosesImmediately(Arc<AtomicUsize>);
    impl PubSubConnect for ClosesImmediately {
        fn is_local(&self) -> bool {
            true
        }
        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            self.0.fetch_add(1, Ordering::SeqCst);
            let (handle, interface) = ConnectionHandle::new();
            interface.close_with_error();
            Ok(handle)
        }
    }

    #[tokio::test(start_paused = true)]
    async fn socket_handshakes_without_responses_do_not_reset_backoff() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (handle, interface) = ConnectionHandle::new();
        let (tx, reqs) = mpsc::unbounded_channel();
        PubSubService {
            handle,
            connector: Arc::new(ClosesImmediately(calls.clone())),
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        }
        .spawn();
        interface.close_with_error();
        // Step time so each reconnect receives a polling turn.
        for _ in 0..600 {
            sleep(Duration::from_millis(10)).await;
        }
        let attempts = calls.load(Ordering::SeqCst);
        assert!(
            (12..=27).contains(&attempts),
            "reconnect attempts did not use bounded backoff: {attempts}"
        );
        drop(tx);
        sleep(Duration::from_secs(1)).await;
        assert_eq!(calls.load(Ordering::SeqCst), attempts);
    }
}
