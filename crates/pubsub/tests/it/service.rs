use alloy_json_rpc::{Id, Request, RequestPacket, Response, ResponsePacket, SerializedRequest};
use alloy_primitives::B256;
use alloy_pubsub::{ConnectionHandle, ConnectionInterface, PubSubConnect, PubSubFrontend};
use alloy_transport::{TransportErrorKind, TransportResult};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{sync::broadcast::error::RecvError, time::Instant};

const MAX_RETRIES: u32 = 3;

#[derive(Clone, Debug, Default)]
struct MockConnect {
    backends: Arc<Mutex<VecDeque<ConnectionHandle>>>,
    reconnects: Arc<AtomicUsize>,
}

impl PubSubConnect for MockConnect {
    fn is_local(&self) -> bool {
        true
    }

    async fn connect(&self) -> TransportResult<ConnectionHandle> {
        self.backends
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| TransportErrorKind::custom_str("no backend left"))
    }

    async fn try_reconnect(&self) -> TransportResult<ConnectionHandle> {
        self.reconnects.fetch_add(1, Ordering::SeqCst);
        self.connect().await
    }
}

#[derive(Debug)]
struct Backend(ConnectionInterface);

impl Backend {
    async fn recv(&mut self) -> Value {
        let req = within(self.0.recv_from_frontend()).await.expect("service dropped the backend");
        serde_json::from_str(req.get()).unwrap()
    }

    fn send(&self, item: Value) {
        self.0.send_to_frontend(serde_json::from_str(&item.to_string()).unwrap()).unwrap();
    }

    fn respond(&self, id: u64, result: Value) {
        self.send(json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }

    fn notify(&self, server_id: &str, result: Value) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "eth_subscription",
            "params": { "subscription": server_id, "result": result },
        }));
    }

    fn close(self) {
        self.0.close_with_error();
    }
}

async fn spawn_service<const N: usize>() -> (PubSubFrontend, MockConnect, [Backend; N]) {
    spawn_service_with(MAX_RETRIES).await
}

async fn spawn_service_with<const N: usize>(
    max_retries: u32,
) -> (PubSubFrontend, MockConnect, [Backend; N]) {
    let connector = MockConnect::default();
    let backends = std::array::from_fn(|_| {
        let (handle, interface) = ConnectionHandle::new();
        connector.backends.lock().unwrap().push_back(handle.with_max_retries(max_retries));
        Backend(interface)
    });
    let frontend = connector.clone().into_service().await.unwrap();
    (frontend, connector, backends)
}

async fn within<F: Future>(fut: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(60), fut).await.expect("pubsub service stalled")
}

fn request(method: &'static str, id: u64, params: Value) -> SerializedRequest {
    Request::new(method, Id::Number(id), params).serialize().unwrap()
}

fn to_json(req: &SerializedRequest) -> Value {
    serde_json::from_str(req.serialized().get()).unwrap()
}

async fn subscribe(
    frontend: &PubSubFrontend,
    backend: &mut Backend,
    id: u64,
    params: Value,
    server_id: &str,
) -> B256 {
    subscribe_with(frontend, backend, request("eth_subscribe", id, params), server_id).await
}

async fn subscribe_with(
    frontend: &PubSubFrontend,
    backend: &mut Backend,
    req: SerializedRequest,
    server_id: &str,
) -> B256 {
    let (method, id) = (req.method().to_owned(), req.id().clone());
    let resp = tokio::spawn(frontend.send(req));
    assert_eq!(backend.recv().await["method"], method);
    let Id::Number(id) = id else { unreachable!() };
    backend.respond(id, json!(server_id));
    local_id(&within(resp).await.unwrap().unwrap())
}

fn local_id(resp: &Response) -> B256 {
    serde_json::from_str(resp.payload.as_success().unwrap().get()).unwrap()
}

