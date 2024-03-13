//! Example of using a local wallet to sign and broadcast a transaction on a local Anvil node.

use alloy_network::{Ethereum, EthereumSigner};
use alloy_node_bindings::Anvil;
use alloy_primitives::{U256, U64};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::request::TransactionRequest;
use alloy_signer::LocalWallet;
use alloy_transport_http::Http;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spin up an Anvil node.
    let anvil = Anvil::new().block_time(1).spawn();

    // Set up the wallets.
    let alice: LocalWallet = anvil.keys()[0].clone().into();
    let bob: LocalWallet = anvil.keys()[1].clone().into();

    // Create a provider with a signer and the network.
    let http = Http::<Client>::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::<_, Ethereum>::new()
        .signer(EthereumSigner::from(alice))
        .network::<Ethereum>()
        .provider(RootProvider::new(RpcClient::new(http, true)));

    // Create a transaction.
    let tx = TransactionRequest {
        value: Some(U256::from(100)),
        to: Some(bob.address()),
        nonce: Some(U64::from(0)),
        gas_price: Some(U256::from(20e9)),
        gas: Some(U256::from(21000)),
        ..Default::default()
    };

    // Broadcast the transaction and wait for the receipt.
    let receipt = provider.send_transaction(tx).await?.with_confirmations(1).get_receipt().await?;

    println!("Send transaction: {:?}", receipt.transaction_hash.unwrap());

    Ok(())
}
