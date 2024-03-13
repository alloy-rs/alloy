use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::U256;
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_trace_types::parity::TraceType;
use alloy_rpc_types::{BlockId, BlockNumberOrTag, TransactionRequest};
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let (provider, anvil) = init();

    let from = anvil.addresses()[0];
    let to = anvil.addresses()[1];

    let tx_req = TransactionRequest {
        from: Some(from),
        to: Some(to),
        value: Some(U256::from(100)),
        ..Default::default()
    };
    let trace_type = [TraceType::Trace];
    let res = provider
        .trace_call(&tx_req, &trace_type, Some(BlockId::Number(BlockNumberOrTag::Latest)))
        .await?;

    println!("{:?}", res.trace);

    Ok(())
}

fn init() -> (HttpProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().fork("https://eth.merkle.io").spawn();
    let url = "https://eth.merkle.io".parse().unwrap(); // Use mainnet as anvil doesn't support trace_call
    let http = Http::<Client>::new(url);
    (HttpProvider::new(RpcClient::new(http, true)), anvil)
}
