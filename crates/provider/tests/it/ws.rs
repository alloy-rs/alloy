use std::time::Duration;

use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::{ConnectionConfig, RpcClient};
use alloy_transport::layers::RetryBackoffLayer;
use alloy_transport_ws::WsConnect;
use futures::{SinkExt, StreamExt};
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::Message;

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

// <https://github.com/alloy-rs/alloy/issues/3948>
#[tokio::test]
async fn subscribe_blocks_with_server_subscription_ids() -> Result<(), Box<dyn std::error::Error>> {
    for (server_id, notification_id) in [
        (serde_json::json!("0x1"), serde_json::json!("0x1")),
        (
            serde_json::json!("0x7413bf1aeb8f1c0087c36b4243f7a41a"),
            serde_json::json!("0x7413bf1aeb8f1c0087c36b4243f7a41a"),
        ),
        (
            serde_json::json!(format!("0x{}", "ab".repeat(32))),
            serde_json::json!(format!("0x{}", "ab".repeat(32))),
        ),
        (serde_json::json!("opaque-subscription-id"), serde_json::json!("opaque-subscription-id")),
        (serde_json::json!(42), serde_json::json!(42)),
        (serde_json::json!("0x2a"), serde_json::json!(42)),
        (serde_json::json!(42), serde_json::json!("0x002A")),
        (serde_json::json!("0x00aB"), serde_json::json!("0xab")),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let mut expected: alloy_rpc_types_eth::Header = Default::default();
        expected.inner.number = 42;
        expected.hash = alloy_primitives::B256::repeat_byte(0x42);
        let header = serde_json::to_value(&expected)?;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let id = server_id.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let message = ws.next().await.expect("missing subscribe request").unwrap();
            let request: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(request["method"], "eth_subscribe");
            assert_eq!(request["params"], serde_json::json!(["newHeads"]));
            assert!(request["id"].is_number());

            let response = serde_json::json!({
                "jsonrpc": "2.0", "id": request["id"], "result": id
            });
            ws.send(Message::Text(response.to_string().into())).await.unwrap();
            // Different numeric and opaque IDs must not leak into this subscription.
            for unrelated in [
                serde_json::json!(43),
                serde_json::json!("opaque-other"),
                serde_json::json!("0xnothex"),
            ] {
                let wrong = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "eth_subscription",
                    "params": {"subscription": unrelated, "result": {}}
                });
                ws.send(Message::Text(wrong.to_string().into())).await.unwrap();
            }
            // Deliver immediately to also exercise buffering before the subscription is retrieved.
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_subscription",
                "params": {"subscription": notification_id, "result": header}
            });
            ws.send(Message::Text(notification.to_string().into())).await.unwrap();
            let malformed = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_subscription",
                "params": {
                    "subscription": notification_id,
                    "result": {"number": "not-a-quantity"}
                }
            });
            ws.send(Message::Text(malformed.to_string().into())).await.unwrap();
            shutdown_rx.await.expect("client did not consume notification");
        });

        let client = async {
            let provider =
                ProviderBuilder::new().connect_ws(WsConnect::new(format!("ws://{addr}"))).await?;
            let subscription = provider.subscribe_blocks().await?;
            let mut results = subscription.into_result_stream();
            let actual = results.next().await.expect("subscription ended")?;
            assert_eq!(actual, expected, "server subscription ID: {server_id}");
            assert!(results.next().await.expect("missing malformed header").is_err());
            shutdown_tx.send(()).expect("server stopped before consumption");
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (client_result, server_result) = tokio::join!(
            timeout(Duration::from_secs(5), client),
            timeout(Duration::from_secs(5), server)
        );
        // Inspect the server first so assertion and socket failures are not hidden by client
        // timeouts.
        server_result.expect("local WebSocket server timed out")?;
        client_result.unwrap_or_else(|_| panic!("subscription timed out for {server_id}"))?;
    }
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

#[tokio::test]
async fn replay_publicnode_newheads_wire() -> Result<(), Box<dyn std::error::Error>> {
    // Captured from ethereum-rpc.publicnode.com; replayed locally without network access.
    let [_, mut response, notification]: [serde_json::Value; 3] =
        serde_json::from_str(include_str!("fixtures/publicnode-newheads.json"))?;
    let expected: alloy_rpc_types_eth::Header =
        serde_json::from_value(notification["params"]["result"].clone())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let request: serde_json::Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(request["method"], "eth_subscribe");
        assert_eq!(request["params"], serde_json::json!(["newHeads"]));
        // Only the JSON-RPC request correlation ID is adapted to this client.
        response["id"] = request["id"].clone();
        ws.send(Message::Text(response.to_string().into())).await.unwrap();
        ws.send(Message::Text(notification.to_string().into())).await.unwrap();
        shutdown_rx.await.unwrap();
    });
    let client = async {
        let provider =
            ProviderBuilder::new().connect_ws(WsConnect::new(format!("ws://{addr}"))).await?;
        let mut subscription = provider.subscribe_blocks().await?;
        let actual = subscription.recv_result().await??;
        assert_eq!(actual, expected);
        shutdown_tx.send(()).unwrap();
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (client_result, server_result) = tokio::join!(
        timeout(Duration::from_secs(5), client),
        timeout(Duration::from_secs(5), server)
    );
    server_result??;
    client_result??;
    Ok(())
}