/// Asserts that a request sent now is the next one on the wire, i.e. that the service dispatched
/// nothing else for the instructions queued before it.
async fn probe(frontend: &PubSubFrontend, backend: &mut Backend, id: u64) {
    let resp = tokio::spawn(frontend.send(request("eth_blockNumber", id, json!([]))));
    let req = backend.recv().await;
    assert_eq!((&req["method"], &req["id"]), (&json!("eth_blockNumber"), &json!(id)), "{req}");
    backend.respond(id, json!("0x1"));
    within(resp).await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn reconnect_reissues_pending_request() {
    let (frontend, _, [mut first, mut second]) = spawn_service().await;
    let req = request("eth_blockNumber", 7, json!([]));
    let expected = to_json(&req);

    let resp = tokio::spawn(frontend.send(req));
    assert_eq!(first.recv().await, expected);
    first.close();

    assert_eq!(second.recv().await, expected);
    second.respond(7, json!("0x10"));

    let resp = within(resp).await.unwrap().unwrap();
    assert_eq!(resp.id, Id::Number(7));
    assert_eq!(resp.payload.as_success().unwrap().get(), r#""0x10""#);
}

#[tokio::test(start_paused = true)]
async fn reconnect_resubscribes_under_new_server_id() {
    let (frontend, _, [mut first, mut second]) = spawn_service().await;
    let expected = to_json(&request("eth_subscribe", 1, json!(["newHeads"])));
    let local_id = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0xaaa").await;
    let mut sub = frontend.get_subscription(local_id).await.unwrap();
    first.notify("0xaaa", json!(1));
    assert_eq!(within(sub.recv()).await.unwrap().get(), "1");

    first.close();
    assert_eq!(second.recv().await, expected);
    second.notify("0xaaa", json!(2));
    second.respond(1, json!("0xbbb"));
    second.notify("0xaaa", json!(3));
    second.notify("0xbbb", json!(4));

    assert_eq!(within(sub.recv()).await.unwrap().get(), "4");
    assert!(sub.is_empty());
    assert!(frontend.get_subscription(local_id).await.unwrap().same_channel(&sub));
}

#[tokio::test(start_paused = true)]
async fn unsubscribe_during_resubscribe_keeps_service_alive() {
    let (frontend, _, [mut first, mut second]) = spawn_service().await;
    let local_id = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0xaaa").await;
    first.notify("0xaaa", json!(1));
    let sub = frontend.get_subscription(local_id).await.unwrap();

    first.close();
    assert_eq!(second.recv().await["method"], "eth_subscribe");
    frontend.unsubscribe(local_id).unwrap();
    assert!(frontend.get_subscription(local_id).await.is_err());
    drop(sub);
    second.respond(1, json!("0xbbb"));
    // The server-side subscription created by the resubscription is cancelled.
    let unsubscribe = second.recv().await;
    assert_eq!(unsubscribe["method"], "eth_unsubscribe");
    assert_eq!(unsubscribe["params"], json!(["0xbbb"]));

    let resp = tokio::spawn(frontend.send(request("eth_blockNumber", 2, json!([]))));
    assert_eq!(second.recv().await["method"], "eth_blockNumber");
    second.respond(2, json!("0x10"));
    assert_eq!(
        within(resp).await.unwrap().unwrap().payload.as_success().unwrap().get(),
        r#""0x10""#
    );
    assert!(frontend.get_subscription(local_id).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn unsubscribe_during_resubscribe_without_receivers_keeps_service_alive() {
    let (frontend, _, [mut first, mut second]) = spawn_service().await;
    let local = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0xaaa").await;
    // Take the initial receiver and drop it, so that the subscription has no receiver left.
    first.notify("0xaaa", json!(1));
    drop(frontend.get_subscription(local).await.unwrap());
    // A caller is handed the local id, but has not fetched its receiver yet.
    let resp = within(frontend.send(request("eth_subscribe", 2, json!(["newHeads"])))).await;
    assert_eq!(local_id(&resp.unwrap()), local);

    first.close();
    assert_eq!(second.recv().await["method"], "eth_subscribe");
    frontend.unsubscribe(local).unwrap();
    assert!(frontend.get_subscription(local).await.is_err());
    second.respond(1, json!("0xbbb"));
    let unsubscribe = second.recv().await;
    assert_eq!(unsubscribe["method"], "eth_unsubscribe");
    assert_eq!(unsubscribe["params"], json!(["0xbbb"]));
    probe(&frontend, &mut second, 3).await;
}

#[tokio::test(start_paused = true)]
async fn exhausted_reconnects_shut_down_service() {
    let (mut frontend, connector, [mut backend]) = spawn_service().await;
    let resp = tokio::spawn(frontend.send(request("eth_blockNumber", 1, json!([]))));
    backend.recv().await;
    backend.close();

    let err = within(resp).await.unwrap().unwrap_err();
    assert_eq!(err.to_string(), "no backend left");
    assert_eq!(connector.reconnects.load(Ordering::SeqCst), MAX_RETRIES as usize);

    let err =
        std::future::poll_fn(|cx| tower::Service::poll_ready(&mut frontend, cx)).await.unwrap_err();
    assert!(err.as_transport_err().is_some_and(TransportErrorKind::is_backend_gone), "{err:?}");
}

#[tokio::test(start_paused = true)]
async fn subscription_response_carries_local_id() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let resp = tokio::spawn(frontend.send(request("eth_subscribe", 1, json!(["newHeads"]))));
    backend.recv().await;
    backend.respond(1, json!("0xaaa"));

    let resp = within(resp).await.unwrap().unwrap();
    assert_eq!(resp.id, Id::Number(1));
    let local_id: B256 = serde_json::from_str(resp.payload.as_success().unwrap().get()).unwrap();
    let mut sub = frontend.get_subscription(local_id).await.unwrap();
    assert_eq!(sub.local_id, local_id);

    backend.notify("0xbbb", json!(1));
    backend.notify("0xaaa", json!(2));
    assert_eq!(within(sub.recv()).await.unwrap().get(), "2");
    assert!(sub.is_empty());
}

#[tokio::test(start_paused = true)]
async fn identical_subscriptions_share_local_id() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let first = subscribe(&frontend, &mut backend, 1, json!(["newHeads"]), "0xaaa").await;
    let mut sub = frontend.get_subscription(first).await.unwrap();
    // The identical subscribe is answered without another server-side subscription.
    let second = within(frontend.send(request("eth_subscribe", 2, json!(["newHeads"])))).await;
    let second = second.unwrap();
    assert_eq!(second.id, Id::Number(2));
    let other =
        subscribe(&frontend, &mut backend, 3, json!(["newPendingTransactions"]), "0xccc").await;
    assert_eq!(first, local_id(&second));
    assert_ne!(first, other);

    backend.notify("0xaaa", json!(1));
    assert_eq!(within(sub.recv()).await.unwrap().get(), "1");
    assert!(sub.is_empty());
}

#[tokio::test(start_paused = true)]
async fn unsubscribe_sends_server_id_and_drops_subscription() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let local_id = subscribe(&frontend, &mut backend, 1, json!(["newHeads"]), "0xaaa").await;

    frontend.unsubscribe(local_id).unwrap();
    let unsubscribe = backend.recv().await;
    assert_eq!(unsubscribe["method"], "eth_unsubscribe");
    assert_eq!(unsubscribe["params"], json!(["0xaaa"]));
    assert!(frontend.get_subscription(local_id).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn unsubscribe_reply_does_not_resolve_unrelated_request() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let local_id = subscribe(&frontend, &mut backend, 0, json!(["newHeads"]), "0xaaa").await;
    let resp = tokio::spawn(frontend.send(request("eth_blockNumber", 1, json!([]))));
    backend.recv().await;

    frontend.unsubscribe(local_id).unwrap();
    let unsubscribe = backend.recv().await;
    backend.send(json!({ "jsonrpc": "2.0", "id": unsubscribe["id"], "result": true }));
    backend.respond(1, json!("0x10"));

    let resp = within(resp).await.unwrap().unwrap();
    assert_eq!(resp.payload.as_success().unwrap().get(), r#""0x10""#);
}

#[tokio::test(start_paused = true)]
async fn get_subscription_for_unknown_id_fails() {
    let (frontend, _, [_backend]) = spawn_service().await;
    let err = frontend.get_subscription(B256::repeat_byte(1)).await.unwrap_err();
    assert!(err.to_string().contains("subscription not found"), "{err:?}");
}

#[tokio::test(start_paused = true)]
async fn batch_is_split_and_reassembled_in_order() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let reqs = vec![request("eth_blockNumber", 1, json!([])), request("eth_chainId", 2, json!([]))];
    let expected: Vec<_> = reqs.iter().map(to_json).collect();

    let resp = tokio::spawn(frontend.send_packet(RequestPacket::Batch(reqs)));
    let mut dispatched = vec![backend.recv().await, backend.recv().await];
    dispatched.sort_by_key(|req| req["id"].as_u64());
    assert_eq!(dispatched, expected);

    backend.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "error": { "code": -32601, "message": "method not found" },
    }));
    backend.respond(1, json!("0x10"));

    let ResponsePacket::Batch(resps) = within(resp).await.unwrap().unwrap() else {
        panic!("expected a batch response");
    };
    assert_eq!(resps.len(), 2);
    assert_eq!(resps[0].id, Id::Number(1));
    assert_eq!(resps[0].payload.as_success().unwrap().get(), r#""0x10""#);
    assert_eq!(resps[1].id, Id::Number(2));
    assert_eq!(resps[1].payload.as_error().unwrap().code, -32601);
}

