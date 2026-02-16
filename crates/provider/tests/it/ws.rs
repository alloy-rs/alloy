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
