# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- Use `server_id` when unsubscribing ([#1182](https://github.com/alloy-rs/alloy/issues/1182))
- Allow arbitrary strings in subscription ids ([#1163](https://github.com/alloy-rs/alloy/issues/1163))

### Miscellaneous Tasks

- Release 0.2.1
- Release 0.2.0
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Miscellaneous Tasks

- Release 0.1.4

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Miscellaneous Tasks

- Release 0.1.3
- Nightly clippy ([#947](https://github.com/alloy-rs/alloy/issues/947))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Miscellaneous Tasks

- Release 0.1.2
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- [pubsub] Handle subscription response on reconnects ([#105](https://github.com/alloy-rs/alloy/issues/105)) ([#107](https://github.com/alloy-rs/alloy/issues/107))

### Documentation

- Update descriptions and top level summary ([#128](https://github.com/alloy-rs/alloy/issues/128))
- Fix some backticks
- Resolve broken links

### Features

- [pubsub] Set channel size ([#602](https://github.com/alloy-rs/alloy/issues/602))
- [json-rpc] Use `Cow` instead of `&'static str` for method names ([#319](https://github.com/alloy-rs/alloy/issues/319))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Add `alloy` prelude crate ([#203](https://github.com/alloy-rs/alloy/issues/203))
- Subscription type ([#175](https://github.com/alloy-rs/alloy/issues/175))
- Signers ([#44](https://github.com/alloy-rs/alloy/issues/44))

### Miscellaneous Tasks

- [clippy] Apply lint suggestions ([#903](https://github.com/alloy-rs/alloy/issues/903))
- Clippy, warnings ([#504](https://github.com/alloy-rs/alloy/issues/504))
- Use `impl Future` in `PubSubConnect` ([#218](https://github.com/alloy-rs/alloy/issues/218))
- Simplify PubsubFrontend ([#168](https://github.com/alloy-rs/alloy/issues/168))
- Clean up tracing macro uses ([#154](https://github.com/alloy-rs/alloy/issues/154))
- Misc improvements ([#26](https://github.com/alloy-rs/alloy/issues/26))
- More lints and warns and errors
- Add warns and denies to some lib files
- Fix wasm

### Other

- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Update clippy warnings ([#765](https://github.com/alloy-rs/alloy/issues/765))
- ClientRefs, Poller, and Streams ([#179](https://github.com/alloy-rs/alloy/issues/179))
- Various Subscription improvements ([#177](https://github.com/alloy-rs/alloy/issues/177))
- Merge pull request [#21](https://github.com/alloy-rs/alloy/issues/21) from alloy-rs/prestwich/new-pubsub
- Clippy

### Performance

- Don't collect or try_for_each in pubsub code ([#153](https://github.com/alloy-rs/alloy/issues/153))

### Refactor

- RpcError and RpcResult and TransportError and TransportResult ([#28](https://github.com/alloy-rs/alloy/issues/28))
- Break transports into several crates

### Styling

- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))
- Clean up fmt::Debug impls ([#75](https://github.com/alloy-rs/alloy/issues/75))
- Sync with core ([#27](https://github.com/alloy-rs/alloy/issues/27))

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
