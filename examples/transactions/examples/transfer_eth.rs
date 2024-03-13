use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{address, fixed_bytes, Address, U256};
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;
#[tokio::main]
async fn main() -> Result<()> {
    let (provider, _anvil) = init();

    // Transfer 1ETH from 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266 to Address::ZERO
    let input = fixed_bytes!("f865808477359400825208940000000000000000000000000000000000000000018082f4f5a00505e227c1c636c76fac55795db1a40a4d24840d81b40d2fe0cc85767f6bd202a01e91b437099a8a90234ac5af3cb7ca4fb1432e133f75f9a91678eaf5f487c74b");

    let tx = provider.send_raw_transaction(input.as_slice()).await?;
    let hash = tx.tx_hash();
    println!("Pending transaction hash: {}", hash);

    let transaction = provider.get_transaction_by_hash(hash.to_owned()).await?;

    assert_eq!(transaction.from, address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266"));
    assert_eq!(transaction.to, Some(Address::ZERO));
    assert_eq!(transaction.value, U256::from(1));

    Ok(())
}

fn init() -> (HttpProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().spawn();
    let url = anvil.endpoint().parse().unwrap();
    let http = Http::<Client>::new(url);
    (HttpProvider::new(RpcClient::new(http, true)), anvil)
}
