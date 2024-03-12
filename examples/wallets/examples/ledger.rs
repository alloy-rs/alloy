//! # Ledger Wallet Example

use alloy_network::Ethereum;
use alloy_providers::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Http::new("http://localhost:8545".parse()?);
    let client = RpcClient::new(transport, true);
    let provider = HttpProvider::<Ethereum>::new(client);

    let block_number = provider.get_block_number().await.unwrap();

    println!("Block number: {}", block_number);

    Ok(())
}
