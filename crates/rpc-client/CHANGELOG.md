# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- [provider] Serialize no parameters as `[]` instead of `null` ([#1193](https://github.com/alloy-rs/alloy/issues/1193))
- Use `server_id` when unsubscribing ([#1182](https://github.com/alloy-rs/alloy/issues/1182))
- Use `BlockId` superset over `BlockNumberOrTag` where applicable  ([#1135](https://github.com/alloy-rs/alloy/issues/1135))

### Features

- [json-rpc] Implement `From<u64> for Id` and `From<String> for Id` ([#1088](https://github.com/alloy-rs/alloy/issues/1088))

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

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Miscellaneous Tasks

- Release 0.1.2
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- Correctly parse IPC sockets in builtin connections ([#522](https://github.com/alloy-rs/alloy/issues/522))
- Remove the cow ([#34](https://github.com/alloy-rs/alloy/issues/34))
- Dep tokio
- 1 url type
- Url in deps
- Cargo hack
- Turn ws off by default

### Dependencies

- [deps] Update to interprocess 2 ([#687](https://github.com/alloy-rs/alloy/issues/687))
- [deps] Update to hyper 1.0 ([#55](https://github.com/alloy-rs/alloy/issues/55))
- [deps] Update all dependencies ([#258](https://github.com/alloy-rs/alloy/issues/258))
- Alloy-consensus crate ([#83](https://github.com/alloy-rs/alloy/issues/83))

### Documentation

- Move rpc client from transport readme ([#782](https://github.com/alloy-rs/alloy/issues/782))
- Update descriptions and top level summary ([#128](https://github.com/alloy-rs/alloy/issues/128))

### Features

- Set poll interval based on connected chain ([#767](https://github.com/alloy-rs/alloy/issues/767))
- [pubsub] Set channel size ([#602](https://github.com/alloy-rs/alloy/issues/602))
- Bubble up set_subscription_status ([#581](https://github.com/alloy-rs/alloy/issues/581))
- [rpc] Trace requests and responses ([#498](https://github.com/alloy-rs/alloy/issues/498))
- [providers] Connect_boxed api ([#342](https://github.com/alloy-rs/alloy/issues/342))
- [json-rpc] Use `Cow` instead of `&'static str` for method names ([#319](https://github.com/alloy-rs/alloy/issues/319))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Add `alloy` prelude crate ([#203](https://github.com/alloy-rs/alloy/issues/203))
- Subscription type ([#175](https://github.com/alloy-rs/alloy/issues/175))
- Add `alloy-node-bindings` ([#111](https://github.com/alloy-rs/alloy/issues/111))
- Temporary provider trait ([#20](https://github.com/alloy-rs/alloy/issues/20))
- Interprocess-based IPC ([#59](https://github.com/alloy-rs/alloy/issues/59))

### Miscellaneous Tasks

- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [general] Add CI workflow for Windows + fix IPC test ([#642](https://github.com/alloy-rs/alloy/issues/642))
- Rm PathBuf import ([#533](https://github.com/alloy-rs/alloy/issues/533))
- Simplify some RpcCall code ([#470](https://github.com/alloy-rs/alloy/issues/470))
- Rename `RpcClient::prepare` to `request` ([#299](https://github.com/alloy-rs/alloy/issues/299))
- Use `impl Future` in `PubSubConnect` ([#218](https://github.com/alloy-rs/alloy/issues/218))
- Clean up tracing macro uses ([#154](https://github.com/alloy-rs/alloy/issues/154))
- Misc improvements ([#26](https://github.com/alloy-rs/alloy/issues/26))
- More lints and warns and errors
- Add warns and denies to more lib files

### Other

- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Exporting waiter struct from batch ([#773](https://github.com/alloy-rs/alloy/issues/773))
- RpcWithBlock ([#674](https://github.com/alloy-rs/alloy/issues/674))
- [Refactor] Delete the internal-test-utils crate ([#632](https://github.com/alloy-rs/alloy/issues/632))
- Configure polling interval ([#437](https://github.com/alloy-rs/alloy/issues/437))
- Removed reqwest prefix ([#462](https://github.com/alloy-rs/alloy/issues/462))
- Adds `check -Zcheck-cfg ` job ([#419](https://github.com/alloy-rs/alloy/issues/419))
- ClientRefs, Poller, and Streams ([#179](https://github.com/alloy-rs/alloy/issues/179))
- Various Subscription improvements ([#177](https://github.com/alloy-rs/alloy/issues/177))
- Merge pull request [#21](https://github.com/alloy-rs/alloy/issues/21) from alloy-rs/prestwich/new-pubsub
- Clippy
- Temporarily comment out tests

### Performance

- Don't collect or try_for_each in pubsub code ([#153](https://github.com/alloy-rs/alloy/issues/153))

### Refactor

- Change u64 to Duration ([#636](https://github.com/alloy-rs/alloy/issues/636))
- RpcError and RpcResult and TransportError and TransportResult ([#28](https://github.com/alloy-rs/alloy/issues/28))
- Break transports into several crates

### Styling

- Use poll loop for CallState ([#779](https://github.com/alloy-rs/alloy/issues/779))
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