#[tokio::test(start_paused = true)]
async fn concurrent_identical_subscribes_share_one_request() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let params = json!(["logs", { "address": "0x0000000000000000000000000000000000000001" }]);
    let first = tokio::spawn(frontend.send(request("eth_subscribe", 3, params.clone())));
    assert_eq!(backend.recv().await["id"], 3);
    let mut second = Box::pin(frontend.send(request("eth_subscribe", 4, params)));
    assert!(futures::poll!(&mut second).is_pending());
    probe(&frontend, &mut backend, 5).await;
    backend.respond(3, json!("0xbbb"));

    let first = within(first).await.unwrap().unwrap();
    let second = within(second).await.unwrap();
    assert_eq!((&first.id, &second.id), (&Id::Number(3), &Id::Number(4)));
    assert_eq!(local_id(&first), local_id(&second));

    // Both callers share the single server-side subscription.
    let mut subs = [
        frontend.get_subscription(local_id(&first)).await.unwrap(),
        frontend.get_subscription(local_id(&second)).await.unwrap(),
    ];
    backend.notify("0xbbb", json!(1));
    for sub in &mut subs {
        assert_eq!(within(sub.recv()).await.unwrap().get(), "1");
    }
    frontend.unsubscribe(local_id(&first)).unwrap();
    let unsubscribe = backend.recv().await;
    assert_eq!(unsubscribe["params"], json!(["0xbbb"]));
    probe(&frontend, &mut backend, 6).await;
}

