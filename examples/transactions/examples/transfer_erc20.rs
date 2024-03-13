use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;

sol!(ERC20Example, "examples/contracts/ERC20Example.json");
#[tokio::main]
async fn main() -> Result<()> {
    let (provider, anvil) = init();

    let from = anvil.addresses()[0];

    let contract_address = deploy_token_contract(&provider, from).await?;

    println!("Deployed contract at: {}", contract_address);

    let to = anvil.addresses()[1];

    let input = ERC20Example::transferCall { to, amount: U256::from(100) }.abi_encode();
    // Convert to Bytes
    let input = Bytes::from(input);

    let transfer_tx = TransactionRequest {
        from: Some(from),
        to: Some(contract_address),
        input: Some(input).into(),
        ..Default::default()
    };

    let pending_tx = provider.send_transaction(transfer_tx).await?;

    let tx_hash = pending_tx.tx_hash().to_owned();

    let _tx = provider.get_transaction_by_hash(tx_hash).await?;

    let to_bal = balance_of(&provider, to, contract_address).await?;
    let from_bal = balance_of(&provider, from, contract_address).await?;

    assert_eq!(to_bal, U256::from(100));
    assert_eq!(from_bal, U256::from(999999999999999999900_i128));

    Ok(())
}

fn init() -> (HttpProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().spawn();
    let url = anvil.endpoint().parse().unwrap();
    let http = Http::<Client>::new(url);
    (HttpProvider::new(RpcClient::new(http, true)), anvil)
}

async fn deploy_token_contract(
    provider: &HttpProvider<Ethereum>,
    from: Address,
) -> Result<Address> {
    // Compile the contract
    let bytecode = ERC20Example::BYTECODE.to_owned();

    let tx_req = TransactionRequest {
        from: Some(from),
        input: Some(bytecode).into(),
        to: None,
        ..Default::default()
    };

    // Deploy the contract
    let pending_tx = provider.send_transaction(tx_req).await?;

    // Wait for the transaction to be mined
    let _tx = provider.get_transaction_by_hash(pending_tx.tx_hash().to_owned()).await?;

    // Get receipt
    let receipt = provider.get_transaction_receipt(pending_tx.tx_hash().to_owned()).await?;

    let contract_address = receipt.unwrap().contract_address.unwrap();
    Ok(contract_address)
}

async fn balance_of(
    provider: &HttpProvider<Ethereum>,
    account: Address,
    contract_address: Address,
) -> Result<U256> {
    let call = ERC20Example::balanceOfCall { account }.abi_encode();
    let input = Bytes::from(call);

    let tx_req = TransactionRequest {
        to: Some(contract_address),
        input: Some(input).into(),
        ..Default::default()
    };

    let result = provider.call(&tx_req, None).await?;
    let result = ERC20Example::balanceOfCall::abi_decode_returns(&result, true)?;
    Ok(result._0)
}
