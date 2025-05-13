# alloy-provider

[<img alt="github" src="https://img.shields.io/badge/github-alloy--rs/alloy-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/alloy-rs/alloy/tree/main/crates/provider)
[<img alt="crates.io" src="https://img.shields.io/crates/v/alloy-provider.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/alloy-provider)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-alloy--provider-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/alloy-provider)

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

[alloy-networks]: https://github.com/alloy-rs/alloy/tree/main/crates/network
[`tower::ServiceBuilder`]: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html
[`ethers.js`]: https://docs.ethers.org/v6/

## Feature flags

- `pubsub` - Enable support for subscription methods.
- `ws` - Enable WebSocket support. Implicitly enables `pubsub`.
- `ipc` - Enable IPC support. Implicitly enables `pubsub`.

## Usage

```rust,no_run
use alloy_provider::{ProviderBuilder, RootProvider, Provider};
use alloy_network::Ethereum;
use alloy_primitives::address;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a basic HTTP provider
    let provider = RootProvider::<Ethereum>::new_http("https://reth-ethereum.ithaca.xyz/rpc".parse()?);

    // Get the latest block number
    let block_number = provider.get_block_number().await?;
    println!("Latest block number: {block_number}");

    // Get balance of an address
    let address = address!("0x71C7656EC7ab88b098defB751B7401B5f6d8976F");
    let balance = provider.get_balance(address).await?;
    println!("Balance: {balance}");

    // Use the builder pattern to create a provider with recommended fillers
    let provider = ProviderBuilder::new().connect_http("https://reth-ethereum.ithaca.xyz/rpc".parse()?);

    Ok(())
}
```