#[tokio::test(start_paused = true)]
async fn failed_subscribe_fails_every_waiter() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let first = tokio::spawn(frontend.send(request("eth_subscribe", 1, json!(["newHeads"]))));
    assert_eq!(backend.recv().await["id"], 1);
    let mut second = Box::pin(frontend.send(request("eth_subscribe", 2, json!(["newHeads"]))));
    assert!(futures::poll!(&mut second).is_pending());
    probe(&frontend, &mut backend, 3).await;
    backend.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": { "code": -32000, "message": "too many subscriptions" },
    }));

    let first = within(first).await.unwrap().unwrap();
    let second = within(second).await.unwrap();
    assert_eq!((&first.id, &second.id), (&Id::Number(1), &Id::Number(2)));
    for resp in [first, second] {
        assert_eq!(resp.payload.as_error().unwrap().message, "too many subscriptions");
    }

    // The next attempt creates the subscription.
    subscribe(&frontend, &mut backend, 4, json!(["newHeads"]), "0xaaa").await;
}

#[tokio::test(start_paused = true)]
async fn cancelled_subscribe_is_unsubscribed() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let mut resp = Box::pin(frontend.send(request("eth_subscribe", 1, json!(["newHeads"]))));
    assert!(futures::poll!(&mut resp).is_pending());
    assert_eq!(backend.recv().await["method"], "eth_subscribe");
    drop(resp);
    backend.respond(1, json!("0xaaa"));

    let unsubscribe = backend.recv().await;
    assert_eq!(unsubscribe["method"], "eth_unsubscribe");
    assert_eq!(unsubscribe["params"], json!(["0xaaa"]));

    // The subscription was not kept, so the next subscribe creates it again.
    subscribe(&frontend, &mut backend, 2, json!(["newHeads"]), "0xbbb").await;
}

