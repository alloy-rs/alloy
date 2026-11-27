use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::{ConnectionConfig, RpcClient};
use alloy_transport::layers::RetryBackoffLayer;
use alloy_transport_ws::{WebSocketConfig, WsConnect};
use futures::{SinkExt, StreamExt};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message,
};

// <https://github.com/alloy-rs/alloy/issues/1972>
// <https://github.com/alloy-rs/alloy/issues/2362>
#[tokio::test]
async fn ws_retry_pubsub_unsubscribe() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();
    let retry_layer = RetryBackoffLayer::new(1, 50, 1600);
    let rpc_client =
        RpcClient::builder().layer(retry_layer).ws(WsConnect::new(anvil.ws_endpoint())).await?;

    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(rpc_client);

    let sub = provider.subscribe_blocks().await?;
    provider.unsubscribe(*sub.local_id()).unwrap();

    Ok(())
}

#[tokio::test]
async fn ws_retry_pubsub_connect_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();
    let retry_layer = RetryBackoffLayer::new(1, 50, 1600);

    let rpc_client = RpcClient::builder()
        .layer(retry_layer)
        .connect_with_config(&anvil.ws_endpoint(), ConnectionConfig::new())
        .await?;

    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(rpc_client);

    let _ = provider.subscribe_blocks().await?;

    Ok(())
}

// <https://github.com/alloy-rs/alloy/issues/1601>
#[tokio::test]
async fn test_subscription_race_condition() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

        // Read the subscribe request.
        let msg = ws.next().await.unwrap().unwrap();
        let req: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        let id = req["id"].clone();

        // Respond with a subscription ID.
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": "0x1"
        });
        ws.send(Message::Text(resp.to_string().into())).await.unwrap();

        // Send subscription notifications.
        for i in 0..usize::MAX {
            let notif = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "s_hello",
                "params": {
                    "subscription": "0x1",
                    "result": i
                }
            });
            if ws.send(Message::Text(notif.to_string().into())).await.is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });

    let ws_provider = ProviderBuilder::new().connect(format!("ws://{addr}").as_str()).await?;
    let mut request = ws_provider.client().request("subscribe_hello", ());
    // required if not eth_subscribe
    request.set_is_subscription();
    let sub_id = request.await?;
    // call the pubsub service to get a broadcast receiver.
    let mut sub = ws_provider.root().get_subscription(sub_id).await?;

    let num: usize = sub.recv().await.unwrap();
    assert_eq!(num, 0);

    Ok(())
}

#[cfg(feature = "debug-api")]
#[tokio::test]
async fn debug_trace_chain_subscription() -> Result<(), Box<dyn std::error::Error>> {
    use alloy_provider::ext::DebugApi;
    use alloy_rpc_types_eth::BlockNumberOrTag;
    use futures::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (request_tx, request_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

        let message = ws.next().await.unwrap().unwrap();
        let request: serde_json::Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        let id = request["id"].clone();
        request_tx.send(request).unwrap();

        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": "0x1"
        });
        ws.send(Message::Text(response.to_string().into())).await.unwrap();

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "debug_subscription",
            "params": {
                "subscription": "0x1",
                "result": {
                    "block": "0x2",
                    "hash": alloy_primitives::B256::repeat_byte(0x11),
                    "traces": []
                }
            }
        });
        ws.send(Message::Text(notification.to_string().into())).await.unwrap();
        let _ = shutdown_rx.await;
    });

    let provider = ProviderBuilder::new().connect(&format!("ws://{addr}")).await?;
    let mut subscription = provider
        .debug_subscribe_trace_chain(BlockNumberOrTag::Number(1), BlockNumberOrTag::Number(2), None)
        .await?;

    let request = request_rx.await?;
    assert_eq!(request["method"], "debug_subscribe");
    assert_eq!(request["params"], serde_json::json!(["traceChain", "0x1", "0x2", null]));

    let result = subscription.recv().await?;
    assert_eq!(result.block, alloy_primitives::U256::from(2));
    let _ = shutdown_tx.send(());
    server.await?;

    Ok(())
}

// Verifies that basic auth credentials embedded in a WS URL are automatically
// extracted and sent as an Authorization header.
#[tokio::test]
async fn ws_basic_auth_from_url() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<Option<String>>();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();

        // Use accept_hdr_async to inspect the Authorization header during the
        // WS upgrade handshake.
        let mut auth_tx = Some(auth_tx);
        let callback = |request: &http::Request<()>,
                        response: http::Response<()>|
         -> Result<http::Response<()>, http::Response<Option<String>>> {
            let auth_value = request
                .headers()
                .get(http::header::AUTHORIZATION)
                .map(|v| v.to_str().unwrap().to_string());
            auth_tx.take().unwrap().send(auth_value).unwrap();
            Ok(response)
        };

        let mut ws = tokio_tungstenite::accept_hdr_async(stream, callback).await.unwrap();

        // Read the eth_blockNumber request and respond.
        let msg = ws.next().await.unwrap().unwrap();
        let req: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        let id = req["id"].clone();

        let resp = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": "0x1" });
        ws.send(Message::Text(resp.to_string().into())).await.unwrap();
    });

    // URL with embedded basic auth — the provider should auto-extract it.
    let url = format!("ws://user:pass@{addr}");
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect(&url).await?;
    let num = provider.get_block_number().await?;
    assert_eq!(num, 1);

    // Verify the server received the Authorization header.
    let auth_header = auth_rx.await?.expect("Authorization header was not sent");
    assert!(auth_header.starts_with("Basic "), "expected Basic auth, got: {auth_header}");

    Ok(())
}

