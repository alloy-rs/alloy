# alloy-contract

Interact with on-chain contracts.

The main type is `CallBuilder`, which is a builder for constructing calls to on-chain contracts.
It provides a way to encode and decode data for on-chain calls, and to send those calls to the chain.
See its documentation for more details.

## Usage

Combined with the `sol!` macro's `#[sol(rpc)]` attribute, `CallBuilder` can be used to interact with
on-chain contracts. The `#[sol(rpc)]` attribute generates a method for each function in a contract
that returns a `CallBuilder` for that function. See its documentation for more details.

```rust,no_run
# async fn test() -> Result<(), Box<dyn std::error::Error>> {
use alloy_contract::SolCallBuilder;
use alloy_primitives::{Address, U256};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::sol;

sol! {
    #[sol(rpc)] // <-- Important! Generates the necessary `MyContract` struct and function methods.
    contract MyContract {
        #[derive(Debug)]
        function doStuff(uint a, bool b) public payable returns(address c, bytes32 d);
    }
}

// Configure a funded sender. `PRIVATE_KEY` must belong to an account funded on this node.
let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
let sender = signer.address();
let provider = ProviderBuilder::new()
    .wallet(signer)
    .connect("http://localhost:8545")
    .await?;

// Connect to an existing deployment.
let address: Address = std::env::var("CONTRACT_ADDRESS")?.parse()?;
let contract = MyContract::new(address, &provider);

// Build a call to the `doStuff` function and configure it.
let a = U256::from(123);
let b = true;
let call_builder = contract.doStuff(a, b).from(sender);

// Simulate the call with `eth_call`. This does not broadcast a transaction.
let call_return = call_builder.call().await?;
println!("{call_return:?}"); // doStuffReturn { c: 0x..., d: 0x... }

// Use `send` to broadcast the call as a transaction.
let _pending_tx = call_builder.send().await?;
# Ok(())
# }
```

When the `sol!` contract has real creation bytecode through `#[sol(bytecode = "0x...")]`, it also
generates a `deploy` method. Payable calls can attach wei with `CallBuilder::value`; set a deliberate
amount only when broadcasting.
