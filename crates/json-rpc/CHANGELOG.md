# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- Allow arbitrary strings in subscription ids ([#1163](https://github.com/alloy-rs/alloy/issues/1163))
- [rpc] Show data in when cast send result in custom error ([#1129](https://github.com/alloy-rs/alloy/issues/1129))

### Features

- Add helper for decoding custom errors ([#1098](https://github.com/alloy-rs/alloy/issues/1098))
- [json-rpc] Implement `From<u64> for Id` and `From<String> for Id` ([#1088](https://github.com/alloy-rs/alloy/issues/1088))

### Miscellaneous Tasks

- Clippy für docs ([#1194](https://github.com/alloy-rs/alloy/issues/1194))
- JSON-RPC 2.0 spelling ([#1146](https://github.com/alloy-rs/alloy/issues/1146))
- Release 0.2.1
- Release 0.2.0

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Features

- [transport] Retry layer ([#849](https://github.com/alloy-rs/alloy/issues/849))

### Miscellaneous Tasks

- Release 0.1.4

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Miscellaneous Tasks

- Release 0.1.3

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Miscellaneous Tasks

- Release 0.1.2
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- Remove app-layer usage of transport error ([#363](https://github.com/alloy-rs/alloy/issues/363))
- Map deserde error to ErrorResp if it is an error payload ([#236](https://github.com/alloy-rs/alloy/issues/236))
- Deserialize EthNotification from params field ([#93](https://github.com/alloy-rs/alloy/issues/93))
- Remove the cow ([#34](https://github.com/alloy-rs/alloy/issues/34))
- Clippy
- Manually impl deser of pubsubitem
- Simplify deser_ok
- Remove unnecessary functions
- Lifetimes for rpc calls

### Dependencies

- Alloy-consensus crate ([#83](https://github.com/alloy-rs/alloy/issues/83))

### Documentation

- Fix some backticks
- Comments for deser impl
- Doc fix
- Note about not wanting this crate
- Nits
- Add readmes
- More of em

### Features

- [rpc] Trace requests and responses ([#498](https://github.com/alloy-rs/alloy/issues/498))
- Joinable transaction fillers ([#426](https://github.com/alloy-rs/alloy/issues/426))
- [json-rpc] Use `Cow` instead of `&'static str` for method names ([#319](https://github.com/alloy-rs/alloy/issues/319))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Subscription type ([#175](https://github.com/alloy-rs/alloy/issues/175))
- Signers ([#44](https://github.com/alloy-rs/alloy/issues/44))
- Interprocess-based IPC ([#59](https://github.com/alloy-rs/alloy/issues/59))
- Ws
- New pubsub
- Add RPC types + Add temporary bare `Provider` ([#13](https://github.com/alloy-rs/alloy/issues/13))
- Misc QoL
- SerializedRequest
- Docs note and try_as fns
- Eth-notification and expanded json-rpc
- Generic request
- RpcObject
- Separate rpc type crate

### Miscellaneous Tasks

- [clippy] Apply lint suggestions ([#903](https://github.com/alloy-rs/alloy/issues/903))
- Simplify some RpcCall code ([#470](https://github.com/alloy-rs/alloy/issues/470))
- Clean up Display impls ([#222](https://github.com/alloy-rs/alloy/issues/222))
- Correct doc typo ([#116](https://github.com/alloy-rs/alloy/issues/116))
- Add helper functions to ResponsePacket ([#115](https://github.com/alloy-rs/alloy/issues/115))
- Misc improvements ([#26](https://github.com/alloy-rs/alloy/issues/26))
- More lints and warns and errors
- Add warns and denies to some lib files
- Propagate generic error payload
- Improve id docs and ser
- CI and more rustdoc

### Other

- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Update clippy warnings ([#765](https://github.com/alloy-rs/alloy/issues/765))
- Small refactoring ([#724](https://github.com/alloy-rs/alloy/issues/724))
- Implement From<Response> and From<EthNotification> for PubSubItem ([#710](https://github.com/alloy-rs/alloy/issues/710))
- [Feature] Set subscription status on request and meta ([#576](https://github.com/alloy-rs/alloy/issues/576))
- Check no_std in CI ([#367](https://github.com/alloy-rs/alloy/issues/367))
- ClientRefs, Poller, and Streams ([#179](https://github.com/alloy-rs/alloy/issues/179))
- Various Subscription improvements ([#177](https://github.com/alloy-rs/alloy/issues/177))
- Use to_raw_value from serde_json ([#64](https://github.com/alloy-rs/alloy/issues/64))
- Avoid unnecessary serialize for RequestPacket. ([#61](https://github.com/alloy-rs/alloy/issues/61))
- Merge pull request [#21](https://github.com/alloy-rs/alloy/issues/21) from alloy-rs/prestwich/new-pubsub
- Match tuple order
- Merge pull request [#11](https://github.com/alloy-rs/alloy/issues/11) from alloy-rs/prestwich/new-new-transport
- Naming
- Merge pull request [#3](https://github.com/alloy-rs/alloy/issues/3) from alloy-rs/prestwich/readme-and-cleanup
- Merge pull request [#2](https://github.com/alloy-rs/alloy/issues/2) from alloy-rs/prestwich/transports

### Performance

- Don't collect or try_for_each in pubsub code ([#153](https://github.com/alloy-rs/alloy/issues/153))

### Refactor

- RpcError and RpcResult and TransportError and TransportResult ([#28](https://github.com/alloy-rs/alloy/issues/28))
- Update to use packets
- Deserialization of RpcResult
- Packets
- Response module
- Cow for jsonrpc params
- More crate

### Styling

- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))
- Sync with core ([#27](https://github.com/alloy-rs/alloy/issues/27))

### Testing

- Add deserde test for errorpayload with missing data ([#237](https://github.com/alloy-rs/alloy/issues/237))

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
