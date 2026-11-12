use crate::{
    handle::ConnectionHandle,
    ix::PubSubInstruction,
    managers::{subscription_local_id, InFlight, RequestManager, SubscriptionManager},
    PubSubConnect, PubSubFrontend, RawSubscription,
};
use alloy_json_rpc::{Id, PubSubItem, Request, Response, ResponsePayload, RpcError, SubId};
use alloy_primitives::{map::B256Map, B256};
use alloy_transport::{
    utils::{to_json_raw_value, Spawnable},
    SharedTransportError, TransportError, TransportErrorKind, TransportResult,
};
use serde_json::value::RawValue;
use std::{borrow::Cow, time::Duration};
use tokio::sync::{mpsc, oneshot};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

const MAX_RECONNECT_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// The number of times a request is replayed on a new backend before it fails, when every backend
/// dies while the request is pending. This also bounds resubscriptions.
const MAX_REQUEST_REPLAYS: u32 = 3;

/// A subscribe request on the wire, and the callers waiting for the same subscription.
#[derive(Debug)]
struct PendingSubscription {
    /// The id of the request on the wire.
    id: Id,
    /// Callers that asked for the same subscription while the request was in flight.
    waiters: Vec<InFlight>,
}

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

    /// Subscribe requests on the wire, by local id. Identical subscribe requests wait for these
    /// instead of creating another subscription on the server.
    pending_subs: B256Map<PendingSubscription>,

    /// Whether the current backend answered any request since it connected.
    backend_progressed: bool,

    /// The number of consecutive backends that died with requests pending, without answering
    /// any request.
    unhealthy_backends: u32,
}

impl<T: PubSubConnect> PubSubService<T> {
    /// Create a new service from a connected backend.
    pub(crate) fn new(
        handle: ConnectionHandle,
        connector: T,
        reqs: mpsc::UnboundedReceiver<PubSubInstruction>,
    ) -> Self {
        Self {
            handle,
            connector,
            reqs,
            subs: SubscriptionManager::default(),
            in_flights: Default::default(),
            pending_subs: Default::default(),
            backend_progressed: false,
            unhealthy_backends: 0,
        }
    }

    /// Create a new service from a connector.
    pub(crate) async fn connect(connector: T) -> TransportResult<PubSubFrontend> {
        let handle = connector.connect().await?;

        let (tx, reqs) = mpsc::unbounded_channel();
        Self::new(handle, connector, reqs).spawn();
        Ok(PubSubFrontend::new(tx))
    }

    /// Handle the items the current backend sent and the service has not read yet.
    fn drain_backend(&mut self) -> TransportResult<()> {
        while let Ok(item) = self.handle.from_socket.try_recv() {
            self.handle_item(item)?;
        }
        Ok(())
    }

    /// Reconnect the backend, re-issue pending requests, and re-start active
    /// subscriptions.
    async fn reconnect(&mut self) -> TransportResult<()> {
        debug!("Reconnecting pubsub service backend");

        let handle = self.connector.try_reconnect().await?;

        debug!("Draining old backend to_handle");

        // Drain the old backend while it is still the current one, so that cancelling a
        // subscription it created doesn't reach the replacement.
        self.drain_backend()?;

        std::mem::replace(&mut self.handle, handle).shutdown();
        self.backend_progressed = false;

        // Subscriptions nobody consumes are not restored: their server-side counterparts closed
        // with the old connection.
        self.subs.retain_consumed();

        // Re-issue pending requests. Dispatch errors are ignored: if the replacement already
        // died, the service loop notices and accounts for its death like any other.
        self.remove_cancelled();
        debug!(count = self.in_flights.len(), "Reissuing pending requests");
        for in_flight in self.in_flights.iter() {
            let msg = in_flight.request.serialized().to_owned();
            let _ = self.dispatch_request(msg);
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
            let _ = self.dispatch_request(msg);
        }

        Ok(())
    }