#[tokio::test(start_paused = true)]
async fn custom_subscriptions_are_distinct_and_cancelled_in_their_namespace() {
    let (frontend, _, [mut backend]) = spawn_service().await;
    let custom = |method, id| {
        let mut req = Request::new(method, Id::Number(id), ()).serialize().unwrap();
        req.set_is_subscription();
        req
    };
    // Both requests have the same (empty) params.
    let persisted =
        subscribe_with(&frontend, &mut backend, custom("reth_subscribePersistedBlock", 1), "0xaaa")
            .await;
    let peers =
        subscribe_with(&frontend, &mut backend, custom("admin_peerEvents", 2), "0xbbb").await;
    assert_ne!(persisted, peers);

    let mut persisted_sub = frontend.get_subscription(persisted).await.unwrap();
    let mut peers_sub = frontend.get_subscription(peers).await.unwrap();
    backend.notify("0xaaa", json!(1));
    backend.notify("0xbbb", json!(2));
    assert_eq!(within(persisted_sub.recv()).await.unwrap().get(), "1");
    assert_eq!(within(peers_sub.recv()).await.unwrap().get(), "2");

    for (local_id, method, server_id) in [
        (persisted, "reth_unsubscribePersistedBlock", "0xaaa"),
        (peers, "admin_unsubscribe", "0xbbb"),
    ] {
        frontend.unsubscribe(local_id).unwrap();
        let unsubscribe = backend.recv().await;
        assert_eq!(unsubscribe["method"], method);
        assert_eq!(unsubscribe["params"], json!([server_id]));
    }
}

