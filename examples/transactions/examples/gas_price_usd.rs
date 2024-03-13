use alloy_primitives::{address, utils::format_units, Address, Bytes, U256};
use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;
use std::str::FromStr;

sol!(
    #[derive(Debug)]
    function latestAnswer() external view returns (int256);
);
const ETH_USD_FEED: Address = address!("5f4eC3Df9cbd43714FE2740f5E3616155c5b8419");
const ETH_DECIMALS: u32 = 18;

#[tokio::main]
async fn main() -> Result<()> {
    let (provider, _anvil) = init();

    let call = latestAnswerCall {}.abi_encode();
    let input = Bytes::from(call);

    let tx = TransactionRequest {
        to: Some(ETH_USD_FEED),
        input: Some(input).into(),
        ..Default::default()
    };
    let res = provider.call(&tx, None).await?;

    let u = U256::from_str(res.to_string().as_str());

    let wei_per_gas = provider.get_gas_price().await?;

    let gwei = format_units(wei_per_gas, "gwei")?.parse::<f64>()?;

    let usd = usd_value(wei_per_gas, u.unwrap())?;

    println!("Gas price in Gwei: {}", gwei);
    println!("Gas price in USD: {}", usd);

    Ok(())
}

fn init() -> (HttpProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().fork("https://eth.llamarpc.com").spawn();
    let url = anvil.endpoint().parse().unwrap();
    let http = Http::<Client>::new(url);
    (HttpProvider::new(RpcClient::new(http, true)), anvil)
}

fn usd_value(amount: U256, price_usd: U256) -> Result<f64> {
    let base: U256 = U256::from(10).pow(U256::from(ETH_DECIMALS));
    let value: U256 = amount * price_usd / base;
    let usd_price_decimals: u8 = 8;
    let f: String = format_units(value, usd_price_decimals)?;
    Ok(f.parse::<f64>()?)
}
