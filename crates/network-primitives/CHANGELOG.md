# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.3](https://github.com/alloy-rs/alloy/releases/tag/v0.5.3) - 2024-10-22

### Miscellaneous Tasks

- Release 0.5.3

## [0.5.2](https://github.com/alloy-rs/alloy/releases/tag/v0.5.2) - 2024-10-18

### Miscellaneous Tasks

- Release 0.5.2

## [0.5.1](https://github.com/alloy-rs/alloy/releases/tag/v0.5.1) - 2024-10-18

### Miscellaneous Tasks

- Release 0.5.1

## [0.5.0](https://github.com/alloy-rs/alloy/releases/tag/v0.5.0) - 2024-10-18

### Bug Fixes

- Remove signature assoc type from tx response trait ([#1451](https://github.com/alloy-rs/alloy/issues/1451))

### Miscellaneous Tasks

- Release 0.5.0
- Some lifetime simplifications ([#1467](https://github.com/alloy-rs/alloy/issues/1467))

### Other

- Replace `to` by `kind` in Transaction trait ([#1484](https://github.com/alloy-rs/alloy/issues/1484))

## [0.4.2](https://github.com/alloy-rs/alloy/releases/tag/v0.4.2) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.2

## [0.4.1](https://github.com/alloy-rs/alloy/releases/tag/v0.4.1) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.1

## [0.4.0](https://github.com/alloy-rs/alloy/releases/tag/v0.4.0) - 2024-09-30

### Features

- [provider] Subscribe to new blocks if possible in heartbeat ([#1321](https://github.com/alloy-rs/alloy/issues/1321))
- Add getters into TransactionResponse and update implementations  ([#1328](https://github.com/alloy-rs/alloy/issues/1328))

### Miscellaneous Tasks

- Release 0.4.0

### Other

- Add supertrait alloy_consensus::Transaction to RPC TransactionResponse ([#1387](https://github.com/alloy-rs/alloy/issues/1387))
- Make `gas_limit` u64 for transactions ([#1382](https://github.com/alloy-rs/alloy/issues/1382))
- Make `Header` `base_fee_per_gas` u64 ([#1375](https://github.com/alloy-rs/alloy/issues/1375))
- Make `Header` gas limit u64 ([#1333](https://github.com/alloy-rs/alloy/issues/1333))

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Miscellaneous Tasks

- Release 0.3.6

## [0.3.5](https://github.com/alloy-rs/alloy/releases/tag/v0.3.5) - 2024-09-13

### Miscellaneous Tasks

- Release 0.3.5

## [0.3.4](https://github.com/alloy-rs/alloy/releases/tag/v0.3.4) - 2024-09-13

### Miscellaneous Tasks

- Release 0.3.4
- [network-primitives] Remove alloc Vec Dep ([#1267](https://github.com/alloy-rs/alloy/issues/1267))

### Other

- Add trait methods `cumulative_gas_used` and `state_root` to `ReceiptResponse` ([#1275](https://github.com/alloy-rs/alloy/issues/1275))

## [0.3.3](https://github.com/alloy-rs/alloy/releases/tag/v0.3.3) - 2024-09-10

### Miscellaneous Tasks

- Release 0.3.3

## [0.3.2](https://github.com/alloy-rs/alloy/releases/tag/v0.3.2) - 2024-09-09

### Features

- No_std network primitives ([#1248](https://github.com/alloy-rs/alloy/issues/1248))
- [network-primitives] Expose more fields via block response traits ([#1229](https://github.com/alloy-rs/alloy/issues/1229))

### Miscellaneous Tasks

- Release 0.3.2

### Other

- Add getter trait methods to `ReceiptResponse` ([#1251](https://github.com/alloy-rs/alloy/issues/1251))

## [0.3.1](https://github.com/alloy-rs/alloy/releases/tag/v0.3.1) - 2024-09-02

### Miscellaneous Tasks

- Release 0.3.1

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- Make `Block::hash` required ([#1205](https://github.com/alloy-rs/alloy/issues/1205))

### Features

- Network-parameterized block responses ([#1106](https://github.com/alloy-rs/alloy/issues/1106))

### Miscellaneous Tasks

- Release 0.3.0
- Release 0.2.1

### Refactor

- Add network-primitives ([#1101](https://github.com/alloy-rs/alloy/issues/1101))

[`alloy`]: https://crates.io/crates/alloy
[alloy]: https://crates.io/crates/alloy
[`alloy-core`]: https://crates.io/crates/alloy-core
[alloy-core]: https://crates.io/crates/alloy-core
[`alloy-consensus`]: https://crates.io/crates/alloy-consensus
[alloy-consensus]: https://crates.io/crates/alloy-consensus
[`alloy-contract`]: https://crates.io/crates/alloy-contract
[alloy-contract]: https://crates.io/crates/alloy-contract
[`alloy-eips`]: https://crates.io/crates/alloy-eips
[alloy-eips]: https://crates.io/crates/alloy-eips
[`alloy-genesis`]: https://crates.io/crates/alloy-genesis
[alloy-genesis]: https://crates.io/crates/alloy-genesis
[`alloy-json-rpc`]: https://crates.io/crates/alloy-json-rpc
[alloy-json-rpc]: https://crates.io/crates/alloy-json-rpc
[`alloy-network`]: https://crates.io/crates/alloy-network
[alloy-network]: https://crates.io/crates/alloy-network
[`alloy-node-bindings`]: https://crates.io/crates/alloy-node-bindings
[alloy-node-bindings]: https://crates.io/crates/alloy-node-bindings
[`alloy-provider`]: https://crates.io/crates/alloy-provider
[alloy-provider]: https://crates.io/crates/alloy-provider
[`alloy-pubsub`]: https://crates.io/crates/alloy-pubsub
[alloy-pubsub]: https://crates.io/crates/alloy-pubsub
[`alloy-rpc-client`]: https://crates.io/crates/alloy-rpc-client
[alloy-rpc-client]: https://crates.io/crates/alloy-rpc-client
[`alloy-rpc-types`]: https://crates.io/crates/alloy-rpc-types
[alloy-rpc-types]: https://crates.io/crates/alloy-rpc-types
[`alloy-rpc-types-anvil`]: https://crates.io/crates/alloy-rpc-types-anvil
[alloy-rpc-types-anvil]: https://crates.io/crates/alloy-rpc-types-anvil
[`alloy-rpc-types-beacon`]: https://crates.io/crates/alloy-rpc-types-beacon
[alloy-rpc-types-beacon]: https://crates.io/crates/alloy-rpc-types-beacon
[`alloy-rpc-types-engine`]: https://crates.io/crates/alloy-rpc-types-engine
[alloy-rpc-types-engine]: https://crates.io/crates/alloy-rpc-types-engine
[`alloy-rpc-types-eth`]: https://crates.io/crates/alloy-rpc-types-eth
[alloy-rpc-types-eth]: https://crates.io/crates/alloy-rpc-types-eth
[`alloy-rpc-types-trace`]: https://crates.io/crates/alloy-rpc-types-trace
[alloy-rpc-types-trace]: https://crates.io/crates/alloy-rpc-types-trace
[`alloy-serde`]: https://crates.io/crates/alloy-serde
[alloy-serde]: https://crates.io/crates/alloy-serde
[`alloy-signer`]: https://crates.io/crates/alloy-signer
[alloy-signer]: https://crates.io/crates/alloy-signer
[`alloy-signer-aws`]: https://crates.io/crates/alloy-signer-aws
[alloy-signer-aws]: https://crates.io/crates/alloy-signer-aws
[`alloy-signer-gcp`]: https://crates.io/crates/alloy-signer-gcp
[alloy-signer-gcp]: https://crates.io/crates/alloy-signer-gcp
[`alloy-signer-ledger`]: https://crates.io/crates/alloy-signer-ledger
[alloy-signer-ledger]: https://crates.io/crates/alloy-signer-ledger
[`alloy-signer-local`]: https://crates.io/crates/alloy-signer-local
[alloy-signer-local]: https://crates.io/crates/alloy-signer-local
[`alloy-signer-trezor`]: https://crates.io/crates/alloy-signer-trezor
[alloy-signer-trezor]: https://crates.io/crates/alloy-signer-trezor
[`alloy-signer-wallet`]: https://crates.io/crates/alloy-signer-wallet
[alloy-signer-wallet]: https://crates.io/crates/alloy-signer-wallet
[`alloy-transport`]: https://crates.io/crates/alloy-transport
[alloy-transport]: https://crates.io/crates/alloy-transport
[`alloy-transport-http`]: https://crates.io/crates/alloy-transport-http
[alloy-transport-http]: https://crates.io/crates/alloy-transport-http
[`alloy-transport-ipc`]: https://crates.io/crates/alloy-transport-ipc
[alloy-transport-ipc]: https://crates.io/crates/alloy-transport-ipc
[`alloy-transport-ws`]: https://crates.io/crates/alloy-transport-ws
[alloy-transport-ws]: https://crates.io/crates/alloy-transport-ws

<!-- generated by git-cliff -->
