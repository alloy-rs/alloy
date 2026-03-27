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
use std::time::Instant;
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

    /// Tracks reconnect loops that repeatedly fail before the backend makes progress.
    reconnects: ReconnectTracker,
}

const RAPID_RECONNECT_ERR: &str = "pubsub service exceeded rapid reconnect limit";
// Replayed in-flight requests can otherwise recreate the same backend failure forever.
const MAX_RAPID_RECONNECTS: u32 = 10;

#[derive(Debug, Default)]
struct ReconnectTracker {
    rapid_reconnects: u32,
    last_reconnect: Option<Instant>,
}

impl ReconnectTracker {
    fn record_reconnect(
        &mut self,
        now: Instant,
        retry_interval: std::time::Duration,
    ) -> TransportResult<()> {
        self.rapid_reconnects =
            if self.last_reconnect.is_some_and(|last| now.duration_since(last) <= retry_interval) {
                self.rapid_reconnects.saturating_add(1)
            } else {
                1
            };
        self.last_reconnect = Some(now);

        if self.rapid_reconnects > MAX_RAPID_RECONNECTS {
            return Err(TransportErrorKind::custom_str(RAPID_RECONNECT_ERR));
        }

        Ok(())
    }

    fn record_progress(&mut self) {
        self.rapid_reconnects = 0;
        self.last_reconnect = None;
    }
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
            reconnects: Default::default(),
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

    async fn reconnect_or_fail(&mut self) -> TransportResult<()> {
        self.reconnect_with_retries().await?;
        self.reconnects.record_reconnect(Instant::now(), self.handle.retry_interval)
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
                            self.reconnects.record_progress();
                        } else if let Err(e) = self.reconnect_or_fail().await {
                            break Err(e)
                        }
                    }

                    _ = &mut self.handle.error => {
                        error!("Pubsub service backend error.");
                        if let Err(e) = self.reconnect_or_fail().await {
                            break Err(e)
                        }
                    }

                    req_opt = self.reqs.recv() => {
                        if let Some(req) = req_opt {
                            if let Err(e) = self.service_ix(req) {
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
    use alloy_json_rpc::{Id, Request};
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };
    use tokio::time::timeout;

    #[test]
    fn reconnect_tracker_limits_rapid_reconnects() {
        let now = Instant::now();
        let mut tracker = ReconnectTracker::default();

        for attempt in 0..MAX_RAPID_RECONNECTS {
            tracker
                .record_reconnect(
                    now + Duration::from_millis(u64::from(attempt) * 100),
                    Duration::from_secs(1),
                )
                .unwrap();
        }
        let err = tracker
            .record_reconnect(
                now + Duration::from_millis(u64::from(MAX_RAPID_RECONNECTS) * 100),
                Duration::from_secs(1),
            )
            .unwrap_err();

        assert_eq!(err.to_string(), RAPID_RECONNECT_ERR);
    }

    #[test]
    fn reconnect_tracker_resets_after_progress() {
        let now = Instant::now();
        let mut tracker = ReconnectTracker::default();

        tracker.record_reconnect(now, Duration::from_secs(1)).unwrap();
        tracker.record_progress();
        tracker.record_reconnect(now + Duration::from_millis(100), Duration::from_secs(1)).unwrap();
    }

    #[derive(Clone, Debug)]
    struct FailingReplayConnect {
        connects: Arc<AtomicUsize>,
        requests: Arc<AtomicUsize>,
        max_retries: u32,
        retry_interval: Duration,
    }

    impl FailingReplayConnect {
        fn new(max_retries: u32, retry_interval: Duration) -> Self {
            Self {
                connects: Arc::new(AtomicUsize::new(0)),
                requests: Arc::new(AtomicUsize::new(0)),
                max_retries,
                retry_interval,
            }
        }
    }

    impl PubSubConnect for FailingReplayConnect {
        fn is_local(&self) -> bool {
            true
        }

        async fn connect(&self) -> TransportResult<ConnectionHandle> {
            self.connects.fetch_add(1, Ordering::SeqCst);

            let (handle, mut interface) = ConnectionHandle::new();
            let requests = self.requests.clone();
            tokio::spawn(async move {
                if interface.recv_from_frontend().await.is_some() {
                    requests.fetch_add(1, Ordering::SeqCst);
                    interface.close_with_error();
                }
            });

            Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stops_replayed_requests_from_reconnecting_forever() {
        let connector = FailingReplayConnect::new(u32::MAX, Duration::from_millis(50));
        let frontend = PubSubService::connect(connector.clone()).await.unwrap();
        let request = Request::new("eth_getLogs", Id::Number(1), ()).serialize().unwrap();

        let err = timeout(Duration::from_secs(1), frontend.send(request))
            .await
            .expect("request should complete")
            .unwrap_err();

        assert_eq!(err.to_string(), "backend connection task has stopped");
        assert!(connector.connects.load(Ordering::SeqCst) > 1);
        assert!(connector.requests.load(Ordering::SeqCst) <= MAX_RAPID_RECONNECTS as usize + 2);
    }
}
