//! Example of deploying a contract from Solidity code to Anvil and interacting with it.

use alloy_network::{Ethereum, EthereumSigner};
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
    // Codegen from embedded Solidity code and precompiled bytecode.
    sol! {
        // solc v0.8.24; solc a.sol --via-ir --optimize --bin
        #[sol(rpc, bytecode="608080604052346100155760d2908161001a8239f35b5f80fdfe60808060405260043610156011575f80fd5b5f3560e01c9081633fb5c1cb1460865781638381f58a14606f575063d09de08a146039575f80fd5b34606b575f366003190112606b575f545f1981146057576001015f55005b634e487b7160e01b5f52601160045260245ffd5b5f80fd5b34606b575f366003190112606b576020905f548152f35b34606b576020366003190112606b576004355f5500fea2646970667358221220bdecd3c1dd631eb40587cafcd6e8297479db76db6a328e18ad1ea5b340852e3864736f6c63430008180033")]
        contract Counter {
            uint256 public number;

            function setNumber(uint256 newNumber) public {
                number = newNumber;
            }

            function increment() public {
                number++;
            }
        }
    }

    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().spawn();

    // Set up wallet
    let wallet: LocalWallet = anvil.keys()[0].clone().into();

    // Create a provider with a signer and the network.
    let http = Http::<Client>::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::<_, Ethereum>::new()
        .signer(EthereumSigner::from(wallet))
        .network::<Ethereum>()
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
