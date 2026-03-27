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
    TransportError, TransportErrorKind, TransportResult,
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
            // Same as `dispatch_request`, but inlined to avoid double-borrowing `self`.
            self.handle.to_socket.send(msg).map_err(|_| TransportErrorKind::backend_gone())?;
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
            self.handle.to_socket.send(msg).map_err(|_| TransportErrorKind::backend_gone())?;
        }

        Ok(())
    }

    /// Dispatch a request to the socket.
    fn dispatch_request(&self, brv: Box<RawValue>) -> TransportResult<()> {
        self.handle.to_socket.send(brv).map(drop).map_err(|_| TransportErrorKind::backend_gone())
    }

    fn is_backend_gone(err: &TransportError) -> bool {
        err.as_transport_err().is_some_and(TransportErrorKind::is_backend_gone)
    }

    /// Service a request, reconnecting and retrying if the backend disappeared
    /// before the request could be dispatched.
    async fn service_request_with_reconnect(&mut self, in_flight: InFlight) -> TransportResult<()> {
        loop {
            match self.dispatch_request(in_flight.request().serialized().to_owned()) {
                Ok(()) => {
                    self.in_flights.insert(in_flight);
                    return Ok(());
                }
                Err(err) if Self::is_backend_gone(&err) => self.reconnect_with_retries().await?,
                Err(err) => return Err(err),
            }
        }
    }

    /// Service a GetSub instruction.
    ///
    /// If the subscription exists, the waiter is sent `Some` broadcast receiver. If
    /// the subscription does not exist, the waiter is sent `None`.
    fn service_get_sub(&self, local_id: B256, tx: oneshot::Sender<Option<RawSubscription>>) {
        let _ = tx.send(self.subs.get_subscription(local_id));
    }

    /// Service an unsubscribe instruction without resurrecting the
    /// subscription during reconnect.
    async fn service_unsubscribe_with_reconnect(&mut self, local_id: B256) -> TransportResult<()> {
        let server_id = self.subs.server_id_for(&local_id).cloned();

        // Remove local state before reconnecting so this subscription is not
        // reissued by a successful reconnect.
        self.subs.remove_sub(local_id);
        let removed = self.in_flights.remove_subscription_requests(&local_id);
        trace!(?local_id, removed, "removed pending resubscribe requests");

        if let Some(server_id) = server_id {
            // TODO: ideally we can send this with an unused id
            let req = Request::new("eth_unsubscribe", Id::Number(1), [server_id]);
            let brv = req.serialize().expect("no ser error").take_request();

            match self.dispatch_request(brv) {
                Ok(()) => Ok(()),
                Err(err) if Self::is_backend_gone(&err) => self.reconnect_with_retries().await,
                Err(err) => Err(err),
            }
        } else {
            Ok(())
        }
    }

    /// Service an instruction, reconnecting when the backend disappears while
    /// dispatching user-driven work.
    async fn service_ix_with_reconnect(&mut self, ix: PubSubInstruction) -> TransportResult<()> {
        trace!(?ix, "servicing instruction with reconnect");
        match ix {
            PubSubInstruction::Request(in_flight) => {
                self.service_request_with_reconnect(in_flight).await
            }
            PubSubInstruction::GetSub(alias, tx) => {
                self.service_get_sub(alias, tx);
                Ok(())
            }
            PubSubInstruction::Unsubscribe(alias) => {
                self.service_unsubscribe_with_reconnect(alias).await
            }
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
                        } else if let Err(e) = self.reconnect_with_retries().await {
                            break Err(e)
                        }
                    }

                    _ = &mut self.handle.error => {
                        error!("Pubsub service backend error.");
                        if let Err(e) = self.reconnect_with_retries().await {
                            break Err(e)
                        }
                    }

                    req_opt = self.reqs.recv() => {
                        if let Some(req) = req_opt {
                            if let Err(e) = self.service_ix_with_reconnect(req).await {
                                break Err(e)
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
    use alloy_json_rpc::{PubSubItem, Request, ResponsePayload, SubId};
    use serde::Deserialize;
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    };
    use tokio::time::timeout;

    #[derive(Debug)]
    struct HeldDeadBackend {
        _to_frontend: mpsc::UnboundedSender<PubSubItem>,
        _error: oneshot::Sender<()>,
        _shutdown: oneshot::Receiver<()>,
    }

    #[derive(Clone, Debug, Default)]
    struct MockConnect {
        handles: Arc<Mutex<VecDeque<ConnectionHandle>>>,
        reconnects: Arc<AtomicUsize>,
    }

    impl MockConnect {
        fn new(handles: Vec<ConnectionHandle>) -> Self {
            Self {
                handles: Arc::new(Mutex::new(handles.into())),
                reconnects: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn reconnects(&self) -> usize {
            self.reconnects.load(Ordering::Relaxed)
        }

        fn next_handle(&self) -> TransportResult<ConnectionHandle> {
            self.handles
                .lock()
                .expect("poisoned mutex")
                .pop_front()
                .ok_or_else(|| TransportErrorKind::custom_str("missing mock connection handle"))
        }
    }

    impl PubSubConnect for MockConnect {
        fn is_local(&self) -> bool {
            true
        }

        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            self.next_handle()
        }

        async fn try_reconnect(&self) -> TransportResult<ConnectionHandle> {
            self.reconnects.fetch_add(1, Ordering::Relaxed);
            self.next_handle()
        }
    }

    fn dead_handle() -> (ConnectionHandle, HeldDeadBackend) {
        let (handle, interface) = ConnectionHandle::new();
        let ConnectionInterface { from_frontend, to_frontend, error, shutdown } = interface;
        drop(from_frontend);
        (handle, HeldDeadBackend { _to_frontend: to_frontend, _error: error, _shutdown: shutdown })
    }

    fn response_handle(result: Box<RawValue>) -> ConnectionHandle {
        let (handle, mut interface) = ConnectionHandle::new();
        tokio::spawn(async move {
            #[derive(Deserialize)]
            struct IncomingRequest {
                id: Id,
            }

            while let Some(msg) = interface.recv_from_frontend().await {
                let request: IncomingRequest =
                    serde_json::from_str(msg.get()).expect("valid serialized request");
                let response = alloy_json_rpc::Response {
                    id: request.id,
                    payload: ResponsePayload::Success(result.clone()),
                };
                if interface.send_to_frontend(PubSubItem::Response(response)).is_err() {
                    break;
                }
            }
        });
        handle
    }

    #[tokio::test]
    async fn reconnects_request_dispatches_when_backend_is_already_gone() {
        let (dead, _guard) = dead_handle();
        let connector =
            MockConnect::new(vec![dead, response_handle(to_json_raw_value(&"0x1").unwrap())]);
        let frontend = PubSubService::connect(connector.clone()).await.unwrap();

        let request = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let response = timeout(Duration::from_secs(1), frontend.send(request))
            .await
            .expect("request should not hang")
            .expect("request should succeed");

        assert_eq!(connector.reconnects(), 1);
        match response.payload {
            ResponsePayload::Success(result) => assert_eq!(result.get(), r#""0x1""#),
            payload => panic!("unexpected payload: {payload:?}"),
        }
    }

    #[tokio::test]
    async fn unsubscribe_clears_pending_resubscribe_responses() {
        let (handle, _guard) = dead_handle();
        let (_tx, reqs) = mpsc::unbounded_channel();
        let connector = MockConnect::default();
        let mut service = PubSubService {
            handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: RequestManager::default(),
        };

        let request =
            Request::new("eth_subscribe", Id::Number(7), ("newHeads",)).serialize().unwrap();
        let local_id = request.params_hash();

        let (in_flight, _) = InFlight::new(request.clone(), 16);
        service.handle_sub_response(in_flight, SubId::from(String::from("0xdeadbeef"))).unwrap();
        assert!(service.subs.get_subscription(local_id).is_some());

        service.subs.drop_server_ids();
        let (pending_resubscribe, _) = InFlight::new(request.clone(), 16);
        service.in_flights.insert(pending_resubscribe);
        assert_eq!(service.in_flights.len(), 1);

        service.service_unsubscribe_with_reconnect(local_id).await.unwrap();
        assert!(service.subs.get_subscription(local_id).is_none());
        assert_eq!(service.in_flights.len(), 0);

        let late_response = alloy_json_rpc::Response {
            id: request.id().clone(),
            payload: ResponsePayload::Success(
                to_json_raw_value(&SubId::from(String::from("0xbeef"))).unwrap(),
            ),
        };
        service.handle_item(PubSubItem::Response(late_response)).unwrap();

        assert!(service.subs.get_subscription(local_id).is_none());
    }
}
