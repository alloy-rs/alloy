# Alloy

Alloy connects applications to blockchains.

Alloy is a rewrite of [`ethers-rs`] from the ground up, with exciting new
features, high performance, and excellent [docs](https://alloy-rs.github.io/alloy/).

[`ethers-rs`] will continue to be maintained until we have achieved
feature-parity in Alloy. No action is currently needed from devs.

[![Telegram chat][telegram-badge]][telegram-url]

[`ethers-rs`]: https://github.com/gakonst/ethers-rs
[telegram-badge]: https://img.shields.io/endpoint?color=neon&style=for-the-badge&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Fethers_rs
[telegram-url]: https://t.me/ethers_rs

## Overview

This repository contains the following crates:

- [`alloy`]: Meta-crate for the entire project, including [`alloy-core`]
- [`alloy-consensus`] - Ethereum consensus interface
- [`alloy-contract`] - Interact with on-chain contracts
- [`alloy-eips`] - Ethereum Improvement Proprosal (EIP) implementations
- [`alloy-genesis`] - Ethereum genesis file definitions
- [`alloy-json-rpc`] - Core data types for JSON-RPC 2.0 clients
- [`alloy-network`] - Network abstraction for RPC types
- [`alloy-node-bindings`] - Ethereum execution-layer client bindings
- [`alloy-provider`] - Interface with an Ethereum blockchain
- [`alloy-pubsub`] - Ethereum JSON-RPC [publish-subscribe] tower service and type definitions
- [`alloy-rpc-client`] - Low-level Ethereum JSON-RPC client implementation
- [`alloy-rpc-types`] - Ethereum JSON-RPC types
  - [`alloy-rpc-types-engine`] - Ethereum execution-consensus layer (engine) API RPC types
  - [`alloy-rpc-types-trace`] - Ethereum RPC trace types
- [`alloy-signer`] - Ethereum signer abstraction
  - [`alloy-signer-aws`] - [AWS KMS] signer implementation
  - [`alloy-signer-gcp`] - [GCP KMS] signer implementation
  - [`alloy-signer-ledger`] - [Ledger] signer implementation
  - [`alloy-signer-trezor`] - [Trezor] signer implementation
- [`alloy-transport`] - Low-level Ethereum JSON-RPC transport abstraction
  - [`alloy-transport-http`] - HTTP transport implementation
  - [`alloy-transport-ipc`] - IPC transport implementation
  - [`alloy-transport-ws`] - WS transport implementation

[`alloy`]: https://github.com/alloy-rs/alloy/tree/main/crates/alloy
[`alloy-consensus`]: https://github.com/alloy-rs/alloy/tree/main/crates/consensus
[`alloy-contract`]: https://github.com/alloy-rs/alloy/tree/main/crates/contract
[`alloy-eips`]: https://github.com/alloy-rs/alloy/tree/main/crates/eips
[`alloy-genesis`]: https://github.com/alloy-rs/alloy/tree/main/crates/genesis
[`alloy-json-rpc`]: https://github.com/alloy-rs/alloy/tree/main/crates/json-rpc
[`alloy-network`]: https://github.com/alloy-rs/alloy/tree/main/crates/network
[`alloy-node-bindings`]: https://github.com/alloy-rs/alloy/tree/main/crates/node-bindings
[`alloy-provider`]: https://github.com/alloy-rs/alloy/tree/main/crates/provider
[`alloy-pubsub`]: https://github.com/alloy-rs/alloy/tree/main/crates/pubsub
[`alloy-rpc-client`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-client
[`alloy-rpc-types-engine`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-engine
[`alloy-rpc-types-trace`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types-trace
[`alloy-rpc-types`]: https://github.com/alloy-rs/alloy/tree/main/crates/rpc-types
[`alloy-signer`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer
[`alloy-signer-aws`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-aws
[`alloy-signer-gcp`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-gcp
[`alloy-signer-ledger`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-ledger
[`alloy-signer-trezor`]: https://github.com/alloy-rs/alloy/tree/main/crates/signer-trezor
[`alloy-transport`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport
[`alloy-transport-http`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport-http
[`alloy-transport-ipc`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport-ipc
[`alloy-transport-ws`]: https://github.com/alloy-rs/alloy/tree/main/crates/transport-ws
[`alloy-core`]: https://docs.rs/alloy-core
[publish-subscribe]: https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern
[AWS KMS]: https://aws.amazon.com/kms
[GCP KMS]: https://cloud.google.com/kms
[Ledger]: https://www.ledger.com
[Trezor]: https://trezor.io

## Supported Rust Versions

<!--
When updating this, also update:
- .clippy.toml
- Cargo.toml
- .github/workflows/ci.yml
-->

Alloy will keep a rolling MSRV (minimum supported rust version) policy of **at
least** 6 months. When increasing the MSRV, the new Rust version must have been
released at least six months ago. The current MSRV is 1.76.

Note that the MSRV is not increased automatically, and only as part of a minor
release.

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

- alloy-eips
- alloy-genesis
- alloy-serde
- alloy-consensus

If you would like to add `no_std` support to a crate, please make sure to update
`scripts/check_no_std.sh` as well.

## Credits

None of these crates would have been possible without the great work done in:

- [`ethers.js`](https://github.com/ethers-io/ethers.js/)
- [`rust-web3`](https://github.com/tomusdrw/rust-web3/)
- [`ruint`](https://github.com/recmo/uint)
- [`ethabi`](https://github.com/rust-ethereum/ethabi)
- [`ethcontract-rs`](https://github.com/gnosis/ethcontract-rs/)
- [`guac_rs`](https://github.com/althea-net/guac_rs/)

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
