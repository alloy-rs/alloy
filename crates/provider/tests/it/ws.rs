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

    let provider = ProviderBuilder::new().disable_recommended_fillers().on_client(rpc_client);

    let _ = provider.subscribe_blocks().await?;

    Ok(())
}
