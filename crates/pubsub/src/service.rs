use crate::{
    handle::ConnectionHandle,
    ix::PubSubInstruction,
    managers::{InFlight, RequestManager, SubscriptionManager},
    PubSubConnect, PubSubFrontend, RawSubscription,
};
use alloy_json_rpc::{Id, PubSubItem, Request, Response, ResponsePayload, SubId};
use alloy_primitives::B256;
use alloy_transport::{
    utils::{to_json_raw_value, Spawnable},
    TransportErrorKind, TransportResult,
};
use serde_json::value::RawValue;
use tokio::sync::{mpsc, oneshot};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

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

    /// Consecutive backend deaths where the backend never produced any valid
    /// pubsub traffic. A successful WS handshake alone does not count as
    /// progress; only a received `PubSubItem` does.
    consecutive_unhealthy_backend_deaths: u32,

    /// Whether the current backend has delivered at least one valid
    /// `PubSubItem` before dying.
    backend_had_progress: bool,
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
            consecutive_unhealthy_backend_deaths: 0,
            backend_had_progress: false,
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
        self.backend_had_progress = true;
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

    /// Attempt to reconnect with retries
    async fn reconnect_with_retries(&mut self) -> TransportResult<()> {
        let mut retry_count = 0;
        let max_retries = self.handle.max_retries;
        let interval = self.handle.retry_interval;
        loop {
            match self.reconnect().await {
                Ok(()) => break Ok(()),
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        error!("Reconnect failed after {max_retries} attempts, shutting down: {e}");
                        break Err(e);
                    }
                    warn!(
                        "Reconnection attempt {retry_count}/{max_retries} failed: {e}. \
                         Retrying in {:?}s...",
                        interval.as_secs_f64(),
                    );
                    sleep(interval).await;
                }
            }
        }
    }

    /// Record a backend death and check whether the consecutive-death budget
    /// is exhausted. A backend that delivered at least one valid `PubSubItem`
    /// before dying resets the streak; one that never produced traffic
    /// increments it.
    fn record_backend_death_and_check_budget(&mut self) -> TransportResult<()> {
        if self.backend_had_progress {
            self.consecutive_unhealthy_backend_deaths = 0;
        } else {
            self.consecutive_unhealthy_backend_deaths += 1;
        }
        self.backend_had_progress = false;

        let max = self.handle.max_retries;
        if self.consecutive_unhealthy_backend_deaths > max {
            error!(
                deaths = self.consecutive_unhealthy_backend_deaths,
                max_retries = max,
                "Backend died {deaths} consecutive times without producing valid traffic, \
                 shutting down",
                deaths = self.consecutive_unhealthy_backend_deaths,
            );
            return Err(TransportErrorKind::custom_str(
                "pubsub service exhausted retry budget: backend repeatedly died \
                 without producing valid traffic",
            ));
        }

        Ok(())
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
                            if let Err(e) = self.record_backend_death_and_check_budget() {
                                break Err(e)
                            }
                            if let Err(e) = self.reconnect_with_retries().await {
                                break Err(e)
                            }
                        }
                    }

                    _ = &mut self.handle.error => {
                        error!("Pubsub service backend error.");
                        if let Err(e) = self.record_backend_death_and_check_budget() {
                            break Err(e)
                        }
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
                                    if let Err(e) = self.record_backend_death_and_check_budget() {
                                        break Err(e)
                                    }
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
            }
        };
        fut.spawn_task();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionInterface;
    use alloy_json_rpc::Request;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
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

    #[derive(Clone, Debug)]
    struct MultiMockConnect(Arc<Mutex<VecDeque<ConnectionHandle>>>);

    impl PubSubConnect for MultiMockConnect {
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
                .pop_front()
                .ok_or_else(|| TransportErrorKind::custom_str("no more mock handles"))
        }
    }

    fn make_dead_handle() -> (ConnectionHandle, ConnectionInterface) {
        let (handle, interface) = ConnectionHandle::new();
        (handle, interface)
    }

    fn make_immediately_dying_handle() -> ConnectionHandle {
        let (handle, interface) = ConnectionHandle::new();
        drop(interface.to_frontend);
        let _ = interface.error;
        handle
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
            consecutive_unhealthy_backend_deaths: 0,
            backend_had_progress: false,
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
    async fn consecutive_unhealthy_deaths_exhaust_budget() {
        let max_retries: u32 = 3;

        let mut handles = VecDeque::new();
        for _ in 0..max_retries {
            handles.push_back(
                make_immediately_dying_handle()
                    .with_max_retries(max_retries)
                    .with_retry_interval(Duration::from_millis(10)),
            );
        }
        let connector = MultiMockConnect(Arc::new(Mutex::new(handles)));

        let initial = make_immediately_dying_handle();
        let (tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle: initial
                .with_max_retries(max_retries)
                .with_retry_interval(Duration::from_millis(10)),
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
            consecutive_unhealthy_backend_deaths: 0,
            backend_had_progress: false,
        };
        service.spawn();

        let req = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (in_flight, rx) = InFlight::new(req, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        let result = timeout(Duration::from_secs(5), rx).await;
        assert!(
            result.is_ok(),
            "request should resolve (not hang) once death budget is exhausted"
        );
    }

    #[tokio::test]
    async fn healthy_backend_resets_death_counter() {
        let max_retries: u32 = 2;

        let (healthy_handle, healthy_interface) = make_dead_handle();
        let ConnectionInterface {
            from_frontend: _from_frontend,
            to_frontend: healthy_tx,
            error: _error,
            shutdown: _shutdown,
        } = healthy_interface;

        let dying_after_healthy = make_immediately_dying_handle();

        let (final_handle, mut final_interface) = make_dead_handle();

        let mut handles = VecDeque::new();
        handles.push_back(
            healthy_handle
                .with_max_retries(max_retries)
                .with_retry_interval(Duration::from_millis(10)),
        );
        handles.push_back(
            dying_after_healthy
                .with_max_retries(max_retries)
                .with_retry_interval(Duration::from_millis(10)),
        );
        handles.push_back(final_handle.with_max_retries(max_retries));
        let connector = MultiMockConnect(Arc::new(Mutex::new(handles)));

        let initial = make_immediately_dying_handle();
        let (tx, reqs) = mpsc::unbounded_channel();
        let service = PubSubService {
            handle: initial
                .with_max_retries(max_retries)
                .with_retry_interval(Duration::from_millis(10)),
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
            consecutive_unhealthy_backend_deaths: 0,
            backend_had_progress: false,
        };
        service.spawn();

        // Wait for the service to cycle through the first (immediately dying)
        // backend and land on the healthy one.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send a valid JSON-RPC response through the healthy backend, then
        // close it.
        let raw = RawValue::from_string("\"0x1\"".to_string()).unwrap();
        let resp = Response { id: Id::Number(0), payload: ResponsePayload::Success(raw) };
        let _ = healthy_tx.send(PubSubItem::Response(resp));
        drop(healthy_tx);

        // The service saw progress, so the death counter should have reset.
        // It will now cycle through `dying_after_healthy` (1 unhealthy death)
        // and land on `final_handle`, which is alive.
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Prove the service is still alive by sending a request through
        // the final backend.
        let req = Request::new("eth_chainId", Id::Number(2), ()).serialize().unwrap();
        let expected = req.serialized().get().to_owned();
        let (in_flight, _rx) = InFlight::new(req, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        let dispatched =
            timeout(Duration::from_secs(2), final_interface.recv_from_frontend())
                .await
                .expect("service should still be alive after counter reset")
                .expect("final backend should receive the request");
        assert_eq!(dispatched.get(), expected);
    }
}
