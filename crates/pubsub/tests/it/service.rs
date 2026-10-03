use alloy_json_rpc::{Id, Request, RequestPacket, ResponsePacket, SerializedRequest};
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
    let connector = MockConnect::default();
    let backends = std::array::from_fn(|_| {
        let (handle, interface) = ConnectionHandle::new();
        connector.backends.lock().unwrap().push_back(handle.with_max_retries(MAX_RETRIES));
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
    let resp = tokio::spawn(frontend.send(request("eth_subscribe", id, params)));
    assert_eq!(backend.recv().await["method"], "eth_subscribe");
    backend.respond(id, json!(server_id));
    let resp = within(resp).await.unwrap().unwrap();
    serde_json::from_str(resp.payload.as_success().unwrap().get()).unwrap()
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
async fn exhausted_reconnects_shut_down_service() {
    let (mut frontend, connector, [mut backend]) = spawn_service().await;
    let resp = tokio::spawn(frontend.send(request("eth_blockNumber", 1, json!([]))));
    backend.recv().await;
    backend.close();

    let err = within(resp).await.unwrap().unwrap_err();
    assert!(err.as_transport_err().is_some_and(TransportErrorKind::is_backend_gone), "{err:?}");
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
    let second = subscribe(&frontend, &mut backend, 2, json!(["newHeads"]), "0xbbb").await;
    let other =
        subscribe(&frontend, &mut backend, 3, json!(["newPendingTransactions"]), "0xccc").await;
    assert_eq!(first, second);
    assert_ne!(first, other);

    backend.notify("0xbbb", json!(1));
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