    /// Discard the pending requests that must not be replayed on a new backend.
    ///
    /// Requests are discarded once nobody waits for their response, with two exceptions for
    /// subscribe requests: pending resubscriptions are always discarded because they are re-issued
    /// from their subscriptions, and a subscribe request is kept while another caller waits for
    /// the same subscription.
    fn remove_cancelled(&mut self) {
        let Self { in_flights, subs, pending_subs, .. } = self;
        in_flights.retain(|in_flight| {
            if !in_flight.tx.is_closed() {
                return true;
            }
            if !in_flight.is_subscription() {
                return false;
            }
            let local_id = subscription_local_id(in_flight.request());
            if subs.contains(&local_id) {
                return false;
            }
            let Some(pending) = pending_subs
                .get_mut(&local_id)
                .filter(|pending| pending.id == *in_flight.request.id())
            else {
                return false;
            };
            pending.waiters.retain(|waiter| !waiter.tx.is_closed());
            if pending.waiters.is_empty() {
                pending_subs.remove(&local_id);
                return false;
            }
            true
        });
    }

    /// Dispatch a request to the socket.
    fn dispatch_request(&self, brv: Box<RawValue>) -> TransportResult<()> {
        self.handle.to_socket.send(brv).map(drop).map_err(|_| TransportErrorKind::backend_gone())
    }

    /// Service a request.
    fn service_request(&mut self, in_flight: InFlight) -> TransportResult<()> {
        // A caller may have stopped waiting while reconnect blocked this queue.
        if in_flight.tx.is_closed() {
            return Ok(());
        }
        if in_flight.is_subscription() {
            // Identical subscribe requests share one subscription, instead of each creating
            // another one on the server. A subscription that is not routed, because its
            // resubscription is still in flight, is routed again by the request.
            let local_id = subscription_local_id(in_flight.request());
            if self.subs.is_routed(&local_id) {
                let id = in_flight.request.id().clone();
                self.deliver_local_id(local_id, id, in_flight.tx)?;
                return Ok(());
            }
            if let Some(pending) = self.pending_subs.get_mut(&local_id) {
                pending.waiters.push(in_flight);
                return Ok(());
            }
            let id = in_flight.request.id().clone();
            self.pending_subs.insert(local_id, PendingSubscription { id, waiters: Vec::new() });
        }
        let brv = in_flight.request().serialized().to_owned();
        // Retain the waiter before dispatch: a backend failure must either replay the request
        // after reconnecting or deliver the terminal connection error to its caller.
        self.in_flights.insert(in_flight);
        self.dispatch_request(brv)?;

        Ok(())
    }

    /// Send the local id of a subscription to a caller waiting for it, returning whether the
    /// caller received it.
    fn deliver_local_id(
        &mut self,
        local_id: B256,
        id: Id,
        tx: oneshot::Sender<TransportResult<Response>>,
    ) -> TransportResult<bool> {
        // Serialized B256 is always a valid serialized U256 too.
        let ser_alias = to_json_raw_value(&local_id)?;
        let delivered =
            tx.send(Ok(Response { id, payload: ResponsePayload::Success(ser_alias) })).is_ok();
        if delivered {
            self.subs.add_pending_claim(local_id);
        }
        Ok(delivered)
    }

    /// Service a GetSub instruction.
    ///
    /// If the subscription exists, the waiter is sent `Some` broadcast receiver. If
    /// the subscription does not exist, the waiter is sent `None`.
    fn service_get_sub(&mut self, local_id: B256, tx: oneshot::Sender<Option<RawSubscription>>) {
        let _ = tx.send(self.subs.get_subscription(local_id));
    }

    /// Service an unsubscribe instruction.
    ///
    /// Identical subscribe requests share a subscription, so it is only removed, and cancelled on
    /// the server, once every caller that was handed its local id unsubscribed.
    fn service_unsubscribe(&mut self, local_id: B256) {
        if let Some((sub, Some(server_id))) = self.subs.release(local_id) {
            self.dispatch_unsubscribe(sub.request().method(), local_id, &server_id);
        }
    }

