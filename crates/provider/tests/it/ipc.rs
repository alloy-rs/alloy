use alloy_provider::{IpcConnect, Provider, ProviderBuilder};
use alloy_rpc_client::RpcClient;
use alloy_transport::layers::RetryBackoffLayer;
use tracing_subscriber::EnvFilter;

struct ProviderConfig {
    ipc: String,
    max_rate_limit_retries: u32,
    initial_backoff: u64,
    compute_units_per_second: u64,
}

// <https://github.com/alloy-rs/alloy/issues/1972>
#[tokio::test]
async fn ipc_retry_pubsub() -> Result<(), Box<dyn std::error::Error>> {
    let provider_config = ProviderConfig {
        ipc: "/tmp/reth.ipc".to_string(),
        max_rate_limit_retries: 1,
        initial_backoff: 50,
        compute_units_per_second: 1600,
    };

    let ipc_connect = IpcConnect::new(provider_config.ipc);
    let retry_layer = RetryBackoffLayer::new(
        provider_config.max_rate_limit_retries,
        provider_config.initial_backoff,
        provider_config.compute_units_per_second,
    );

    let rpc_client = RpcClient::builder().layer(retry_layer).ipc(ipc_connect).await?;

    let provider = ProviderBuilder::new().disable_recommended_fillers().on_client(rpc_client);

    let _ = provider.subscribe_blocks().await?;

    Ok(())
}
