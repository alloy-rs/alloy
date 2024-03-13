use alloy_network::Ethereum;
use alloy_node_bindings::Anvil;
use alloy_primitives::{Address, U256};
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_sol_types::sol;
use alloy_transport_http::Http;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().spawn();

    // Create a provider.
    let http = Http::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::<_, Ethereum>::new()
        .network::<Ethereum>()
        .provider(RootProvider::new(RpcClient::new(http, true)));

    // Create a contract.
    sol! {
        #[sol(rpc)]
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

    let contract = Counter::new(Address::ZERO, &provider);

    println!("Anvil running at `{}`", anvil.endpoint());

    println!("Setting the number to 42...");

    contract.setNumber(U256::from(42)).send().await?;

    println!("Incrementing the number...");

    contract.increment().send().await?;

    println!("Getting the number...");

    // TODO: currently throws Error: AbiError(SolTypes(Overrun))
    // let number = contract.number().call().await?;

    // println!("Number is now: {}", number._0);

    Ok(())
}
