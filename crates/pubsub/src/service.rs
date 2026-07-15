use crate::{
    handle::ConnectionHandle,
    ix::PubSubInstruction,
    managers::{InFlight, RequestManager, SubscriptionManager},
    PubSubConnect, PubSubFrontend, RawSubscription, UnsubscribeOutcome,
};
use alloy_json_rpc::{
    Id, PubSubItem, Request, Response, ResponsePayload, RpcError, SerializedRequest, SubId,
};
use alloy_primitives::{Keccak256, B256};
use alloy_transport::{
    utils::{to_json_raw_value, Spawnable},
    TransportErrorKind, TransportResult,
};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap, time::Duration};
use tokio::sync::{mpsc, oneshot};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

const MAX_RECONNECT_RETRY_INTERVAL: Duration = Duration::from_secs(30);

type CleanupWaiter = oneshot::Sender<TransportResult<UnsubscribeOutcome>>;

fn subscription_local_id(request: &SerializedRequest) -> B256 {
    let method = request.method().as_bytes();
    let mut hasher = Keccak256::new();
    hasher.update((method.len() as u64).to_be_bytes());
    hasher.update(method);
    match request.params_with_presence() {
        Some(params) => {
            hasher.update([1]);
            hasher.update((params.get().len() as u64).to_be_bytes());
            hasher.update(params.get().as_bytes());
        }
        None => hasher.update([0]),
    }
    hasher.finalize()
}

#[derive(Debug)]
struct StartingSubscription {
    request: SerializedRequest,
    wire_request_id: Id,
    waiters: Vec<InFlight>,
    channel_size: usize,
    unsubscribe_method: Option<Cow<'static, str>>,
}

#[derive(Debug)]
struct SubscribeRoute {
    local_id: B256,
    unsubscribe_method: Option<Cow<'static, str>>,
    connection_epoch: u64,
    cleanup_waiters: Vec<CleanupWaiter>,
}

#[derive(Debug)]
struct PendingCleanup {
    server_id: SubId,
    unsubscribe_method: Cow<'static, str>,
    connection_epoch: u64,
    waiters: Vec<CleanupWaiter>,
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

    /// Subscription requests that have been dispatched but are not active yet.
    starting: BTreeMap<B256, StartingSubscription>,

    /// Every subscribe request ID remains routed until its connection closes so duplicate or late
    /// successful responses can be compensated with an unsubscribe.
    subscribe_routes: BTreeMap<Id, SubscribeRoute>,

    /// Active keys currently awaiting a resubscribe response.
    reconnecting: BTreeMap<B256, Id>,

    /// Tracked server-side unsubscribe requests.
    pending_cleanups: BTreeMap<Id, PendingCleanup>,

    /// Monotonically increasing connection generation.
    connection_epoch: u64,

    /// Monotonically increasing sequence for service-owned request IDs.
    request_sequence: u64,

    /// Waiters for subscriptions whose protocol provides no cleanup method.
    connection_cleanup_waiters: BTreeMap<u64, Vec<CleanupWaiter>>,
}

