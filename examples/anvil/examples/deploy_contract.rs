use alloy_contract::SolCallBuilder;
use alloy_network::{Ethereum, EthereumSigner};
use alloy_node_bindings::Anvil;
use alloy_primitives::{Address, U256, U64};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_signer::LocalWallet;
use alloy_sol_types::sol;
use alloy_transport_http::Http;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().spawn();

    // Set up wallet
    let wallet: LocalWallet = anvil.keys()[0].clone().into();

    // Create a provider with a signer and the network.
    let http = Http::new(anvil.endpoint().parse()?);
    let provider = ProviderBuilder::<_, Ethereum>::new()
        .signer(EthereumSigner::from(wallet))
        .network::<Ethereum>()
        .provider(RootProvider::new(RpcClient::new(http, true)));

    println!("Anvil running at `{}`", anvil.endpoint());

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

    // Deploy the contract.
    let contract = Counter::new(Address::ZERO, &provider);

    // Get the base fee for the block.
    let base_fee = provider.get_gas_price().await?;

    println!("Setting number to 42...");

    // Set the number to 42.
    let estimate = contract.setNumber(U256::from(42)).estimate_gas().await?;
    let builder: SolCallBuilder<_, _, _, Counter::setNumberCall> =
        contract.setNumber(U256::from(42)).nonce(U64::from(0)).gas(estimate).gas_price(base_fee);
    builder.send().await?;

    println!("Incrementing number...");

    // Increment the number to 43.
    let estimate = contract.increment().estimate_gas().await?;
    let builder: SolCallBuilder<_, _, _, Counter::incrementCall> =
        contract.increment().nonce(U64::from(1)).gas(estimate).gas_price(base_fee);
    builder.send().await?;

    println!("Retrieving number...");

    // Retrieve the number, which should be 43.
    // Error: AbiError(SolTypes(Overrun))
    let estimate = contract.number().estimate_gas().await?;
    let builder = contract.number().gas(estimate).gas_price(base_fee);
    let Counter::numberReturn { _0 } = builder.call().await?;

    println!("Number: {:?}", _0);

    Ok(())
}
