//! Example of deploying a contract from an artifact to Anvil and interacting with it.

use alloy_network::EthereumSigner;
use alloy_node_bindings::Anvil;
use alloy_primitives::{U256, U64};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_signer::LocalWallet;
use alloy_sol_types::sol;
use alloy_transport_http::Http;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Codegen from artifact.
    sol!(
        #[sol(rpc)]
        Counter,
        "examples/artifacts/Counter.json"
    );

    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().spawn();

    // Set up wallet
    let wallet: LocalWallet = anvil.keys()[0].clone().into();

    // Create a provider with a signer and the network.
    let http = Http::<Client>::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::new()
        .signer(EthereumSigner::from(wallet))
        .provider(RootProvider::new(RpcClient::new(http, true)));

    println!("Anvil running at `{}`", anvil.endpoint());

    // Get the base fee for the block.
    let base_fee = provider.get_gas_price().await?;

    // Deploy the contract.
    let contract_builder = Counter::deploy_builder(&provider);
    let estimate = contract_builder.estimate_gas().await?;
    let contract_address =
        contract_builder.gas(estimate).gas_price(base_fee).nonce(U64::from(0)).deploy().await?;

    println!("Deployed contract at address: {:?}", contract_address);

    let contract = Counter::new(contract_address, &provider);

    let estimate = contract.setNumber(U256::from(42)).estimate_gas().await?;
    let builder =
        contract.setNumber(U256::from(42)).nonce(U64::from(1)).gas(estimate).gas_price(base_fee);
    let receipt = builder.send().await?.get_receipt().await?;

    println!("Set number to 42: {:?}", receipt.transaction_hash.unwrap());

    // Increment the number to 43.
    let estimate = contract.increment().estimate_gas().await?;
    let builder = contract.increment().nonce(U64::from(2)).gas(estimate).gas_price(base_fee);
    let receipt = builder.send().await?.get_receipt().await?;

    println!("Incremented number: {:?}", receipt.transaction_hash.unwrap());

    // Retrieve the number, which should be 43.
    let Counter::numberReturn { _0 } = contract.number().call().await?;

    println!("Retrieved number: {:?}", _0.to_string());

    Ok(())
}