    /// Ask the server to cancel the subscription `server_id`, which a `subscribe_method` request
    /// created.
    ///
    /// This is best effort and never fails: if the backend is gone, so is the server-side
    /// subscription.
    fn dispatch_unsubscribe(&self, subscribe_method: &str, local_id: B256, server_id: &SubId) {
        // A string id keeps the reply from matching a request issued by `RpcClient`,
        // which allocates numeric ids. It is not tracked in `in_flights`, so the reply is
        // dropped on arrival.
        let id = Id::String(format!("unsubscribe-{local_id}"));
        let req = Request::new(unsubscribe_method(subscribe_method), id, [server_id]);
        let brv = req.serialize().expect("no ser error").take_request();
        if self.dispatch_request(brv).is_err() {
            debug!(%local_id, ?server_id, "Backend gone, not sending unsubscribe");
        }
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
            PubSubInstruction::Unsubscribe(alias) => {
                self.service_unsubscribe(alias);
                Ok(())
            }
        }
    }

    /// Handle an item from the backend.
    fn handle_item(&mut self, item: PubSubItem) -> TransportResult<()> {
        match item {
            PubSubItem::Response(resp) => {
                let Some(in_flight) = self.in_flights.remove(&resp.id) else {
                    return Ok(());
                };
                self.backend_progressed = true;
                if in_flight.is_subscription() {
                    return self.handle_sub_response(in_flight, resp);
                }
                let _ = in_flight.fulfill(resp);
                Ok(())
            }
            PubSubItem::Notification(notification) => {
                self.subs.notify(notification);
                Ok(())
            }
        }
    }

    /// Take the callers that asked for the subscription `local_id` while the subscribe request
    /// `id` was in flight.
    fn take_waiters(&mut self, local_id: B256, id: &Id) -> Vec<InFlight> {
        match self.pending_subs.get(&local_id) {
            Some(pending) if pending.id == *id => {
                self.pending_subs.remove(&local_id).map(|pending| pending.waiters)
            }
            _ => None,
        }
        .unwrap_or_default()
    }

    /// Handle the response to a subscribe request.
    ///
    /// On success, the subscription is created or restored, and its local id is sent to every
    /// caller waiting for it. A server id that won't be routed, because nobody waits for the
    /// subscription anymore or because the subscription is already routed via another server id,
    /// is cancelled on the server. A failed resubscription removes the subscription, which closes
    /// its receivers.
    fn handle_sub_response(&mut self, in_flight: InFlight, resp: Response) -> TransportResult<()> {
        let local_id = subscription_local_id(in_flight.request());

        let waiters = self.take_waiters(local_id, &resp.id);
        let resubscription = self.subs.is_resubscription(&local_id, &resp.id);
        let payload = (!waiters.is_empty() || resubscription).then(|| resp.payload.clone());

        let Some((server_id, in_flight)) = in_flight.fulfill(resp) else {
            // Nobody waits for the response, so without a server id the subscription would stay
            // silent. An identical subscribe request creates it afresh.
            if resubscription {
                warn!(%local_id, ?payload, "Resubscription failed, dropping the subscription");
                self.subs.remove_sub(local_id);
            }
            // The subscription failed for every caller waiting for it.
            if let Some(payload) = payload {
                for waiter in waiters {
                    let id = waiter.request.id().clone();
                    let _ = waiter.fulfill(Response { id, payload: payload.clone() });
                }
            }
            return Ok(());
        };

        let InFlight { request, channel_size, tx } = in_flight;
        let id = request.id().clone();
        let created = !self.subs.contains(&local_id);
        if created {
            // Nobody waits for the subscription, e.g. because its caller was cancelled, or because
            // it was unsubscribed while being re-issued after a reconnect.
            if tx.is_closed() && waiters.iter().all(|waiter| waiter.tx.is_closed()) {
                self.dispatch_unsubscribe(request.method(), local_id, &server_id);
                return Ok(());
            }
            self.subs.insert(request, server_id, channel_size);
        } else if let Err(server_id) = self.subs.set_server_id(local_id, server_id) {
            // The subscription is already routed via another server id.
            self.dispatch_unsubscribe(request.method(), local_id, &server_id);
        }

        let mut delivered = self.deliver_local_id(local_id, id, tx)?;
        for waiter in waiters {
            let id = waiter.request.id().clone();
            delivered |= self.deliver_local_id(local_id, id, waiter.tx)?;
        }

        // Every caller stopped waiting while the subscription was being created.
        if created && !delivered {
            self.service_unsubscribe(local_id);
        }

        Ok(())
    }

    /// Attempt to reconnect with retries, after accounting for the death of the current backend.
    ///
    /// Aborts immediately when a reconnect attempt returns a
    /// [`TransportErrorKind::NonRetryable`] error so deterministic backend
    /// failures (auth/protocol violations, malformed handshake, etc.) do not
    /// burn the full retry budget.
    async fn reconnect_with_retries(&mut self) -> TransportResult<()> {
        // Answers the backend sent before it died count for its health and their requests.
        self.drain_backend()?;
        self.handle_backend_death().await?;

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

    /// Account for the death of the current backend, before replacing it.
    ///
    /// A request fails once more than [`MAX_REQUEST_REPLAYS`] backends died while it was pending,
    /// and a subscription is dropped once that many consecutive backends answered other requests
    /// but died without acknowledging its resubscription. Backends that answered nothing don't
    /// count for resubscriptions: the health budget bounds them, and dropping a subscription that
    /// still has consumers would end their streams while the service keeps running.
    ///
    /// A backend is unhealthy if it died with requests pending, without answering any request
    /// since it connected. Notifications don't count. A backend that answered a request resets the
    /// count of consecutive unhealthy backends, and one that died idle leaves it unchanged. From
    /// the second unhealthy backend on, the service backs off before connecting, and after
    /// `max_retries` it shuts down, even if connecting keeps succeeding.
    async fn handle_backend_death(&mut self) -> TransportResult<()> {
        if self.backend_progressed {
            self.unhealthy_backends = 0;
        } else if !self.in_flights.is_empty() {
            self.unhealthy_backends = self.unhealthy_backends.saturating_add(1);
        }

        for in_flight in self.in_flights.count_replays(MAX_REQUEST_REPLAYS) {
            warn!(
                method = in_flight.request.method(),
                "Backend died {} times while the request was pending, failing it",
                MAX_REQUEST_REPLAYS + 1
            );
            self.fail_replayed_request(in_flight);
        }
        if self.backend_progressed {
            self.subs.count_unacked_resubscribes(MAX_REQUEST_REPLAYS);
        }

        let unhealthy = self.unhealthy_backends;
        let max_retries = self.handle.max_retries;
        if unhealthy > max_retries {
            error!(
                "{unhealthy} consecutive backends died without answering a request, shutting down"
            );
            return Err(TransportErrorKind::custom_str(&format!(
                "{unhealthy} consecutive backend connections closed without answering a request"
            )));
        }
        if unhealthy > 1 {
            let delay = reconnect_retry_interval(self.handle.retry_interval, unhealthy - 1);
            warn!(
                "{unhealthy} consecutive backends died without answering a request. \
                 Reconnecting in {delay:?}...",
            );
            sleep(delay).await;
        }
        Ok(())
    }

    /// Fail a request that was pending on too many dead backends, along with the callers waiting
    /// for the same subscription.
    fn fail_replayed_request(&mut self, in_flight: InFlight) {
        let error = || {
            TransportErrorKind::custom_str(&format!(
                "backend connection closed {} times while the request was pending",
                MAX_REQUEST_REPLAYS + 1
            ))
        };
        if in_flight.is_subscription() {
            let local_id = subscription_local_id(in_flight.request());
            for waiter in self.take_waiters(local_id, in_flight.request.id()) {
                let _ = waiter.tx.send(Err(error()));
            }
        }
        let _ = in_flight.tx.send(Err(error()));
    }

    /// Handle the loss of the backend, given the error it reported, if any.
    ///
    /// Non-retryable errors are returned, otherwise the backend is reconnected with retries.
    async fn handle_backend_error(&mut self, err: Option<TransportError>) -> TransportResult<()> {
        if let Some(err) = err {
            if matches!(&err, RpcError::Transport(k) if k.is_non_retryable()) {
                error!(%err, "Pubsub service backend reported a non-retryable error, shutting down.");
                return Err(err);
            }
            error!(%err, "Pubsub service backend error.");
        }
        self.reconnect_with_retries().await
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
                            let err = self.handle.error.try_recv().ok();
                            if let Err(e) = self.handle_backend_error(err).await {
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
                        if let Err(e) = self.handle_backend_error(Some(err)).await {
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
                self.fail_pending_requests(err).await;
            }
        };
        fut.spawn_task();
    }

    /// Resolve outstanding requests with the terminal error that stopped the service.
    async fn fail_pending_requests(&mut self, error: TransportError) {
        let error = SharedTransportError::new(error);
        for request in self.in_flights.drain() {
            let _ = request.tx.send(Err(error.to_error()));
        }
        for waiter in self.pending_subs.drain().flat_map(|(_, pending)| pending.waiters) {
            let _ = waiter.tx.send(Err(error.to_error()));
        }
        // Requests queued while reconnecting have not entered `in_flights` yet.
        self.reqs.close();
        while let Some(instruction) = self.reqs.recv().await {
            if let PubSubInstruction::Request(request) = instruction {
                let _ = request.tx.send(Err(error.to_error()));
            }
        }
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

/// Returns the method that cancels a subscription created by `subscribe_method`.
///
/// The namespace is taken from the subscribe method, e.g. `eth_subscribe` maps to
/// `eth_unsubscribe`. Methods named `{namespace}_subscribe{Name}` map to
/// `{namespace}_unsubscribe{Name}`, e.g. `reth_subscribeChainNotifications` to
/// `reth_unsubscribeChainNotifications`. Other methods map to `{method}_unsubscribe`, as reth's
/// `admin_peerEvents` does to `admin_peerEvents_unsubscribe`. Methods without a namespace fall
/// back to `eth_unsubscribe`.
fn unsubscribe_method(subscribe_method: &str) -> Cow<'static, str> {
    match subscribe_method.split_once('_') {
        None | Some(("eth", "subscribe")) => Cow::Borrowed("eth_unsubscribe"),
        Some((namespace, name)) => match name.strip_prefix("subscribe") {
            Some(suffix) => Cow::Owned(format!("{namespace}_unsubscribe{suffix}")),
            None => Cow::Owned(format!("{subscribe_method}_unsubscribe")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionInterface;
    use alloy_json_rpc::Request;
    use alloy_primitives::U256;
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
    async fn reconnect_does_not_replay_dropped_request_futures() {
        let (handle, mut interface) = ConnectionHandle::new();
        let (replacement, mut replacement_interface) = ConnectionHandle::new();
        let (tx, reqs) = mpsc::unbounded_channel();
        let frontend = PubSubFrontend::new(tx);
        let mut service =
            PubSubService::new(handle, MockConnect(Arc::new(Mutex::new(Some(replacement)))), reqs);

        let cancelled = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let mut cancelled = Box::pin(frontend.send(cancelled));
        assert!(futures::poll!(&mut cancelled).is_pending());
        let instruction = service.reqs.try_recv().unwrap();
        service.service_ix(instruction).unwrap();
        interface.from_frontend.try_recv().unwrap();

        let live = Request::new("eth_chainId", Id::Number(2), ()).serialize().unwrap();
        let expected = live.serialized().get().to_owned();
        let mut live = Box::pin(frontend.send(live));
        assert!(futures::poll!(&mut live).is_pending());
        let instruction = service.reqs.try_recv().unwrap();
        service.service_ix(instruction).unwrap();
        assert_eq!(interface.from_frontend.try_recv().unwrap().get(), expected);

        drop(cancelled);
        service.reconnect().await.unwrap();

        assert_eq!(replacement_interface.from_frontend.try_recv().unwrap().get(), expected);
        assert!(matches!(
            replacement_interface.from_frontend.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        assert_eq!(service.in_flights.len(), 1);

        service
            .handle_item(
                serde_json::from_str(r#"{"jsonrpc":"2.0","id":2,"result":"0x1"}"#).unwrap(),
            )
            .unwrap();
        let response = live.await.unwrap();
        assert_eq!(response.id, Id::Number(2));
        assert_eq!(response.payload.as_success().unwrap().get(), r#""0x1""#);
        assert_eq!(service.in_flights.len(), 0);
    }

    #[tokio::test]
    async fn cancelled_queued_request_is_not_dispatched_after_reconnect() {
        let (handle, _interface) = ConnectionHandle::new();
        let (replacement, mut replacement_interface) = ConnectionHandle::new();
        let (tx, reqs) = mpsc::unbounded_channel();
        let frontend = PubSubFrontend::new(tx);
        let mut service =
            PubSubService::new(handle, MockConnect(Arc::new(Mutex::new(Some(replacement)))), reqs);
        let request = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let mut request = Box::pin(frontend.send(request));
        assert!(futures::poll!(&mut request).is_pending());
        // While reconnecting, the service leaves new instructions in its queue.
        // The caller can stop waiting before the service resumes that queue.
        drop(request);
        service.reconnect().await.unwrap();
        let instruction = service.reqs.try_recv().unwrap();
        service.service_ix(instruction).unwrap();
        assert!(matches!(
            replacement_interface.from_frontend.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        assert_eq!(service.in_flights.len(), 0);
    }

    #[tokio::test]
    async fn reconnect_preserves_pending_subscription_without_response_receiver() {
        let (handle, _interface) = ConnectionHandle::new();
        let (replacement, mut replacement_interface) = ConnectionHandle::new();
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service =
            PubSubService::new(handle, MockConnect(Arc::new(Mutex::new(Some(replacement)))), reqs);
        let request =
            Request::new("eth_subscribe", Id::Number(3), ["newHeads"]).serialize().unwrap();
        let expected = request.serialized().get().to_owned();
        // A consumed subscription whose resubscription after an earlier reconnect is pending.
        let local_id = service.subs.insert(request.clone(), SubId::Number(U256::from(1)), 16);
        let _sub = service.subs.get_subscription(local_id).unwrap();
        service.subs.drop_server_ids();
        // Internal resubscription requests have no oneshot receiver: delivery
        // uses the subscription's broadcast channel instead.
        let (pending, response) = InFlight::new(request, 16);
        drop(response);
        service.in_flights.insert(pending);

        service.reconnect().await.unwrap();

        // The resubscription is issued exactly once.
        assert_eq!(replacement_interface.from_frontend.try_recv().unwrap().get(), expected);
        assert!(matches!(
            replacement_interface.from_frontend.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        assert_eq!(service.in_flights.len(), 1);
    }

    #[tokio::test]
    async fn answers_from_dying_backend_count_before_its_death() {
        let (handle, interface) = ConnectionHandle::new();
        let (replacement, mut replacement_interface) = ConnectionHandle::new();
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service =
            PubSubService::new(handle, MockConnect(Arc::new(Mutex::new(Some(replacement)))), reqs);
        let request = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (in_flight, response) = InFlight::new(request, 16);
        service.in_flights.insert(in_flight);
        let request =
            Request::new("eth_subscribe", Id::Number(2), ["newHeads"]).serialize().unwrap();
        let (in_flight, cancelled) = InFlight::new(request, 16);
        drop(cancelled);
        service.in_flights.insert(in_flight);

        // The backend answers both requests and dies before the service reads the answers.
        for answer in [
            r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":"0xaaa"}"#,
        ] {
            interface.send_to_frontend(serde_json::from_str(answer).unwrap()).unwrap();
        }
        drop(interface);
        service.reconnect_with_retries().await.unwrap();

        // The backend answered a request, so it was healthy.
        assert_eq!(service.unhealthy_backends, 0);
        let response = response.await.unwrap().unwrap();
        assert_eq!(response.payload.as_success().unwrap().get(), r#""0x1""#);
        // The subscription the dead backend created is not cancelled on the replacement.
        assert!(matches!(
            replacement_interface.from_frontend.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn duplicate_subscribe_response_is_unsubscribed() {
        let (handle, mut interface) = ConnectionHandle::new();
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService::new(handle, MockConnect::default(), reqs);
        let request =
            Request::new("eth_subscribe", Id::Number(1), ["newHeads"]).serialize().unwrap();
        let local_id = service.subs.insert(request, SubId::Number(U256::from(0xaaa)), 16);
        let mut sub = service.subs.get_subscription(local_id).unwrap();

        // An identical subscribe request raced the first one onto the wire.
        let raced = Request::new("eth_subscribe", Id::Number(2), ["newHeads"]).serialize().unwrap();
        let (in_flight, mut raced_response) = InFlight::new(raced, 16);
        service.in_flights.insert(in_flight);
        service
            .handle_item(
                serde_json::from_str(r#"{"jsonrpc":"2.0","id":2,"result":"0xbbb"}"#).unwrap(),
            )
            .unwrap();

        // The raced caller shares the existing subscription.
        let response = raced_response.try_recv().unwrap().unwrap();
        let alias: B256 =
            serde_json::from_str(response.payload.as_success().unwrap().get()).unwrap();
        assert_eq!(alias, local_id);

        // The second server-side subscription is cancelled instead of replacing the first.
        let unsubscribe = interface.from_frontend.try_recv().unwrap();
        let unsubscribe: serde_json::Value = serde_json::from_str(unsubscribe.get()).unwrap();
        assert_eq!(unsubscribe["method"], "eth_unsubscribe");
        assert_eq!(unsubscribe["id"], format!("unsubscribe-{local_id}"));
        assert_eq!(unsubscribe["params"], serde_json::json!(["0xbbb"]));

        service
            .handle_item(
                serde_json::from_str(
                    r#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"subscription":"0xaaa","result":1}}"#,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(sub.try_recv().unwrap().get(), "1");
    }

    #[test]
    fn unsubscribe_method_uses_subscribe_namespace() {
        for (subscribe, unsubscribe) in [
            ("eth_subscribe", "eth_unsubscribe"),
            ("debug_subscribe", "debug_unsubscribe"),
            ("admin_peerEvents", "admin_peerEvents_unsubscribe"),
            ("reth_subscribeChainNotifications", "reth_unsubscribeChainNotifications"),
            ("reth_subscribePersistedBlock", "reth_unsubscribePersistedBlock"),
            ("subscribe", "eth_unsubscribe"),
        ] {
            assert_eq!(unsubscribe_method(subscribe), unsubscribe, "{subscribe}");
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
        let service = PubSubService::new(dead_handle, connector, reqs);
        service.spawn();

        let first = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let expected_first = first.serialized().get().to_owned();
        let (in_flight, _rx) = InFlight::new(first, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        let replayed = timeout(Duration::from_secs(1), reconnected_interface.recv_from_frontend())
            .await
            .expect("raced request should be replayed after reconnect")
            .expect("new backend should receive the raced request");
        assert_eq!(replayed.get(), expected_first);

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
        let service = PubSubService::new(dead_handle, connector, reqs);
        service.spawn();

        let req = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (in_flight, rx) = InFlight::new(req, 16);
        tx.send(PubSubInstruction::Request(in_flight)).unwrap();

        let error = timeout(Duration::from_secs(1), rx)
            .await
            .expect("non-retryable reconnect should resolve promptly")
            .expect("raced request must receive the terminal error")
            .unwrap_err();
        assert!(error.as_transport_err().unwrap().is_non_retryable());
        assert_eq!(error.to_string(), "non-retryable test failure");

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
        let service = PubSubService::new(live_handle, connector, reqs);
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
    async fn terminal_error_reaches_outstanding_requests() {
        let (handle, mut interface) = ConnectionHandle::new();
        let (tx, reqs) = mpsc::unbounded_channel();
        let frontend = PubSubFrontend::new(tx);
        let service = PubSubService::new(handle, NonRetryableConnect::default(), reqs);
        service.spawn();

        let request = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let mut response = Box::pin(frontend.send(request));
        assert!(futures::poll!(&mut response).is_pending());
        timeout(Duration::from_secs(1), interface.recv_from_frontend()).await.unwrap().unwrap();
        interface.close_with_transport_error(TransportErrorKind::non_retryable_str(
            "authentication failed",
        ));

        let error = timeout(Duration::from_secs(1), response).await.unwrap().unwrap_err();
        assert!(error.as_transport_err().unwrap().is_non_retryable());
        assert_eq!(error.to_string(), "authentication failed");
    }

    #[tokio::test]
    async fn shutdown_error_reaches_tracked_and_queued_requests() {
        let (handle, _interface) = ConnectionHandle::new();
        let (tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService::new(handle, NonRetryableConnect::default(), reqs);
        let request = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (tracked, tracked_response) = InFlight::new(request, 16);
        service.in_flights.insert(tracked);
        let request = Request::new("eth_chainId", Id::Number(2), ()).serialize().unwrap();
        let (queued, queued_response) = InFlight::new(request, 16);
        tx.send(PubSubInstruction::Request(queued)).unwrap();

        service
            .fail_pending_requests(TransportError::ErrorResp(
                alloy_json_rpc::ErrorPayload::invalid_request(),
            ))
            .await;
        for response in [tracked_response, queued_response] {
            let error = response.await.unwrap().unwrap_err();
            assert_eq!(error.as_error_resp().unwrap().code, -32600);
        }
        assert!(tx.is_closed());
        assert_eq!(service.in_flights.len(), 0);
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
        let service = PubSubService::new(live_handle, connector, reqs);
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
