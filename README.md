# Alloy

Alloy connects applications to blockchains.

Alloy is a rewrite of [`ethers-rs`] from the ground up, with exciting new
features, high performance, and excellent [docs](https://docs.rs/alloy).

We also have a [book](https://alloy.rs/) on all things Alloy and many [examples](https://github.com/alloy-rs/examples) to help you get started.

[![Telegram chat][telegram-badge]][telegram-url]

[`ethers-rs`]: https://github.com/gakonst/ethers-rs
[telegram-badge]: https://img.shields.io/endpoint?color=neon&style=for-the-badge&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Fethers_rs
[telegram-url]: https://t.me/ethers_rs

## Installation

Alloy consists of a number of crates that provide a range of functionality essential for interfacing with any Ethereum-based blockchain.

The easiest way to get started is to add the `alloy` crate with the `full` feature flag from the command-line using Cargo:

```sh
cargo add alloy --features full
```

Alternatively, you can add the following to your `Cargo.toml` file:

```toml
alloy = { version = "1", features = ["full"] }
```

For a more fine-grained control over the features you wish to include, you can add the individual crates to your `Cargo.toml` file, or use the `alloy` crate with the features you need.

A comprehensive list of available features can be found on [docs.rs](https://docs.rs/crate/alloy/latest/features) or in the [`alloy` crate's `Cargo.toml`](https://github.com/alloy-rs/alloy/blob/main/crates/alloy/Cargo.toml).

## Examples

### Connecting to a Provider

Here's a simple example of connecting to an Ethereum node and querying the latest block:

```rust
use alloy::providers::{Provider, ProviderBuilder};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
// Create a provider with the HTTP transport using the `reqwest` crate.
let rpc_url = "https://eth.llamarpc.com";
let provider = ProviderBuilder::new().connect(rpc_url).await?;

// Get the latest block number.
let latest_block = provider.get_block_number().await?;
println!("Latest block number: {latest_block}");

// Get chain ID.
let chain_id = provider.get_chain_id().await?;
println!("Chain ID: {chain_id}");
# Ok(())
# }
```

### Network generic

Alloy is network-generic, allowing you to work with any Ethereum-compatible chain. Here's an example using Optimism (see [`op-alloy`](https://docs.rs/op-alloy)) to demonstrate this capability:

```rust,ignore
use alloy::providers::{Provider, ProviderBuilder};
use op_alloy::network::Optimism;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
// Connect to Optimism mainnet.
let rpc_url = "https://mainnet.optimism.io";
let provider = ProviderBuilder::new_with_network::<Optimism>().connect(rpc_url).await?;
# Ok(())
# }
```

For more examples, check out the [Alloy examples repository](https://github.com/alloy-rs/examples).

## Overview

This repository contains the following crates:

- [`alloy`]: Meta-crate for the entire project, including [`alloy-core`]
- [`alloy-consensus`] - Ethereum consensus interface
  - [`alloy-consensus-any`] - Catch-all consensus interface for multiple networks
- [`alloy-contract`] - Interact with on-chain contracts
- [`alloy-eip5792`] - Types for the `wallet_` Ethereum JSON-RPC namespace
- [`alloy-eip7547`] - EIP-7547: Inclusion Lists types
- [`alloy-eips`] - Ethereum Improvement Proposal (EIP) implementations
- [`alloy-genesis`] - Ethereum genesis file definitions
- [`alloy-json-rpc`] - Core data types for JSON-RPC 2.0 clients
- [`alloy-ens`] - Ethereum Name Service (ENS) utilities
- [`alloy-network`] - Network abstraction for RPC types
  - [`alloy-network-primitives`] - Primitive types for the network abstraction
- [`alloy-node-bindings`] - Ethereum execution-layer client bindings
- [`alloy-provider`] - Interface with an Ethereum blockchain
- [`alloy-pubsub`] - Ethereum JSON-RPC [publish-subscribe] tower service and type definitions
- [`alloy-rpc-client`] - Low-level Ethereum JSON-RPC client implementation
- [`alloy-rpc-types`] - Meta-crate for all Ethereum JSON-RPC types
  - [`alloy-rpc-types-admin`] - Types for the `admin` Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-anvil`] - Types for the [Anvil] development node's Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-any`] - Types for JSON-RPC namespaces across multiple networks
  - [`alloy-rpc-types-beacon`] - Types for the [Ethereum Beacon Node API][beacon-apis]
  - [`alloy-rpc-types-debug`] - Types for the `debug` Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-engine`] - Types for the `engine` Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-eth`] - Types for the `eth` Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-mev`] - Types for the MEV bundle JSON-RPC namespace
  - [`alloy-rpc-types-tenderly`] - Types for the Tenderly node's Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-trace`] - Types for the `trace` Ethereum JSON-RPC namespace
  - [`alloy-rpc-types-txpool`] - Types for the `txpool` Ethereum JSON-RPC namespace
- [`alloy-serde`] - [Serde]-related utilities
- [`alloy-signer`] - Ethereum signer abstraction
  - [`alloy-signer-aws`] - [AWS KMS] signer implementation
  - [`alloy-signer-gcp`] - [GCP KMS] signer implementation
  - [`alloy-signer-ledger`] - [Ledger] signer implementation
  - [`alloy-signer-local`] - Local (private key, keystore, mnemonic, YubiHSM) signer implementations
  - [`alloy-signer-trezor`] - [Trezor] signer implementation
  - [`alloy-signer-turnkey`] - [Turnkey] signer implementation
- [`alloy-transport`] - Low-level Ethereum JSON-RPC transport abstraction
  - [`alloy-transport-http`] - HTTP transport implementation
  - [`alloy-transport-ipc`] - IPC transport implementation
  - [`alloy-transport-ws`] - WS transport implementation
- [`alloy-tx-macros`] - Derive macro for transaction envelopes

[`alloy`]: https://github.com/alloy-rs/alloy/tree/main/crates/alloy
[`alloy-core`]: https://docs.rs/alloy-core
[`alloy-consensus`]: https://github.com/alloy-rs/alloy/tree/main/crates/consensus
[`alloy-consensus-any`]: https://github.com/alloy-rs/alloy/tree/main/crates/consensus-any
[`alloy-contract`]: https://github.com/alloy-rs/alloy/tree/main/crates/contract
[`alloy-eip5792`]: https://github.com/alloy-rs/alloy/tree/main/crates/eip5792
[`alloy-eip7547`]: https://github.com/alloy-rs/alloy/tree/main/crates/eip7547
[`alloy-eips`]: https://github.com/alloy-rs/alloy/tree/main/crates/eips
[`alloy-genesis`]: https://github.com/alloy-rs/alloy/tree/main/crates/genesis
[`alloy-json-rpc`]: https://github.com/alloy-rs/alloy/tree/main/crates/json-rpc
[`alloy-network`]: https://github.com/alloy-rs/alloy/tree/main/crates/network
[`alloy-network-primitives`]: https://github.com/alloy-rs/alloy/tree/main/crates/network-primitives
[`alloy-node-bindings`]: https://github.com/alloy-rs/alloy/tree/main/crates/node-bindings
[`alloy-provider`]: https://github.com/alloy-rs/alloy/tree/main/crates/provider
[`alloy-pubsub`]: https://github.com/alloy-rs/alloy/tree/main/crates/pubsub
[`alloy-rpc-client`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-client
[`alloy-rpc-types`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types
[`alloy-rpc-types-admin`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-admin
[`alloy-rpc-types-anvil`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-anvil
[`alloy-rpc-types-any`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-any
[`alloy-rpc-types-beacon`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-beacon
[`alloy-rpc-types-debug`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-debug
[`alloy-rpc-types-engine`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-engine
[`alloy-rpc-types-eth`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-eth
[`alloy-rpc-types-mev`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-mev
[`alloy-rpc-types-tenderly`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-tenderly
[`alloy-rpc-types-trace`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-trace
[`alloy-rpc-types-txpool`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-txpool
[`alloy-serde`]: https://github.com/alloy-rs/alloy/tree/main/crates/serde
[`alloy-signer`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer
[`alloy-signer-aws`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-aws
[`alloy-signer-gcp`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-gcp
[`alloy-signer-ledger`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-ledger
[`alloy-signer-local`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-local
[`alloy-signer-trezor`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-trezor
[`alloy-signer-turnkey`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-turnkey
[`alloy-transport`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport
[`alloy-transport-http`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport-http
[`alloy-transport-ipc`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport-ipc
[`alloy-transport-ws`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport-ws
[`alloy-tx-macros`]: https://github.com/alloy-rs/alloy/tree/main/crates/tx-macros
[`alloy-ens`]: https://github.com/alloy-rs/alloy/tree/main/crates/ens
[publish-subscribe]: https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern
[AWS KMS]: https://aws.amazon.com/kms
[GCP KMS]: https://cloud.google.com/kms
[Ledger]: https://www.ledger.com
[Trezor]: https://trezor.io
[Turnkey]: https://www.turnkey.com
[Serde]: https://serde.rs
[beacon-apis]: https://ethereum.github.io/beacon-APIs
[Anvil]: https://github.com/foundry-rs/foundry

## Supported Rust Versions (MSRV)

<!--
When updating this, also update:
- clippy.toml
- Cargo.toml
- .github/workflows/ci.yml
-->

The current MSRV (minimum supported rust version) is 1.88.

Alloy will keep a rolling MSRV policy of **at least** two versions behind the
latest stable release (so if the latest stable release is 1.58, we would
support 1.56).

Note that the MSRV is not increased automatically, and only as part of a patch
(pre-1.0) or minor (post-1.0) release.

## Contributing

Thanks for your help improving the project! We are so happy to have you! We have
[a contributing guide](./CONTRIBUTING.md) to help you get involved in the
Alloy project.

Pull requests will not be merged unless CI passes, so please ensure that your
contribution follows the linting rules and passes clippy.

## Note on `no_std`

Because these crates are primarily network-focused, we do not intend to support
`no_std` for most of them at this time.

The following crates support `no_std`:

| Crate               | Version Badge                                                                                                 |
| ------------------- | ------------------------------------------------------------------------------------------------------------- |
| **alloy-eips**      | [![Crates.io](https://img.shields.io/crates/v/alloy-eips.svg)](https://crates.io/crates/alloy-eips)           |
| **alloy-genesis**   | [![Crates.io](https://img.shields.io/crates/v/alloy-genesis.svg)](https://crates.io/crates/alloy-genesis)     |
| **alloy-serde**     | [![Crates.io](https://img.shields.io/crates/v/alloy-serde.svg)](https://crates.io/crates/alloy-serde)         |
| **alloy-consensus** | [![Crates.io](https://img.shields.io/crates/v/alloy-consensus.svg)](https://crates.io/crates/alloy-consensus) |

If you would like to add `no_std` support to a crate, please make sure to update
`scripts/check_no_std.sh` as well.

## Credits

None of these crates would have been possible without the great work done in:

| Project                                                       | Description                                                                                    |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| [`ethers.js`](https://github.com/ethers-io/ethers.js/)        | A complete and compact JavaScript library for interacting with the Ethereum blockchain.        |
| [`rust-web3`](https://github.com/tomusdrw/rust-web3/)         | Rust library for Ethereum JSON-RPC client communication, including support for async and WASM. |
| [`ruint`](https://github.com/recmo/uint)                      | A fast, no-std, const-friendly implementation of fixed-size unsigned integers in Rust.         |
| [`ethabi`](https://github.com/rust-ethereum/ethabi)           | Ethereum ABI encoding/decoding in Rust for contracts and transactions.                         |
| [`ethcontract-rs`](https://github.com/gnosis/ethcontract-rs/) | Rust library to generate type-safe bindings to Ethereum smart contracts.                       |
| [`guac_rs`](https://github.com/althea-net/guac_rs/)           | Rust implementation of the GUAC protocol for Ethereum state channels.                          |

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in these crates by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>
