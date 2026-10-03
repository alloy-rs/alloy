use alloy_node_bindings::Anvil;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ws::WsConnect;
use futures_util::{SinkExt, StreamExt};
use similar_asserts::assert_eq;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn it_makes_a_request() {
    let anvil = Anvil::new().spawn();
    let url = anvil.ws_endpoint();
    let connector = WsConnect::new(url);
    let client = ClientBuilder::default().pubsub(connector).await.unwrap();
    let req: RpcCall<_, _, U64> = client.request_noparams("eth_blockNumber");
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
    let res = timeout.await.unwrap().unwrap();
    assert_eq!(res.to::<u64>(), 0);
}

/// Spawns a local WebSocket server that records every inbound text frame in the
/// returned buffer and replies to each frame with `responder(request_json)`.
///
/// Returns the `ws://` URL to connect to and the shared buffer of raw frames.
async fn spawn_ws_server(
    responder: impl Fn(serde_json::Value) -> String + Send + 'static,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{addr}");

    let frames = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_frames = frames.clone();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(msg)) = ws.next().await {
            match msg {
                Message::Text(text) => {
                    server_frames.lock().unwrap().push(text.to_string());
                    let request: serde_json::Value = serde_json::from_str(&text).unwrap();
                    let reply = responder(request);
                    ws.send(Message::text(reply)).await.unwrap();
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    (url, frames)
}

/// Builds a JSON-RPC batch response, mapping each request id `n` to result
/// `11 * (n + 1)` (id `0` -> `11`, id `1` -> `22`, ...). `reversed` controls
/// whether the responses are emitted in reversed order relative to the request.
fn batch_reply(request: &serde_json::Value, reversed: bool) -> String {
    let requests = request.as_array().expect("expected a JSON-RPC batch array");
    let mut responses: Vec<serde_json::Value> = requests
        .iter()
        .map(|req| {
            let id = req["id"].as_u64().unwrap();
            serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": 11 * (id + 1) })
        })
        .collect();
    if reversed {
        responses.reverse();
    }
    serde_json::to_string(&responses).unwrap()
}

/// A batch created via `new_batch()` must be transmitted as exactly one
/// WebSocket text frame whose JSON root is an array of the two requests.
#[tokio::test]
async fn batch_is_sent_as_one_frame() {
    let (url, frames) = spawn_ws_server(|req| batch_reply(&req, false)).await;

    let client = ClientBuilder::default().pubsub(WsConnect::new(url)).await.unwrap();
    let mut batch = client.new_batch();
    let first = batch.add_call::<_, u64>("first", &()).unwrap();
    let second = batch.add_call::<_, u64>("second", &()).unwrap();
    batch.send().await.unwrap();

    // Both individual futures resolve from the single batch response array.
    assert_eq!(first.await.unwrap(), 11);
    assert_eq!(second.await.unwrap(), 22);

    let frames = frames.lock().unwrap();
    assert_eq!(frames.len(), 1, "batch must be sent as exactly one WS frame");
    let request: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    assert!(request.is_array(), "the wire frame must be a JSON array");
    assert_eq!(request.as_array().unwrap().len(), 2);
}

/// Responses are routed by JSON-RPC id, not by array position: a server that
/// returns the batch responses in reversed order must still resolve each waiter
/// to its own result.
#[tokio::test]
async fn batch_responses_route_by_id() {
    let (url, _frames) = spawn_ws_server(|req| batch_reply(&req, true)).await;

    let client = ClientBuilder::default().pubsub(WsConnect::new(url)).await.unwrap();
    let mut batch = client.new_batch();
    let first = batch.add_call::<_, u64>("first", &()).unwrap();
    let second = batch.add_call::<_, u64>("second", &()).unwrap();
    batch.send().await.unwrap();

    assert_eq!(first.await.unwrap(), 11);
    assert_eq!(second.await.unwrap(), 22);
}

/// An empty batch must not produce any WebSocket message.
#[tokio::test]
async fn empty_batch_sends_no_frame() {
    let (url, frames) = spawn_ws_server(|req| batch_reply(&req, false)).await;

    let client = ClientBuilder::default().pubsub(WsConnect::new(url)).await.unwrap();
    let batch = client.new_batch();
    batch.send().await.unwrap();

    // Give any (erroneous) outbound frame a chance to arrive.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(frames.lock().unwrap().is_empty(), "an empty batch must not send a frame");
}
