use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::RpcClient;
use alloy_transport::layers::RetryBackoffLayer;
use alloy_transport_ws::WsConnect;

struct ProviderConfig {
    endpoint: String,
    max_rate_limit_retries: u32,
    initial_backoff: u64,
    compute_units_per_second: u64,
}

// <https://github.com/alloy-rs/alloy/issues/1972>
#[tokio::test]
async fn ws_retry_pubsub() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();
    let provider_config = ProviderConfig {
        endpoint: anvil.ws_endpoint(),
        max_rate_limit_retries: 1,
        initial_backoff: 50,
        compute_units_per_second: 1600,
    };

    let ws = WsConnect::new(provider_config.endpoint);
    let retry_layer = RetryBackoffLayer::new(
        provider_config.max_rate_limit_retries,
        provider_config.initial_backoff,
        provider_config.compute_units_per_second,
    );

    let rpc_client = RpcClient::builder().layer(retry_layer).ws(ws).await?;

    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(rpc_client);

    let _ = provider.subscribe_blocks().await?;

    Ok(())
}

// <https://github.com/alloy-rs/alloy/issues/1601>
#[tokio::test]
async fn test_subscription_race_condition() -> Result<(), Box<dyn std::error::Error>> {
    use futures::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

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

    use alloy_provider::{Provider, ProviderBuilder};

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

// Verifies that basic auth credentials embedded in a WS URL are automatically
// extracted and sent as an Authorization header.
#[tokio::test]
async fn ws_basic_auth_from_url() -> Result<(), Box<dyn std::error::Error>> {
    use futures::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

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

// <https://github.com/alloy-rs/alloy/issues/2362>
#[tokio::test]
async fn ws_unsubscribe() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();
    let provider_config = ProviderConfig {
        endpoint: anvil.ws_endpoint(),
        max_rate_limit_retries: 1,
        initial_backoff: 50,
        compute_units_per_second: 1600,
    };

    let ws = WsConnect::new(provider_config.endpoint);
    let retry_layer = RetryBackoffLayer::new(
        provider_config.max_rate_limit_retries,
        provider_config.initial_backoff,
        provider_config.compute_units_per_second,
    );

    let rpc_client = RpcClient::builder().layer(retry_layer).ws(ws).await?;

    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(rpc_client);

    let sub = provider.subscribe_blocks().await?;
    let id = sub.local_id();

    provider.unsubscribe(*id).unwrap();

    Ok(())
}

// <https://github.com/alloy-rs/alloy/issues/3821>
// Server accepts the WS upgrade and then immediately sends a close frame.
// With a bounded retry budget the provider call should return Err instead of
// looping forever.
#[tokio::test]
async fn ws_close_frame_exhausts_retry_budget() {
    use futures::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { break };
            tokio::spawn(async move {
                let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
                    return;
                };
                // Read one request so the client actually sends something.
                let _ = ws.next().await;
                // Reply with a close frame carrying data, triggering the
                // error path in WsBackend::handle.
                let close = Message::Close(Some(CloseFrame {
                    code: CloseCode::Again,
                    reason: "service restart".into(),
                }));
                let _ = ws.send(close).await;
            });
        }
    });

    let ws = WsConnect::new(format!("ws://{addr}"))
        .with_max_retries(2)
        .with_retry_interval(std::time::Duration::from_millis(50));

    let rpc_client = RpcClient::builder().ws(ws).await;
    let Ok(rpc_client) = rpc_client else {
        // If the initial connect itself failed, that is acceptable.
        return;
    };
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(rpc_client);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        provider.get_block_number(),
    )
    .await;
    assert!(result.is_ok(), "get_block_number should not hang forever");
    if let Ok(inner) = result {
        assert!(inner.is_err(), "expected transport error from dying backend");
    }
}

// <https://github.com/alloy-rs/alloy/issues/3821>
// Server accepts the WS upgrade and echoes non-JSON-RPC text back. The
// deserialization failure in WsBackend::handle_text triggers close_with_error,
// which should be bounded by the new consecutive-death budget.
#[tokio::test]
async fn ws_invalid_text_exhausts_retry_budget() {
    use futures::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { break };
            tokio::spawn(async move {
                let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
                    return;
                };
                // Read the request, then send back garbage text that will
                // fail JSON-RPC deserialization.
                let _ = ws.next().await;
                let _ = ws.send(Message::Text("not valid json-rpc".into())).await;
            });
        }
    });

    let ws = WsConnect::new(format!("ws://{addr}"))
        .with_max_retries(2)
        .with_retry_interval(std::time::Duration::from_millis(50));

    let rpc_client = RpcClient::builder().ws(ws).await;
    let Ok(rpc_client) = rpc_client else {
        return;
    };
    let provider = ProviderBuilder::new().disable_recommended_fillers().connect_client(rpc_client);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        provider.get_block_number(),
    )
    .await;
    assert!(result.is_ok(), "get_block_number should not hang forever");
    if let Ok(inner) = result {
        assert!(inner.is_err(), "expected transport error from invalid text");
    }
}
