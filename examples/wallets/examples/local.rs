//! # Local Wallet Example

use alloy_network::{Ethereum, EthereumSigner};
use alloy_node_bindings::Anvil;
use alloy_primitives::{address, U256, U64};
use alloy_providers::{Provider, ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::request::TransactionRequest;
use alloy_signer::LocalWallet;
use alloy_signer_ledger::{HDPath, LedgerSigner};
use alloy_transport_http::Http;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();

    let alice: LocalWallet = anvil.keys()[0].clone().into();
    let bob: LocalWallet = anvil.keys()[1].clone().into();

    let http = Http::new(anvil.endpoint().parse()?);

    // Create a provider with a signer and the network.
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
    // TODO: Confirmation count is not configurable yet.
    let pending_tx = provider.send_transaction(tx).await?;
    let receipt = pending_tx.await?;

    println!("Transaction receipt: {:?}", receipt);

    Ok(())
}
