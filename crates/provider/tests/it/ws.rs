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
    async fn run_server() -> Result<std::net::SocketAddr, Box<dyn std::error::Error>> {
        use jsonrpsee::server::{RpcModule, Server, SubscriptionMessage};

        let server = Server::builder().build("127.0.0.1:0").await?;
        let mut module = RpcModule::new(());
        module
            .register_subscription(
                "subscribe_hello",
                "s_hello",
                "unsubscribe_hello",
                |_, pending, _, _| async move {
                    let sub = pending.accept().await.unwrap();

                    for i in 0..usize::MAX {
                        let raw = serde_json::value::to_raw_value(&i).unwrap();
                        let msg = SubscriptionMessage::from(raw);
                        sub.send(msg).await.unwrap();
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }

                    Ok(())
                },
            )
            .unwrap();
        let addr = server.local_addr()?;

        let handle = server.start(module);

        tokio::spawn(handle.stopped());

        Ok(addr)
    }
    use alloy_provider::{Provider, ProviderBuilder};

    let addr = run_server().await?;

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