/// Spawns a WS server that completes every handshake, then closes the connection with `code` once
/// it reads a request. Returns its address and the number of connections it accepted.
async fn spawn_closing_ws_server(code: u16) -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let _ = ws.next().await;
                let frame = CloseFrame { code: CloseCode::from(code), reason: "closing".into() };
                let _ = ws.close(Some(frame)).await;
            });
        }
    });
    (addr, accepted)
}

// <https://github.com/alloy-rs/alloy/issues/3821>
#[tokio::test]
async fn ws_backend_closing_after_handshake_fails_request() -> Result<(), Box<dyn std::error::Error>>
{
    // 1013 (try again later) is retryable, but every new connection closes the same way.
    let (addr, accepted) = spawn_closing_ws_server(1013).await;
    let ws = WsConnect::new(format!("ws://{addr}"))
        .with_max_retries(2)
        .with_retry_interval(Duration::from_millis(10));
    let client = RpcClient::builder().ws(ws).await?;
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(client);

    let err = tokio::time::timeout(Duration::from_secs(10), provider.get_block_number())
        .await
        .expect("request must not hang")
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "3 consecutive backend connections closed without answering a request"
    );
    assert_eq!(accepted.load(Ordering::SeqCst), 3);

    Ok(())
}

#[tokio::test]
async fn ws_policy_violation_close_is_not_retried() -> Result<(), Box<dyn std::error::Error>> {
    let (addr, accepted) = spawn_closing_ws_server(1008).await;
    let client = RpcClient::builder().ws(WsConnect::new(format!("ws://{addr}"))).await?;
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(client);

    let err = tokio::time::timeout(Duration::from_secs(10), provider.get_block_number())
        .await
        .expect("request must not hang")
        .unwrap_err();
    assert!(err.as_transport_err().is_some_and(|err| err.is_non_retryable()), "{err:?}");
    assert_eq!(err.to_string(), "WS server closed the connection: closing (1008)");
    assert_eq!(accepted.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn ws_rejected_reconnect_handshake_is_not_retried() -> Result<(), Box<dyn std::error::Error>>
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    tokio::spawn(async move {
        // The first connection drops without a close frame, which is retryable.
        let (stream, _) = listener.accept().await.unwrap();
        counter.fetch_add(1, Ordering::SeqCst);
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let _ = ws.next().await;
        drop(ws);

        // Reconnecting is unauthorized.
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            let reject =
                |_: &http::Request<()>,
                 _: http::Response<()>|
                 -> Result<http::Response<()>, http::Response<Option<String>>> {
                    Err(http::Response::builder().status(401).body(None).unwrap())
                };
            let _ = tokio_tungstenite::accept_hdr_async(stream, reject).await;
        }
    });

    let client = RpcClient::builder().ws(WsConnect::new(format!("ws://{addr}"))).await?;
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(client);

    let err = tokio::time::timeout(Duration::from_secs(10), provider.get_block_number())
        .await
        .expect("request must not hang")
        .unwrap_err();
    assert!(err.as_transport_err().is_some_and(|err| err.is_non_retryable()), "{err:?}");
    assert_eq!(accepted.load(Ordering::SeqCst), 2);

    Ok(())
}

// An oversized message is retryable: it fails the request that caused it, not the connection.
#[tokio::test]
async fn ws_oversized_response_fails_request_and_keeps_connection(
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(msg)) = ws.next().await {
                    let Ok(text) = msg.to_text() else { continue };
                    let req: serde_json::Value = serde_json::from_str(text).unwrap();
                    // `eth_getLogs` gets an oversized response, anything else a small one.
                    let result = if req["method"] == "eth_getLogs" {
                        serde_json::json!(format!("0x{}", "0".repeat(4096)))
                    } else {
                        serde_json::json!("0x1")
                    };
                    let resp =
                        serde_json::json!({ "jsonrpc": "2.0", "id": req["id"], "result": result });
                    if ws.send(Message::Text(resp.to_string().into())).await.is_err() {
                        break;
                    }
                }
            });
        }
    });

    let config = WebSocketConfig::default().max_message_size(Some(1024));
    let ws = WsConnect::new(format!("ws://{addr}"))
        .with_config(config)
        .with_retry_interval(Duration::from_millis(10));
    let client = RpcClient::builder().ws(ws).await?;
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(client);

    let logs =
        provider.client().request::<_, serde_json::Value>("eth_getLogs", [serde_json::json!({})]);
    let err = tokio::time::timeout(Duration::from_secs(10), logs)
        .await
        .expect("request must not hang")
        .unwrap_err();
    assert_eq!(err.to_string(), "backend connection closed 4 times while the request was pending");

    // The connection is restored for the following requests.
    let num = tokio::time::timeout(Duration::from_secs(10), provider.get_block_number())
        .await
        .expect("request must not hang")?;
    assert_eq!(num, 1);
    // Four connections died on the oversized response, the fifth answered the next request.
    assert_eq!(accepted.load(Ordering::SeqCst), 5);

    Ok(())
}
