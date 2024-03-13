use alloy_network::Ethereum;
use alloy_primitives::address;
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::{BlockId, BlockNumberOrTag};
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = init();

    // Get bytecode of USDC-ETH Uniswap V3 pool
    let pool_address = address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640");

    let bytecode =
        provider.get_code_at(pool_address, BlockId::Number(BlockNumberOrTag::Latest)).await?;

    println!("Bytecode: {:?}", bytecode);

    Ok(())
}

fn init() -> HttpProvider<Ethereum> {
    let http = Http::<Client>::new("https://eth.llamarpc.com".parse().unwrap());
    HttpProvider::new(RpcClient::new(http, true))
}
