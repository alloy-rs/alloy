//! Example of signing and sending a transaction using a Trezor device.

use alloy_network::{Ethereum, EthereumSigner};
use alloy_primitives::{address, U256};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::request::TransactionRequest;
use alloy_signer_trezor::{TrezorHDPath, TrezorSigner};
use alloy_transport_http::Http;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Instantiate the application by acquiring a lock on the Trezor device.
    let signer = TrezorSigner::new(TrezorHDPath::TrezorLive(0), Some(1)).await?;

    // Create a provider with the signer and the network.
    let provider = ProviderBuilder::<_, Ethereum>::new()
        .signer(EthereumSigner::from(signer))
        .network::<Ethereum>()
        .provider(RootProvider::new(RpcClient::new(
            Http::new("http://localhost:8545".parse()?),
            true,
        )));

    // Create a transaction.
    let tx = TransactionRequest {
        value: Some(U256::from(100)),
        to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
        gas_price: Some(U256::from(20e9)),
        gas: Some(U256::from(21000)),
        ..Default::default()
    };

    // Send the transaction and wait for the receipt.
    // TODO: Confirmation count is not configurable yet.
    let pending_tx = provider.send_transaction(tx).await?;
    let receipt = pending_tx.await?;

    println!("Transaction receipt: {:?}", receipt);

    Ok(())
}
