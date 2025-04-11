# alloy-provider

<!-- TODO: links, docs, examples, etc -->

Interface with an Ethereum blockchain.

This crate contains the `Provider` trait, which exposes Ethereum JSON-RPC
methods. Providers in alloy are similar to [`ethers.js`] providers. They manage
an `RpcClient` and allow other parts of the program to easily make RPC calls.

Unlike an [`ethers.js`] Provider, an alloy Provider is network-aware. It is
parameterized with a `Network` from [`alloy-networks`]. This allows the Provider
to expose a consistent interface to the rest of the program, while adjusting
request and response types to match the underlying blockchain.

Providers can be composed via stacking. For example, a `Provider` that tracks
the nonce for a given address can be stacked onto a `Provider` that signs
transactions to create a `Provider` that can send signed transactions with
correct nonces.

The `ProviderBuilder` struct can quickly create a stacked provider, similar to
[`tower::ServiceBuilder`].

[alloy-networks]: ../networks/
[`tower::ServiceBuilder`]: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html
[`ethers.js`]: https://docs.ethers.org/v6/

## Feature flags

- `pubsub` - Enable support for subscription methods.
- `ws` - Enable WebSocket support. Implicitly enables `pubsub`.
- `ipc` - Enable IPC support. Implicitly enables `pubsub`.

## Usage

### Basic Provider

Create a simple provider to connect to an Ethereum node:

```rust
use alloy_provider::ProviderBuilder;
use alloy_primitives::Address;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to a node using HTTP
    let provider = ProviderBuilder::new().connect("http://localhost:8545").await?;
    
    // Query basic blockchain information
    let block_number = provider.get_block_number().await?;
    let balance = provider.get_balance(Address::ZERO, None).await?;
    
    println!("Current block: {block_number}");
    println!("Balance of address 0x0: {balance}");
    
    Ok(())
}
```

### Provider with Wallet

Create a provider with a wallet for signing transactions:

```rust
use alloy_provider::ProviderBuilder;
use alloy_primitives::{Address, U256};
use alloy_signer::PrivateKeySigner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a wallet from a private key
    let private_key = "0x..."; // Your private key
    let wallet = PrivateKeySigner::from_str(private_key)?;
    
    // Create a provider with the wallet, gas estimation and nonce management
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect("http://localhost:8545").await?;
    
    // Send a transaction
    let to = Address::from_str("0x...")?;
    let tx = provider.send_transaction()
        .to(to)
        .value(U256::from(1_000_000_000_000_000_000u64)) // 1 ETH
        .build();
    
    let tx_hash = provider.send_transaction(tx).await?;
    println!("Transaction sent: {tx_hash}");
    
    Ok(())
}
```

### Advanced Configuration

Customize your provider with specific features:

```rust
use alloy_provider::ProviderBuilder;
use alloy_chains::NamedChain;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a provider with specific configuration
    let provider = ProviderBuilder::new()
        // Specify chain
        .with_chain(NamedChain::Sepolia)
        // Add call batching for efficient RPC usage
        .with_call_batching()
        // Connect to the node
        .connect("https://sepolia.infura.io/v3/YOUR_API_KEY").await?;
    
    // Use the provider...
    
    Ok(())
}
```

### Local Development with Anvil

Use with a local Anvil instance for development:

```rust
use alloy_provider::ProviderBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Launch an Anvil instance with a configured wallet
    let provider = ProviderBuilder::new()
        .connect_anvil_with_wallet()
        // or configure Anvil:
        // .connect_anvil_with_config(|anvil| anvil.port(8555).mnemonic("test test..."))
        ;
    
    // The provider is now connected to a local Anvil instance
    // with pre-funded accounts
    
    Ok(())
}
```
