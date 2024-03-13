use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{address, fixed_bytes};
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::Filter;
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;
#[tokio::main]
async fn main() -> Result<()> {
    let provider = init();

    // Get logs from the latest block
    let latest_block = provider.get_block_number().await?;
    let filter = Filter::new().from_block(latest_block);
    // .address(vec![address!("1f9840a85d5aF5bf1D1762F925BDADdC4201F984")]) // Emitted by the
    // UNI token
    // .event("Transfer(address,address,uint256)")
    // .event_signature(fixed_bytes!("
    // ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")); // Using transfer event
    // signature
    let logs = provider.get_logs(&filter).await?;

    for log in logs {
        println!("{:?}", log);
    }
    Ok(())
}

fn init() -> HttpProvider<Ethereum> {
    let http = Http::<Client>::new("https://eth.llamarpc.com".parse().unwrap());
    HttpProvider::new(RpcClient::new(http, true))
}
