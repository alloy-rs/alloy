# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1](https://github.com/alloy-rs/alloy/releases/tag/v0.2.1) - 2024-08-02

### Bug Fixes

- Trim conflicting key `max_fee_per_blob_gas` from Eip1559 tx type ([#1064](https://github.com/alloy-rs/alloy/issues/1064))

### Dependencies

- Bump jsonrpsee 0.24 ([#1067](https://github.com/alloy-rs/alloy/issues/1067))

### Features

- Eth_simulateV1 Request / Response types ([#1042](https://github.com/alloy-rs/alloy/issues/1042))
- Feat(rpc-type-eth) convert vec TxReq to bundle ([#1091](https://github.com/alloy-rs/alloy/issues/1091))
- Feat(provider) : introduction to eth_sendRawTransactionConditional  RPC endpoint type ([#1009](https://github.com/alloy-rs/alloy/issues/1009))
- [rpc-types-eth] Serde flatten `BlobTransactionSidecar` in tx req ([#1054](https://github.com/alloy-rs/alloy/issues/1054))
- Add authorization list to rpc transaction and tx receipt types ([#1051](https://github.com/alloy-rs/alloy/issues/1051))

### Miscellaneous Tasks

- Release 0.2.1
- [rpc] Make `Deserialize` impl for `FilterChanges` generic over transaction ([#1118](https://github.com/alloy-rs/alloy/issues/1118))
- Export rpc account type ([#1075](https://github.com/alloy-rs/alloy/issues/1075))
- Release 0.2.0
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))

### Other

- Add `AccessListResult` type (EIP-2930) ([#1110](https://github.com/alloy-rs/alloy/issues/1110))
- Derive arbitrary for `TransactionRequest` ([#1113](https://github.com/alloy-rs/alloy/issues/1113))
- Added stages to the sync info rpc type ([#1079](https://github.com/alloy-rs/alloy/issues/1079))

### Refactor

- Add network-primitives ([#1101](https://github.com/alloy-rs/alloy/issues/1101))
- Replace `U64` with `u64`  ([#1057](https://github.com/alloy-rs/alloy/issues/1057))

### Styling

- Remove proptest in all crates and Arbitrary derives ([#966](https://github.com/alloy-rs/alloy/issues/966))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Features

- Add helper to set both input and data fields ([#1019](https://github.com/alloy-rs/alloy/issues/1019))
- [rpc-types-eth] Add more utils to `TransactionIndex` ([#1007](https://github.com/alloy-rs/alloy/issues/1007))
- Add into transactions iterator ([#984](https://github.com/alloy-rs/alloy/issues/984))

### Miscellaneous Tasks

- Release 0.1.4
- Convert rcp-types-eth block Header to consensus Header ([#1014](https://github.com/alloy-rs/alloy/issues/1014))
- Make wrapped index value pub ([#988](https://github.com/alloy-rs/alloy/issues/988))

### Other

- Add range test in `FilterBlockOption` ([#939](https://github.com/alloy-rs/alloy/issues/939))

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Features

- Add eip-7702 helpers ([#950](https://github.com/alloy-rs/alloy/issues/950))
- [contract] Implement Filter's builder methods on Event ([#960](https://github.com/alloy-rs/alloy/issues/960))

### Miscellaneous Tasks

- Release 0.1.3
- Nightly clippy ([#947](https://github.com/alloy-rs/alloy/issues/947))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- Add `is_` and `as_` utils for `FilterBlockOption` ([#927](https://github.com/alloy-rs/alloy/issues/927))
- Add utils to `ValueOrArray` ([#924](https://github.com/alloy-rs/alloy/issues/924))
- Add `is_` utils to `FilterChanges` ([#923](https://github.com/alloy-rs/alloy/issues/923))

### Miscellaneous Tasks

- Release 0.1.2
- [rpc-types] Remove duplicate `Index` definition in `rpc-types-anvil` in favor of the one in `rpc-types-eth` ([#943](https://github.com/alloy-rs/alloy/issues/943))
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Dependencies

- [deps] Bump all ([#864](https://github.com/alloy-rs/alloy/issues/864))

### Features

- Integrate `EvmOverrides` to rpc types ([#906](https://github.com/alloy-rs/alloy/issues/906))
- Add getter methods for `FilterChanges` ([#899](https://github.com/alloy-rs/alloy/issues/899))
- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- [rpc] Split off `eth` namespace in `alloy-rpc-types` to `alloy-rpc-types-eth` ([#847](https://github.com/alloy-rs/alloy/issues/847))

### Miscellaneous Tasks

- Rm unused txtype mod ([#879](https://github.com/alloy-rs/alloy/issues/879))
- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [docs] Crate completeness and fix typos ([#861](https://github.com/alloy-rs/alloy/issues/861))

### Other

- Add custom conversion error to handle additional situations (such as optimism deposit tx) ([#875](https://github.com/alloy-rs/alloy/issues/875))
- Add receipt deserialize tests for `AnyTransactionReceipt` ([#868](https://github.com/alloy-rs/alloy/issues/868))

### Refactor

- [rpc] Extract `admin` and `txpool` into their respective crate ([#898](https://github.com/alloy-rs/alloy/issues/898))

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