impl<T: PubSubConnect> PubSubService<T> {
    fn new(
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
            starting: BTreeMap::new(),
            subscribe_routes: BTreeMap::new(),
            reconnecting: BTreeMap::new(),
            pending_cleanups: BTreeMap::new(),
            connection_epoch: 0,
            request_sequence: 0,
            connection_cleanup_waiters: BTreeMap::new(),
        }
    }

    /// Create a new service from a connector.
    pub(crate) async fn connect(connector: T) -> TransportResult<PubSubFrontend> {
        let handle = connector.connect().await?;

        let (tx, reqs) = mpsc::unbounded_channel();
        let this = Self::new(handle, connector, reqs);
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

        let old_epoch = self.connection_epoch;
        let mut old_handle = self.get_new_backend().await?;

        debug!("Draining old backend to_handle");

        // Drain the old backend
        while let Ok(item) = old_handle.from_socket.try_recv() {
            self.handle_item(item)?;
        }

        old_handle.shutdown();
        self.finish_connection_epoch(old_epoch);
        self.connection_epoch =
            self.connection_epoch.checked_add(1).expect("connection epoch exhausted");

        // Re-issue pending requests.
        debug!(count = self.in_flights.len(), "Reissuing pending requests");
        let pending_requests = self
            .in_flights
            .iter()
            .map(|(_, in_flight)| in_flight.request.serialized().to_owned())
            .collect::<Vec<_>>();
        for msg in pending_requests {
            self.dispatch_request(msg)?;
        }

        // Re-issue live single-flight subscriptions. Fully cancelled requests are discarded: the
        // old connection closing has already reclaimed any server-side resource they created.
        let starting_local_ids = self.starting.keys().copied().collect::<Vec<_>>();
        for local_id in starting_local_ids {
            let has_live_waiter = self.starting.get_mut(&local_id).is_some_and(|starting| {
                starting.waiters.retain(|waiter| !waiter.tx.is_closed());
                !starting.waiters.is_empty()
            });
            if !has_live_waiter {
                self.starting.remove(&local_id);
                continue;
            }

            let wire_request_id = self.next_service_id();
            let (request, unsubscribe_method) = {
                let starting = self.starting.get_mut(&local_id).expect("checked above");
                starting.wire_request_id = wire_request_id.clone();
                (
                    starting.request.with_id(wire_request_id.clone()).map_err(RpcError::ser_err)?,
                    starting.unsubscribe_method.clone(),
                )
            };
            self.insert_subscribe_route(wire_request_id, local_id, unsubscribe_method);
            self.dispatch_request(request.into_serialized())?;
        }

        // Re-subscribe to all active subscriptions
        debug!(count = self.subs.len(), "Re-starting active subscriptions");

        // Drop all server IDs. We'll re-insert them as we get responses.
        self.subs.drop_server_ids();
        self.reconnecting.clear();

        // Dispatch all subscription requests.
        let active = self
            .subs
            .iter()
            .map(|(&local_id, sub)| {
                (local_id, sub.request().clone(), sub.unsubscribe_method.clone())
            })
            .collect::<Vec<_>>();
        for (local_id, request, unsubscribe_method) in active {
            let wire_request_id = self.next_service_id();
            let request = request.with_id(wire_request_id.clone()).map_err(RpcError::ser_err)?;
            self.reconnecting.insert(local_id, wire_request_id.clone());
            self.insert_subscribe_route(wire_request_id, local_id, unsubscribe_method);
            self.dispatch_request(request.into_serialized())?;
        }

        Ok(())
    }

    /// Dispatch a request to the socket.
    fn dispatch_request(&self, brv: Box<RawValue>) -> TransportResult<()> {
        self.handle.to_socket.send(brv).map(drop).map_err(|_| TransportErrorKind::backend_gone())
    }

    fn next_service_id(&mut self) -> Id {
        let sequence = self.request_sequence;
        self.request_sequence =
            self.request_sequence.checked_add(1).expect("service request ID sequence exhausted");
        Id::String(format!("alloy-pubsub:{}:{sequence}", self.connection_epoch))
    }

    fn insert_subscribe_route(
        &mut self,
        request_id: Id,
        local_id: B256,
        unsubscribe_method: Option<Cow<'static, str>>,
    ) {
        let route = SubscribeRoute {
            local_id,
            unsubscribe_method,
            connection_epoch: self.connection_epoch,
            cleanup_waiters: Vec::new(),
        };
        let previous = self.subscribe_routes.insert(request_id, route);
        debug_assert!(previous.is_none(), "service request IDs must never be reused");
    }

    fn finish_connection_epoch(&mut self, epoch: u64) {
        let cleanup_ids = self
            .pending_cleanups
            .iter()
            .filter(|(_, cleanup)| cleanup.connection_epoch == epoch)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in cleanup_ids {
            if let Some(cleanup) = self.pending_cleanups.remove(&id) {
                debug!(?cleanup.server_id, method=%cleanup.unsubscribe_method, "cleanup completed by transport close");
                Self::complete_cleanup_waiters(
                    cleanup.waiters,
                    UnsubscribeOutcome::TransportClosed,
                );
            }
        }

        let route_ids = self
            .subscribe_routes
            .iter()
            .filter(|(_, route)| route.connection_epoch == epoch)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in route_ids {
            if let Some(route) = self.subscribe_routes.remove(&id) {
                Self::complete_cleanup_waiters(
                    route.cleanup_waiters,
                    UnsubscribeOutcome::TransportClosed,
                );
            }
        }
        if let Some(waiters) = self.connection_cleanup_waiters.remove(&epoch) {
            Self::complete_cleanup_waiters(waiters, UnsubscribeOutcome::TransportClosed);
        }
    }

    /// Service a request.
    fn service_request(&mut self, in_flight: InFlight) -> TransportResult<()> {
        if !in_flight.is_subscription() {
            let brv = in_flight.request();

            self.dispatch_request(brv.serialized().to_owned())?;
            self.in_flights.insert(in_flight);
            return Ok(());
        }

        if in_flight.tx.is_closed() {
            return Ok(());
        }

        if in_flight.channel_size == 0 {
            let _ = in_flight
                .tx
                .send(Err(RpcError::local_usage_str("subscription channel size must be non-zero")));
            return Ok(());
        }

        let local_id = subscription_local_id(in_flight.request());
        let unsubscribe_method = Self::unsubscribe_method(&in_flight);

        if let Some(starting) = self.starting.get_mut(&local_id) {
            Self::warn_config_conflict(
                local_id,
                starting.channel_size,
                starting.unsubscribe_method.as_deref(),
                in_flight.channel_size,
                unsubscribe_method.as_deref(),
            );
            starting.waiters.push(in_flight);
            return Ok(());
        }

        if let Some(active) = self.subs.get(&local_id) {
            Self::warn_config_conflict(
                local_id,
                active.channel_size,
                active.unsubscribe_method.as_deref(),
                in_flight.channel_size,
                unsubscribe_method.as_deref(),
            );
            return Self::send_subscription_alias(in_flight, active.local_id);
        }

        let wire_request_id = in_flight.request.id().clone();
        let request = in_flight.request.clone();
        let message = request.serialized().to_owned();
        let channel_size = in_flight.channel_size;
        self.starting.insert(
            local_id,
            StartingSubscription {
                request,
                wire_request_id: wire_request_id.clone(),
                waiters: vec![in_flight],
                channel_size,
                unsubscribe_method: unsubscribe_method.clone(),
            },
        );
        self.insert_subscribe_route(wire_request_id, local_id, unsubscribe_method);
        self.dispatch_request(message)?;

        Ok(())
    }

    fn unsubscribe_method(in_flight: &InFlight) -> Option<Cow<'static, str>> {
        if let Some(method) = &in_flight.unsubscribe_method {
            return Some(method.clone());
        }
        if in_flight.request.method() == "eth_subscribe" {
            return Some(Cow::Borrowed("eth_unsubscribe"));
        }
        warn!(
            method = in_flight.request.method(),
            "custom subscription has no cleanup method; it can only be reclaimed by closing the connection"
        );
        None
    }

    fn warn_config_conflict(
        local_id: B256,
        existing_channel_size: usize,
        existing_unsubscribe_method: Option<&str>,
        requested_channel_size: usize,
        requested_unsubscribe_method: Option<&str>,
    ) {
        if existing_channel_size != requested_channel_size {
            warn!(
                ?local_id,
                existing_channel_size,
                requested_channel_size,
                "subscription already exists; keeping its channel size"
            );
        }
        if existing_unsubscribe_method != requested_unsubscribe_method {
            warn!(
                ?local_id,
                existing_unsubscribe_method,
                requested_unsubscribe_method,
                "subscription already exists; keeping its unsubscribe method"
            );
        }
    }

    fn send_subscription_alias(in_flight: InFlight, local_id: B256) -> TransportResult<()> {
        let id = in_flight.request.id().clone();
        let alias = to_json_raw_value(&local_id)?;
        let _ = in_flight.tx.send(Ok(Response { id, payload: ResponsePayload::Success(alias) }));

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
    fn service_unsubscribe(
        &mut self,
        local_id: B256,
        waiter: Option<CleanupWaiter>,
    ) -> TransportResult<()> {
        let mut waiters = waiter.into_iter().collect::<Vec<_>>();
        if let Some((subscription, server_id)) = self.subs.remove_sub(local_id) {
            if let Some(server_id) = server_id {
                if let Some(unsubscribe_method) = subscription.unsubscribe_method {
                    return self.start_cleanup(server_id, unsubscribe_method, waiters);
                }
                warn!(?server_id, "subscription has no cleanup method; deferring reclamation until connection close");
                self.defer_cleanup_until_connection_close(waiters);
                return Ok(());
            }

            if let Some(route_id) = self.reconnecting.remove(&subscription.local_id) {
                if let Some(route) = self.subscribe_routes.get_mut(&route_id) {
                    route.cleanup_waiters.append(&mut waiters);
                    return Ok(());
                }
            }

            Self::complete_cleanup_waiters(waiters, UnsubscribeOutcome::TransportClosed);
            return Ok(());
        }

        if let Some(starting) = self.starting.remove(&local_id) {
            for in_flight in starting.waiters {
                let _ = in_flight.tx.send(Err(RpcError::local_usage_str(
                    "subscription was unsubscribed before activation",
                )));
            }
            if let Some(route) = self.subscribe_routes.get_mut(&starting.wire_request_id) {
                route.cleanup_waiters.append(&mut waiters);
                return Ok(());
            }
        }

        Self::complete_cleanup_waiters(waiters, UnsubscribeOutcome::AlreadyAbsent);
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
            PubSubInstruction::Unsubscribe(alias) => self.service_unsubscribe(alias, None),
            PubSubInstruction::UnsubscribeAndWait(alias, tx) => {
                self.service_unsubscribe(alias, Some(tx))
            }
        }
    }

    /// Handle an item from the backend.
    fn handle_item(&mut self, item: PubSubItem) -> TransportResult<()> {
        match item {
            PubSubItem::Response(resp) => {
                if self.pending_cleanups.contains_key(&resp.id) {
                    self.handle_cleanup_response(resp)
                } else if self.subscribe_routes.contains_key(&resp.id) {
                    self.handle_sub_response(resp)
                } else {
                    let _ = self.in_flights.handle_response(resp);
                    Ok(())
                }
            }
            PubSubItem::Notification(notification) => {
                self.subs.notify(notification);
                Ok(())
            }
        }
    }

    /// Route a subscribe or resubscribe response without ever overwriting an active server ID.
    fn handle_sub_response(&mut self, response: Response) -> TransportResult<()> {
        let route = self.subscribe_routes.get(&response.id).expect("checked by caller");
        let local_id = route.local_id;
        let unsubscribe_method = route.unsubscribe_method.clone();

        match response.payload {
            ResponsePayload::Success(value) => match serde_json::from_str::<SubId>(value.get()) {
                Ok(server_id) => {
                    self.handle_sub_success(response.id, local_id, unsubscribe_method, server_id)
                }
                Err(error) => {
                    self.handle_invalid_sub_response(response.id, local_id, value.get(), error);
                    Ok(())
                }
            },
            ResponsePayload::Failure(error) => {
                self.handle_sub_failure(response.id, local_id, error);
                Ok(())
            }
        }
    }

    fn handle_sub_success(
        &mut self,
        response_id: Id,
        local_id: B256,
        unsubscribe_method: Option<Cow<'static, str>>,
        server_id: SubId,
    ) -> TransportResult<()> {
        let is_expected_start = self
            .starting
            .get(&local_id)
            .is_some_and(|starting| starting.wire_request_id == response_id);
        if is_expected_start {
            let mut starting = self.starting.remove(&local_id).expect("checked above");
            starting.waiters.retain(|waiter| !waiter.tx.is_closed());
            if starting.waiters.is_empty() || self.subs.get(&local_id).is_some() {
                return self.compensate_subscription(response_id, server_id, unsubscribe_method);
            }
            if self.subs.contains_server_id(&server_id) {
                warn!(?server_id, ?local_id, "subscription response reused a live server id");
                for in_flight in starting.waiters {
                    let _ = in_flight.tx.send(Err(RpcError::local_usage_str(
                        "subscription response reused a live server id",
                    )));
                }
                return Ok(());
            }

            self.subs.insert(
                local_id,
                starting.request,
                server_id.clone(),
                starting.channel_size,
                starting.unsubscribe_method,
            );
            let mut delivered = false;
            for in_flight in starting.waiters {
                let id = in_flight.request.id().clone();
                let alias = to_json_raw_value(&local_id)?;
                delivered |= in_flight
                    .tx
                    .send(Ok(Response { id, payload: ResponsePayload::Success(alias) }))
                    .is_ok();
            }
            if !delivered {
                if let Some((subscription, _)) = self.subs.remove_sub(local_id) {
                    if let Some(unsubscribe_method) = subscription.unsubscribe_method {
                        self.start_cleanup(server_id, unsubscribe_method, Vec::new())?;
                    } else {
                        warn!(?server_id, "abandoned subscription has no cleanup method; it will remain until connection close");
                    }
                }
            }
            return Ok(());
        }

        let is_expected_reconnect =
            self.reconnecting.get(&local_id).is_some_and(|request_id| request_id == &response_id);
        if is_expected_reconnect {
            self.reconnecting.remove(&local_id);
            if self.subs.set_server_id(&local_id, server_id.clone()) {
                return Ok(());
            }
            self.subs.remove_sub(local_id);
        }

        self.compensate_subscription(response_id, server_id, unsubscribe_method)
    }

    fn handle_invalid_sub_response(
        &mut self,
        response_id: Id,
        local_id: B256,
        value: &str,
        error: serde_json::Error,
    ) {
        warn!(?local_id, %error, "invalid subscription response");
        let is_expected_start = self
            .starting
            .get(&local_id)
            .is_some_and(|starting| starting.wire_request_id == response_id);
        if is_expected_start {
            if let Some(starting) = self.starting.remove(&local_id) {
                for in_flight in starting.waiters {
                    let error = serde_json::from_str::<SubId>(value).unwrap_err();
                    let _ = in_flight
                        .tx
                        .send(Err(alloy_transport::TransportError::deser_err(error, value)));
                }
            }
        } else if self
            .reconnecting
            .get(&local_id)
            .is_some_and(|request_id| request_id == &response_id)
        {
            self.reconnecting.remove(&local_id);
            self.subs.remove_sub(local_id);
        }

        if let Some(route) = self.subscribe_routes.get_mut(&response_id) {
            for waiter in std::mem::take(&mut route.cleanup_waiters) {
                let error = serde_json::from_str::<SubId>(value).unwrap_err();
                let _ = waiter.send(Err(alloy_transport::TransportError::deser_err(error, value)));
            }
        }
    }

    fn handle_sub_failure(
        &mut self,
        response_id: Id,
        local_id: B256,
        error: alloy_json_rpc::ErrorPayload,
    ) {
        let is_expected_start = self
            .starting
            .get(&local_id)
            .is_some_and(|starting| starting.wire_request_id == response_id);
        if is_expected_start {
            if let Some(starting) = self.starting.remove(&local_id) {
                for in_flight in starting.waiters {
                    let id = in_flight.request.id().clone();
                    let _ = in_flight.tx.send(Ok(Response {
                        id,
                        payload: ResponsePayload::Failure(error.clone()),
                    }));
                }
            }
        } else if self
            .reconnecting
            .get(&local_id)
            .is_some_and(|request_id| request_id == &response_id)
        {
            self.reconnecting.remove(&local_id);
            self.subs.remove_sub(local_id);
            warn!(?local_id, %error, "failed to restore subscription after reconnect");
        }

        if let Some(route) = self.subscribe_routes.get_mut(&response_id) {
            Self::complete_cleanup_waiters(
                std::mem::take(&mut route.cleanup_waiters),
                UnsubscribeOutcome::AlreadyAbsent,
            );
        }
    }

    fn compensate_subscription(
        &mut self,
        response_id: Id,
        server_id: SubId,
        unsubscribe_method: Option<Cow<'static, str>>,
    ) -> TransportResult<()> {
        if self.subs.contains_server_id(&server_id) {
            let waiters = self
                .subscribe_routes
                .get_mut(&response_id)
                .map(|route| std::mem::take(&mut route.cleanup_waiters))
                .unwrap_or_default();
            warn!(?server_id, "refusing to clean up a server id held by a live subscription");
            for waiter in waiters {
                let _ = waiter.send(Err(RpcError::local_usage_str(
                    "stale cleanup targeted a live subscription",
                )));
            }
            return Ok(());
        }
        let Some(unsubscribe_method) = unsubscribe_method else {
            warn!(?server_id, "unclaimed subscription has no cleanup method; it will remain until connection close");
            return Ok(());
        };
        let waiters = self
            .subscribe_routes
            .get_mut(&response_id)
            .map(|route| std::mem::take(&mut route.cleanup_waiters))
            .unwrap_or_default();
        warn!(?server_id, "cleaning up an unclaimed subscription response");
        self.start_cleanup(server_id, unsubscribe_method, waiters)
    }

    fn defer_cleanup_until_connection_close(&mut self, waiters: Vec<CleanupWaiter>) {
        if !waiters.is_empty() {
            self.connection_cleanup_waiters
                .entry(self.connection_epoch)
                .or_default()
                .extend(waiters);
        }
    }

    fn start_cleanup(
        &mut self,
        server_id: SubId,
        unsubscribe_method: Cow<'static, str>,
        waiters: Vec<CleanupWaiter>,
    ) -> TransportResult<()> {
        if let Some((_, cleanup)) =
            self.pending_cleanups.iter_mut().find(|(_, cleanup)| cleanup.server_id == server_id)
        {
            cleanup.waiters.extend(waiters);
            return Ok(());
        }

        let request_id = self.next_service_id();
        let request =
            Request::new(unsubscribe_method.clone(), request_id.clone(), [server_id.clone()])
                .serialize()
                .map_err(RpcError::ser_err)?;
        self.pending_cleanups.insert(
            request_id,
            PendingCleanup {
                server_id,
                unsubscribe_method,
                connection_epoch: self.connection_epoch,
                waiters,
            },
        );
        self.dispatch_request(request.into_serialized())
    }

    fn handle_cleanup_response(&mut self, response: Response) -> TransportResult<()> {
        let cleanup = self.pending_cleanups.remove(&response.id).expect("checked by caller");
        match response.payload {
            ResponsePayload::Success(value) => match serde_json::from_str::<bool>(value.get()) {
                Ok(true) => {
                    trace!(?cleanup.server_id, method=%cleanup.unsubscribe_method, "subscription cleanup confirmed");
                    Self::complete_cleanup_waiters(
                        cleanup.waiters,
                        UnsubscribeOutcome::ServerConfirmed,
                    );
                }
                Ok(false) => {
                    info!(?cleanup.server_id, method=%cleanup.unsubscribe_method, "subscription was already absent");
                    Self::complete_cleanup_waiters(
                        cleanup.waiters,
                        UnsubscribeOutcome::AlreadyAbsent,
                    );
                }
                Err(error) => {
                    warn!(?cleanup.server_id, method=%cleanup.unsubscribe_method, %error, "invalid subscription cleanup response");
                    for waiter in cleanup.waiters {
                        let error = serde_json::from_str::<bool>(value.get()).unwrap_err();
                        let _ = waiter.send(Err(alloy_transport::TransportError::deser_err(
                            error,
                            value.get(),
                        )));
                    }
                }
            },
            ResponsePayload::Failure(error) => {
                warn!(?cleanup.server_id, method=%cleanup.unsubscribe_method, %error, "subscription cleanup failed");
                for waiter in cleanup.waiters {
                    let _ = waiter.send(Err(RpcError::ErrorResp(error.clone())));
                }
            }
        }
        Ok(())
    }

    fn complete_cleanup_waiters(waiters: Vec<CleanupWaiter>, outcome: UnsubscribeOutcome) {
        for waiter in waiters {
            let _ = waiter.send(Ok(outcome));
        }
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

            let epoch = self.connection_epoch;
            self.finish_connection_epoch(epoch);
            if let Err(err) = result {
                error!(%err, "pubsub service reconnection error");
            }
        };
        fut.spawn_task();
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
    use crate::{ConnectionInterface, SubscriptionOptions};
    use alloy_json_rpc::{Request, SerializedRequest};
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

    fn test_service() -> (PubSubService<MockConnect>, ConnectionInterface) {
        let (handle, interface) = ConnectionHandle::new();
        let (_tx, reqs) = mpsc::unbounded_channel();
        (PubSubService::new(handle, MockConnect::default(), reqs), interface)
    }

    fn eth_subscription(id: u64, topic: &'static str) -> SerializedRequest {
        Request::new("eth_subscribe", Id::Number(id), (topic,)).serialize().unwrap()
    }

    fn custom_subscription(
        id: u64,
        method: &'static str,
        unsubscribe_method: Option<&'static str>,
    ) -> SerializedRequest {
        let mut request = Request::new(method, Id::Number(id), ());
        request.set_is_subscription();
        if let Some(unsubscribe_method) = unsubscribe_method {
            request
                .meta
                .extensions_mut()
                .get_or_insert_default::<SubscriptionOptions>()
                .set_unsubscribe_method(unsubscribe_method);
        }
        request.serialize().unwrap()
    }

    fn with_channel_size(mut request: SerializedRequest, channel_size: usize) -> SerializedRequest {
        request
            .meta_mut()
            .extensions_mut()
            .get_or_insert_default::<SubscriptionOptions>()
            .set_channel_size(channel_size);
        request
    }

    fn subscription_response(id: Id, server_id: &str) -> Response {
        Response {
            id,
            payload: ResponsePayload::Success(
                to_json_raw_value(&SubId::String(server_id.to_owned())).unwrap(),
            ),
        }
    }

    fn bool_response(id: Id, value: bool) -> Response {
        Response { id, payload: ResponsePayload::Success(to_json_raw_value(&value).unwrap()) }
    }

    fn take_wire(interface: &mut ConnectionInterface) -> serde_json::Value {
        let request = interface.from_frontend.try_recv().expect("expected outbound request");
        serde_json::from_str(request.get()).unwrap()
    }

    fn assert_no_wire(interface: &mut ConnectionInterface) {
        assert!(matches!(
            interface.from_frontend.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    fn alias_from_response(response: Response, expected_id: Id) -> B256 {
        assert_eq!(response.id, expected_id);
        match response.payload {
            ResponsePayload::Success(value) => serde_json::from_str(value.get()).unwrap(),
            ResponsePayload::Failure(error) => panic!("unexpected error response: {error}"),
        }
    }

    fn activate(
        service: &mut PubSubService<MockConnect>,
        interface: &mut ConnectionInterface,
        request: SerializedRequest,
        server_id: &str,
    ) -> B256 {
        let request_id = request.id().clone();
        let (in_flight, mut rx) = InFlight::new(request, 16);
        service.service_request(in_flight).unwrap();
        let _ = take_wire(interface);
        service.handle_item(subscription_response(request_id.clone(), server_id).into()).unwrap();
        let response = rx.try_recv().unwrap().unwrap();
        alias_from_response(response, request_id)
    }

    #[test]
    fn subscription_local_id_includes_the_method() {
        let admin = Request::new("admin_peerEvents", Id::Number(1), ()).serialize().unwrap();
        let reth = Request::new("reth_subscribeChainNotifications", Id::Number(2), ())
            .serialize()
            .unwrap();

        assert_ne!(subscription_local_id(&admin), subscription_local_id(&reth));
    }

    #[test]
    fn subscription_local_id_distinguishes_params_presence_and_value() {
        let omitted = Request::new("custom_subscribe", Id::Number(1), ()).serialize().unwrap();
        let null = Request::new("custom_subscribe", Id::Number(2), serde_json::Value::Null)
            .serialize()
            .unwrap();
        let empty =
            Request::new("custom_subscribe", Id::Number(3), Vec::<u8>::new()).serialize().unwrap();

        let omitted = subscription_local_id(&omitted);
        let null = subscription_local_id(&null);
        let empty = subscription_local_id(&empty);
        assert_ne!(omitted, null);
        assert_ne!(omitted, empty);
        assert_ne!(null, empty);
    }

    #[test]
    fn identical_subscriptions_share_one_wire_request_and_preserve_waiter_ids() {
        let (mut service, mut interface) = test_service();
        let (first, mut first_rx) = InFlight::new(eth_subscription(1, "newHeads"), 16);
        let (second, mut second_rx) = InFlight::new(eth_subscription(2, "newHeads"), 16);

        service.service_request(first).unwrap();
        let wire = take_wire(&mut interface);
        assert_eq!(wire["id"], 1);
        service.service_request(second).unwrap();
        assert_no_wire(&mut interface);

        service.handle_item(subscription_response(Id::Number(1), "server-1").into()).unwrap();
        let first_alias = alias_from_response(first_rx.try_recv().unwrap().unwrap(), Id::Number(1));
        let second_alias =
            alias_from_response(second_rx.try_recv().unwrap().unwrap(), Id::Number(2));
        assert_eq!(first_alias, second_alias);
        assert_eq!(service.subs.len(), 1);

        let first_local = service.subs.get_subscription(first_alias).unwrap();
        let second_local = service.subs.get_subscription(second_alias).unwrap();
        assert_eq!(first_local.local_id, second_local.local_id);

        let (third, mut third_rx) = InFlight::new(eth_subscription(3, "newHeads"), 16);
        service.service_request(third).unwrap();
        assert_no_wire(&mut interface);
        assert_eq!(
            alias_from_response(third_rx.try_recv().unwrap().unwrap(), Id::Number(3)),
            first_alias
        );
    }

    #[test]
    fn different_methods_with_omitted_params_do_not_collide() {
        let (mut service, mut interface) = test_service();
        let (admin, _admin_rx) =
            InFlight::new(custom_subscription(1, "admin_peerEvents", None), 16);
        let (reth, _reth_rx) =
            InFlight::new(custom_subscription(2, "reth_subscribeChainNotifications", None), 16);

        service.service_request(admin).unwrap();
        service.service_request(reth).unwrap();
        assert_eq!(take_wire(&mut interface)["method"], "admin_peerEvents");
        assert_eq!(take_wire(&mut interface)["method"], "reth_subscribeChainNotifications");
        assert_eq!(service.starting.len(), 2);
    }

    #[test]
    fn cancelled_single_flight_is_compensated_after_success() {
        let (mut service, mut interface) = test_service();
        let (in_flight, rx) = InFlight::new(eth_subscription(1, "logs"), 16);
        service.service_request(in_flight).unwrap();
        let _ = take_wire(&mut interface);
        drop(rx);

        service.handle_item(subscription_response(Id::Number(1), "abandoned").into()).unwrap();
        assert_eq!(service.subs.len(), 0);
        let cleanup = take_wire(&mut interface);
        assert_eq!(cleanup["method"], "eth_unsubscribe");
        assert_eq!(cleanup["params"], serde_json::json!(["abandoned"]));
    }

    #[test]
    fn cancelling_first_waiter_does_not_cancel_other_waiters() {
        let (mut service, mut interface) = test_service();
        let (first, first_rx) = InFlight::new(eth_subscription(1, "logs"), 16);
        let (second, mut second_rx) = InFlight::new(eth_subscription(2, "logs"), 16);
        service.service_request(first).unwrap();
        let _ = take_wire(&mut interface);
        service.service_request(second).unwrap();
        drop(first_rx);

        service.handle_item(subscription_response(Id::Number(1), "shared").into()).unwrap();
        let _ = alias_from_response(second_rx.try_recv().unwrap().unwrap(), Id::Number(2));
        assert_eq!(service.subs.len(), 1);
        assert_no_wire(&mut interface);
    }

    #[test]
    fn new_waiter_joins_existing_single_flight_after_earlier_waiters_cancel() {
        let (mut service, mut interface) = test_service();
        let (first, first_rx) = InFlight::new(eth_subscription(1, "logs"), 16);
        service.service_request(first).unwrap();
        let _ = take_wire(&mut interface);
        drop(first_rx);

        let (second, mut second_rx) = InFlight::new(eth_subscription(2, "logs"), 16);
        service.service_request(second).unwrap();
        assert_no_wire(&mut interface);

        service.handle_item(subscription_response(Id::Number(1), "shared").into()).unwrap();
        let _ = alias_from_response(second_rx.try_recv().unwrap().unwrap(), Id::Number(2));
        assert_eq!(service.subs.len(), 1);
        assert_no_wire(&mut interface);
    }

    #[test]
    fn cancelled_before_service_dispatch_has_no_side_effect() {
        let (mut service, mut interface) = test_service();
        let (in_flight, rx) = InFlight::new(eth_subscription(1, "logs"), 16);
        drop(rx);

        service.service_request(in_flight).unwrap();
        assert!(service.starting.is_empty());
        assert_no_wire(&mut interface);
    }

    #[test]
    fn subscription_error_is_fanned_out_with_each_waiter_id() {
        let (mut service, mut interface) = test_service();
        let (first, mut first_rx) = InFlight::new(eth_subscription(1, "logs"), 16);
        let (second, mut second_rx) = InFlight::new(eth_subscription(2, "logs"), 16);
        service.service_request(first).unwrap();
        let _ = take_wire(&mut interface);
        service.service_request(second).unwrap();

        service.handle_item(Response::internal_error(Id::Number(1)).into()).unwrap();
        let first = first_rx.try_recv().unwrap().unwrap();
        let second = second_rx.try_recv().unwrap().unwrap();
        assert_eq!(first.id, Id::Number(1));
        assert_eq!(second.id, Id::Number(2));
        assert!(first.is_error());
        assert!(second.is_error());
        assert!(service.starting.is_empty());
        assert_eq!(service.subs.len(), 0);
    }

    #[test]
    fn different_params_create_independent_upstream_subscriptions() {
        let (mut service, mut interface) = test_service();
        let (heads, _heads_rx) = InFlight::new(eth_subscription(1, "newHeads"), 16);
        let (logs, _logs_rx) = InFlight::new(eth_subscription(2, "logs"), 16);

        service.service_request(heads).unwrap();
        service.service_request(logs).unwrap();

        assert_eq!(take_wire(&mut interface)["params"], serde_json::json!(["newHeads"]));
        assert_eq!(take_wire(&mut interface)["params"], serde_json::json!(["logs"]));
        assert_eq!(service.starting.len(), 2);
    }

    #[test]
    fn duplicate_server_response_is_cleaned_without_overwriting_active_id() {
        let (mut service, mut interface) = test_service();
        let alias =
            activate(&mut service, &mut interface, eth_subscription(1, "newHeads"), "active");

        service.handle_item(subscription_response(Id::Number(1), "duplicate").into()).unwrap();
        let cleanup = take_wire(&mut interface);
        assert_eq!(cleanup["params"], serde_json::json!(["duplicate"]));
        assert_eq!(
            service.subs.server_id_for_local_id(&alias),
            Some(&SubId::String("active".into()))
        );

        service.handle_item(subscription_response(Id::Number(1), "active").into()).unwrap();
        assert_no_wire(&mut interface);
    }

    #[test]
    fn per_request_channel_size_is_first_creator_wins_and_zero_is_rejected() {
        let (mut service, mut interface) = test_service();
        let first_request = with_channel_size(eth_subscription(1, "newHeads"), 7);
        let second_request = with_channel_size(eth_subscription(2, "newHeads"), 11);
        let local_id = subscription_local_id(&first_request);
        let (first, mut first_rx) = InFlight::new(first_request, 16);
        let (second, mut second_rx) = InFlight::new(second_request, 16);
        service.service_request(first).unwrap();
        let _ = take_wire(&mut interface);
        service.service_request(second).unwrap();
        assert_no_wire(&mut interface);
        service.handle_item(subscription_response(Id::Number(1), "sized").into()).unwrap();
        first_rx.try_recv().unwrap().unwrap();
        second_rx.try_recv().unwrap().unwrap();
        assert_eq!(service.subs.get(&local_id).unwrap().channel_size, 7);

        let active_request = with_channel_size(eth_subscription(3, "newHeads"), 19);
        let (active, mut active_rx) = InFlight::new(active_request, 16);
        service.service_request(active).unwrap();
        assert_no_wire(&mut interface);
        active_rx.try_recv().unwrap().unwrap();
        assert_eq!(service.subs.get(&local_id).unwrap().channel_size, 7);

        let independent_request = with_channel_size(eth_subscription(4, "logs"), 13);
        let independent_local_id = subscription_local_id(&independent_request);
        let _ = activate(&mut service, &mut interface, independent_request, "independent");
        assert_eq!(service.subs.get(&independent_local_id).unwrap().channel_size, 13);
        assert_eq!(service.subs.get(&local_id).unwrap().channel_size, 7);

        let zero = with_channel_size(eth_subscription(5, "newPendingTransactions"), 0);
        let (zero, mut zero_rx) = InFlight::new(zero, 16);
        service.service_request(zero).unwrap();
        assert!(zero_rx.try_recv().unwrap().unwrap_err().is_local_usage_error());
        assert_no_wire(&mut interface);
    }

    #[test]
    fn tracked_cleanup_uses_reserved_id_and_waits_for_ack() {
        let (mut service, mut interface) = test_service();
        let alias =
            activate(&mut service, &mut interface, eth_subscription(1, "newHeads"), "active");
        let (tx, mut rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        let cleanup = take_wire(&mut interface);
        assert_eq!(cleanup["method"], "eth_unsubscribe");
        let cleanup_id = cleanup["id"].as_str().unwrap().to_owned();
        assert!(cleanup_id.starts_with("alloy-pubsub:0:"));
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Empty)));

        service.handle_item(bool_response(Id::String(cleanup_id.clone()), true).into()).unwrap();
        assert_eq!(rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::ServerConfirmed);

        let live = activate(&mut service, &mut interface, eth_subscription(2, "logs"), "live");
        service.handle_item(bool_response(Id::String(cleanup_id), true).into()).unwrap();
        assert!(service.subs.get(&live).is_some());
        assert_no_wire(&mut interface);
    }

    #[test]
    fn concurrent_cleanups_use_distinct_service_request_ids() {
        let (mut service, mut interface) = test_service();
        let first =
            activate(&mut service, &mut interface, eth_subscription(1, "newHeads"), "first");
        let second = activate(&mut service, &mut interface, eth_subscription(2, "logs"), "second");

        service.service_unsubscribe(first, None).unwrap();
        service.service_unsubscribe(second, None).unwrap();
        let first_cleanup = take_wire(&mut interface);
        let second_cleanup = take_wire(&mut interface);

        assert_ne!(first_cleanup["id"], second_cleanup["id"]);
        assert!(first_cleanup["id"].as_str().unwrap().starts_with("alloy-pubsub:0:"));
        assert!(second_cleanup["id"].as_str().unwrap().starts_with("alloy-pubsub:0:"));
    }

    #[test]
    fn cleanup_response_cannot_hijack_numeric_in_flight_request() {
        let (mut service, mut interface) = test_service();
        let normal = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
        let (normal, mut normal_rx) = InFlight::new(normal, 16);
        service.service_request(normal).unwrap();
        let _ = take_wire(&mut interface);

        let alias =
            activate(&mut service, &mut interface, eth_subscription(2, "newHeads"), "active");
        let (tx, mut cleanup_rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        let cleanup = take_wire(&mut interface);
        let cleanup_id = Id::String(cleanup["id"].as_str().unwrap().to_owned());
        service.handle_item(bool_response(cleanup_id, true).into()).unwrap();
        assert_eq!(cleanup_rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::ServerConfirmed);
        assert!(matches!(normal_rx.try_recv(), Err(oneshot::error::TryRecvError::Empty)));

        let normal_response = Response {
            id: Id::Number(1),
            payload: ResponsePayload::Success(to_json_raw_value(&"0x1").unwrap()),
        };
        service.handle_item(normal_response.into()).unwrap();
        assert_eq!(normal_rx.try_recv().unwrap().unwrap().id, Id::Number(1));
    }

    #[test]
    fn tracked_cleanup_reports_false_and_rpc_error_without_retry() {
        let (mut service, mut interface) = test_service();
        let alias =
            activate(&mut service, &mut interface, eth_subscription(1, "newHeads"), "first");
        let (tx, mut rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        let cleanup = take_wire(&mut interface);
        let cleanup_id = Id::String(cleanup["id"].as_str().unwrap().to_owned());
        service.handle_item(bool_response(cleanup_id, false).into()).unwrap();
        assert_eq!(rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::AlreadyAbsent);

        let alias = activate(&mut service, &mut interface, eth_subscription(2, "logs"), "second");
        let (tx, mut rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        let cleanup = take_wire(&mut interface);
        let cleanup_id = Id::String(cleanup["id"].as_str().unwrap().to_owned());
        service.handle_item(Response::internal_error(cleanup_id).into()).unwrap();
        assert!(rx.try_recv().unwrap().unwrap_err().is_error_resp());
        assert_no_wire(&mut interface);
    }

    #[tokio::test]
    async fn pending_cleanup_completes_on_close_and_is_not_replayed() {
        let (old_handle, mut old_interface) = ConnectionHandle::new();
        let (new_handle, mut new_interface) = ConnectionHandle::new();
        let connector = MockConnect(Arc::new(Mutex::new(Some(new_handle))));
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService::new(old_handle, connector, reqs);
        let alias =
            activate(&mut service, &mut old_interface, eth_subscription(1, "newHeads"), "active");
        let (tx, mut rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        let _ = take_wire(&mut old_interface);

        service.reconnect().await.unwrap();
        assert_eq!(rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::TransportClosed);
        assert_no_wire(&mut new_interface);
    }

    #[test]
    fn force_unsubscribe_closes_all_local_receivers() {
        let (mut service, mut interface) = test_service();
        let alias =
            activate(&mut service, &mut interface, eth_subscription(1, "newHeads"), "active");
        let mut first = service.subs.get_subscription(alias).unwrap();
        let mut second = service.subs.get_subscription(alias).unwrap();

        service.service_unsubscribe(alias, None).unwrap();
        let _ = take_wire(&mut interface);
        assert!(matches!(
            first.rx.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Closed)
        ));
        assert!(matches!(
            second.rx.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Closed)
        ));
    }

    #[test]
    fn force_unsubscribe_while_starting_compensates_the_late_server_id() {
        let (mut service, mut interface) = test_service();
        let request = eth_subscription(1, "newHeads");
        let alias = subscription_local_id(&request);
        let (in_flight, mut response_rx) = InFlight::new(request, 16);
        service.service_request(in_flight).unwrap();
        let _ = take_wire(&mut interface);

        let (tx, mut cleanup_rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        assert!(response_rx.try_recv().unwrap().unwrap_err().is_local_usage_error());
        assert_no_wire(&mut interface);

        service.handle_item(subscription_response(Id::Number(1), "late").into()).unwrap();
        let cleanup = take_wire(&mut interface);
        assert_eq!(cleanup["params"], serde_json::json!(["late"]));
        let cleanup_id = Id::String(cleanup["id"].as_str().unwrap().to_owned());
        service.handle_item(bool_response(cleanup_id, true).into()).unwrap();
        assert_eq!(cleanup_rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::ServerConfirmed);
        assert_eq!(service.subs.len(), 0);
    }

    #[test]
    fn force_unsubscribe_while_starting_reports_an_invalid_subscribe_response() {
        let (mut service, mut interface) = test_service();
        let request = eth_subscription(1, "newHeads");
        let alias = subscription_local_id(&request);
        let (in_flight, _response_rx) = InFlight::new(request, 16);
        service.service_request(in_flight).unwrap();
        let _ = take_wire(&mut interface);

        let (tx, mut cleanup_rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        let invalid = Response {
            id: Id::Number(1),
            payload: ResponsePayload::Success(to_json_raw_value(&true).unwrap()),
        };
        service.handle_item(invalid.into()).unwrap();

        assert!(cleanup_rx.try_recv().unwrap().unwrap_err().is_deser_error());
        assert_no_wire(&mut interface);
        assert_eq!(service.subs.len(), 0);
    }

    #[tokio::test]
    async fn force_unsubscribe_during_reconnect_tracks_late_server_id() {
        let (old_handle, mut old_interface) = ConnectionHandle::new();
        let (new_handle, mut new_interface) = ConnectionHandle::new();
        let connector = MockConnect(Arc::new(Mutex::new(Some(new_handle))));
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService::new(old_handle, connector, reqs);
        let alias = activate(
            &mut service,
            &mut old_interface,
            eth_subscription(1, "newHeads"),
            "old-server",
        );

        service.reconnect().await.unwrap();
        let replay = take_wire(&mut new_interface);
        let replay_id = Id::String(replay["id"].as_str().unwrap().to_owned());
        let (tx, mut rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        assert_no_wire(&mut new_interface);

        service.handle_item(subscription_response(replay_id, "late-server").into()).unwrap();
        let cleanup = take_wire(&mut new_interface);
        assert_eq!(cleanup["params"], serde_json::json!(["late-server"]));
        let cleanup_id = Id::String(cleanup["id"].as_str().unwrap().to_owned());
        service.handle_item(bool_response(cleanup_id, true).into()).unwrap();
        assert_eq!(rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::ServerConfirmed);
    }

    #[test]
    fn verified_custom_cleanup_methods_are_used_exactly() {
        let cases = [
            ("admin_peerEvents", "admin_unsubscribe"),
            ("reth_subscribeChainNotifications", "reth_unsubscribeChainNotifications"),
            ("reth_subscribePersistedBlock", "reth_unsubscribePersistedBlock"),
        ];

        for (index, (subscribe, unsubscribe)) in cases.into_iter().enumerate() {
            let (mut service, mut interface) = test_service();
            let alias = activate(
                &mut service,
                &mut interface,
                custom_subscription(index as u64 + 1, subscribe, Some(unsubscribe)),
                &format!("server-{index}"),
            );
            service.service_unsubscribe(alias, None).unwrap();
            assert_eq!(take_wire(&mut interface)["method"], unsubscribe);
        }
    }

    #[test]
    fn conflicting_custom_cleanup_method_keeps_first_creator_value() {
        let (mut service, mut interface) = test_service();
        let (first, mut first_rx) =
            InFlight::new(custom_subscription(1, "custom_events", Some("custom_unsubscribeA")), 16);
        let (second, mut second_rx) =
            InFlight::new(custom_subscription(2, "custom_events", Some("custom_unsubscribeB")), 16);
        service.service_request(first).unwrap();
        let _ = take_wire(&mut interface);
        service.service_request(second).unwrap();
        service.handle_item(subscription_response(Id::Number(1), "custom").into()).unwrap();
        let alias = alias_from_response(first_rx.try_recv().unwrap().unwrap(), Id::Number(1));
        let second_alias =
            alias_from_response(second_rx.try_recv().unwrap().unwrap(), Id::Number(2));
        assert_eq!(alias, second_alias);

        service.service_unsubscribe(alias, None).unwrap();
        assert_eq!(take_wire(&mut interface)["method"], "custom_unsubscribeA");
    }

    #[test]
    fn custom_subscription_without_cleanup_method_waits_for_connection_close() {
        let (mut service, mut interface) = test_service();
        let alias = activate(
            &mut service,
            &mut interface,
            custom_subscription(1, "custom_events", None),
            "custom",
        );
        let (tx, mut rx) = oneshot::channel();
        service.service_unsubscribe(alias, Some(tx)).unwrap();
        assert_no_wire(&mut interface);
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Empty)));

        service.finish_connection_epoch(0);
        assert_eq!(rx.try_recv().unwrap().unwrap(), UnsubscribeOutcome::TransportClosed);
    }

    #[tokio::test]
    async fn reconnect_uses_unique_route_and_preserves_channel_capacity() {
        let (old_handle, mut old_interface) = ConnectionHandle::new();
        let (new_handle, mut new_interface) = ConnectionHandle::new();
        let connector = MockConnect(Arc::new(Mutex::new(Some(new_handle))));
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService::new(old_handle, connector, reqs);
        let request = with_channel_size(eth_subscription(1, "newHeads"), 23);
        let local_id = subscription_local_id(&request);
        let _ = activate(&mut service, &mut old_interface, request, "old-server");

        service.reconnect().await.unwrap();
        let replay = take_wire(&mut new_interface);
        assert_eq!(replay["method"], "eth_subscribe");
        let replay_id = replay["id"].as_str().unwrap().to_owned();
        assert!(replay_id.starts_with("alloy-pubsub:1:"));
        assert_eq!(service.subs.get(&local_id).unwrap().channel_size, 23);

        service
            .handle_item(subscription_response(Id::String(replay_id), "new-server").into())
            .unwrap();
        assert_eq!(
            service.subs.server_id_for_local_id(&local_id),
            Some(&SubId::String("new-server".into()))
        );
    }

    #[tokio::test]
    async fn reconnect_does_not_reissue_fully_cancelled_starting_request() {
        let (old_handle, mut old_interface) = ConnectionHandle::new();
        let (new_handle, mut new_interface) = ConnectionHandle::new();
        let connector = MockConnect(Arc::new(Mutex::new(Some(new_handle))));
        let (_tx, reqs) = mpsc::unbounded_channel();
        let mut service = PubSubService::new(old_handle, connector, reqs);
        let (in_flight, rx) = InFlight::new(eth_subscription(1, "newHeads"), 16);
        service.service_request(in_flight).unwrap();
        let _ = take_wire(&mut old_interface);
        drop(rx);

        service.reconnect().await.unwrap();
        assert!(service.starting.is_empty());
        assert_no_wire(&mut new_interface);
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
        let service = PubSubService::new(dead_handle, connector, reqs);
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
        let service = PubSubService::new(dead_handle, connector, reqs);
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
