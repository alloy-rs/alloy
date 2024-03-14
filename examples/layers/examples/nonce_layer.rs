//! Example of using the `ManagedNonceLayer` in the provider.

use alloy_network::EthereumSigner;
use alloy_node_bindings::Anvil;
use alloy_primitives::{address, U256, U64};
use alloy_provider::{layers::ManagedNonceLayer, Provider, ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::request::TransactionRequest;
use alloy_signer::LocalWallet;
use alloy_transport_http::Http;
use reqwest::Client;

/// In Ethereum, the nonce of a transaction is a number that represents the number of transactions
/// that have been sent from a particular account. The nonce is used to ensure that transactions are
/// processed in the order they are intended, and to prevent the same transaction from being
/// processed multiple times.
///
/// The nonce manager in Alloy is a layer that helps you manage the nonce
/// of transactions by keeping track of the current nonce for a given account and automatically
/// incrementing it as needed. This can be useful if you want to ensure that transactions are sent
/// in the correct order, or if you want to avoid having to manually manage the nonce yourself.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().spawn();

    // Set up the wallets.
    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let from = wallet.address();

    // Create a provider with a signer and the network.
    let http = Http::<Client>::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::new()
        .layer(ManagedNonceLayer)
        .signer(EthereumSigner::from(wallet))
        .provider(RootProvider::new(RpcClient::new(http, true)));

    let tx = TransactionRequest {
        from: Some(from),
        value: Some(U256::from(100)),
        to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
        gas_price: Some(U256::from(20e9)),
        gas: Some(U256::from(21000)),
        ..Default::default()
    };

    let pending = provider.send_transaction(tx.clone()).await?;
    let tx_hash = pending.watch().await?;
    let mined_tx = provider.get_transaction_by_hash(tx_hash).await?;
    assert_eq!(mined_tx.nonce, U64::from(0));

    println!("Transaction sent with nonce: {}", mined_tx.nonce);

    let pending = provider.send_transaction(tx).await?;
    let tx_hash = pending.watch().await?;
    let mined_tx = provider.get_transaction_by_hash(tx_hash).await?;
    assert_eq!(mined_tx.nonce, U64::from(1));

    println!("Transaction sent with nonce: {}", mined_tx.nonce);

    Ok(())
}