#[tokio::test(start_paused = true)]
async fn reconnect_skips_subscriptions_without_consumers() {
    let (frontend, _, [mut first, mut second]) = spawn_service().await;
    let dropped = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0xaaa").await;
    drop(frontend.get_subscription(dropped).await.unwrap());
    let kept =
        subscribe(&frontend, &mut first, 2, json!(["newPendingTransactions"]), "0xbbb").await;
    let _sub = frontend.get_subscription(kept).await.unwrap();
    // The caller was handed the local id but has not fetched its receiver yet.
    let unclaimed = subscribe(&frontend, &mut first, 3, json!(["logs", {}]), "0xccc").await;

    first.close();
    let mut resubscribed = [second.recv().await["id"].as_u64(), second.recv().await["id"].as_u64()];
    resubscribed.sort();
    assert_eq!(resubscribed, [Some(2), Some(3)]);
    probe(&frontend, &mut second, 4).await;

    assert!(frontend.get_subscription(dropped).await.is_err());
    assert!(frontend.get_subscription(unclaimed).await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn repeated_reconnect_resubscribes_once() {
    let (frontend, _, [mut first, mut second, mut third]) = spawn_service().await;
    let local_id = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0xaaa").await;
    let mut sub = frontend.get_subscription(local_id).await.unwrap();

    first.close();
    assert_eq!(second.recv().await["method"], "eth_subscribe");
    // The backend dies before acknowledging the resubscription.
    second.close();
    assert_eq!(third.recv().await["method"], "eth_subscribe");
    probe(&frontend, &mut third, 2).await;

    third.respond(1, json!("0xccc"));
    third.notify("0xccc", json!(1));
    assert_eq!(within(sub.recv()).await.unwrap().get(), "1");
}

#[tokio::test(start_paused = true)]
async fn backends_dying_without_answering_exhaust_health_budget() {
    let (frontend, connector, backends) = spawn_service_with::<3>(2).await;
    let resp = tokio::spawn(frontend.send(request("eth_blockNumber", 1, json!([]))));

    let mut deaths = Vec::new();
    for mut backend in backends {
        assert_eq!(backend.recv().await["id"], 1);
        // Notifications are no progress.
        backend.notify("0xaaa", json!(1));
        deaths.push(Instant::now());
        backend.close();
    }

    let err = within(resp).await.unwrap().unwrap_err();
    assert_eq!(
        err.to_string(),
        "3 consecutive backend connections closed without answering a request"
    );
    assert_eq!(connector.reconnects.load(Ordering::SeqCst), 2);
    // Connecting succeeded every time, yet the second replacement waited for the retry interval.
    assert_eq!(deaths[1] - deaths[0], Duration::ZERO);
    assert_eq!(deaths[2] - deaths[1], Duration::from_secs(3));
}

#[tokio::test(start_paused = true)]
async fn idle_backend_deaths_do_not_exhaust_health_budget() {
    let (frontend, connector, [first, second, third, fourth, mut last]) =
        spawn_service_with(2).await;
    let start = Instant::now();
    for backend in [first, second, third, fourth] {
        backend.close();
    }

    probe(&frontend, &mut last, 1).await;
    assert_eq!(connector.reconnects.load(Ordering::SeqCst), 4);
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn request_that_keeps_killing_backends_fails() {
    let (frontend, _, [mut first, second, third, fourth, mut last]) = spawn_service().await;
    let local_id = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0x1").await;
    let mut sub = frontend.get_subscription(local_id).await.unwrap();
    let resp = tokio::spawn(frontend.send(request("eth_getLogs", 2, json!([{}]))));
    assert_eq!(first.recv().await["id"], 2);
    let start = Instant::now();
    first.close();

    for (server_id, mut backend) in [(2, second), (3, third), (4, fourth)] {
        // The backend acknowledges the resubscription, but dies on the replayed request.
        let mut reqs = [backend.recv().await, backend.recv().await];
        reqs.sort_by_key(|req| req["id"].as_u64());
        assert_eq!(reqs.map(|req| req["method"].clone()), ["eth_subscribe", "eth_getLogs"]);
        backend.respond(1, json!(format!("0x{server_id}")));
        backend.close();
    }

    let err = within(resp).await.unwrap().unwrap_err();
    assert_eq!(err.to_string(), "backend connection closed 4 times while the request was pending");
    // Backends that answered a request are healthy, so there was no backoff.
    assert_eq!(start.elapsed(), Duration::ZERO);

    // The service keeps running without the failed request.
    assert_eq!(last.recv().await["method"], "eth_subscribe");
    probe(&frontend, &mut last, 3).await;
    last.respond(1, json!("0x5"));
    last.notify("0x5", json!(1));
    assert_eq!(within(sub.recv()).await.unwrap().get(), "1");
}

#[tokio::test(start_paused = true)]
async fn resubscription_that_keeps_killing_backends_is_dropped() {
    let (frontend, _, [mut first, second, third, fourth, fifth, mut last]) = spawn_service().await;
    let local_id = subscribe(&frontend, &mut first, 1, json!(["newHeads"]), "0x1").await;
    let mut sub = frontend.get_subscription(local_id).await.unwrap();
    first.close();

    for (id, mut backend) in [(10, second), (11, third), (12, fourth), (13, fifth)] {
        // The backend answers other requests, but dies before acknowledging the resubscription.
        assert_eq!(backend.recv().await["method"], "eth_subscribe");
        probe(&frontend, &mut backend, id).await;
        backend.close();
    }

    // Dropping the subscription closed its receivers.
    assert!(matches!(within(sub.recv()).await, Err(RecvError::Closed)));
    probe(&frontend, &mut last, 20).await;
    assert!(frontend.get_subscription(local_id).await.is_err());
}
