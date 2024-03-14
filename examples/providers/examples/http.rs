use alloy_network::Ethereum;
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup the HTTP transport which is consumed by the RPC client
    let rpc_url = "https://eth.llamarpc.com".parse().unwrap();
    let http = Http::<Client>::new(rpc_url);

    // Create the RPC client
    let rpc_client = RpcClient::new(http, false);

    // Provider can then be instantiated using the RPC client, HttpProvider is an alias
    // RootProvider. RootProvider requires two generics N: Network and T: Transport
    let provider = HttpProvider::<Ethereum>::new(rpc_client);

    // Get latest block number
    let latest_block = provider.get_block_number().await?;

    println!("Latest block number: {}", latest_block);

    Ok(())
}
