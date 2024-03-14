//! Example of generating code from ABI file to interact with the contract.

use alloy_network::Ethereum;
use alloy_node_bindings::Anvil;
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_sol_types::sol;
use alloy_transport_http::Http;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Codegen from ABI file to interact with the contract.
    sol!(
        #[sol(rpc)]
        IERC20,
        "examples/abi/IERC20.json"
    );

    // Spin up a forked Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().fork("https://eth.llamarpc.com").spawn();

    // Create a provider.
    let http = Http::<Client>::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::<_, Ethereum>::new()
        .provider(RootProvider::new(RpcClient::new(http, true)));

    // Create a contract instance.
    let contract = IERC20::new("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?, provider);

    // Call the contract, retrieve the total supply.
    let IERC20::totalSupplyReturn { _0 } = contract.totalSupply().call().await?;

    println!("WETH total supply is {_0}");

    Ok(())
}
