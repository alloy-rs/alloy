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

### Miscellaneous Tasks

- Release 0.5.0
- Flatten eip-7685 requests into a single opaque list ([#1383](https://github.com/alloy-rs/alloy/issues/1383))

## [0.4.2](https://github.com/alloy-rs/alloy/releases/tag/v0.4.2) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.2

## [0.4.1](https://github.com/alloy-rs/alloy/releases/tag/v0.4.1) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.1

## [0.4.0](https://github.com/alloy-rs/alloy/releases/tag/v0.4.0) - 2024-09-30

### Features

- Replace std/hashbrown with alloy_primitives::map ([#1384](https://github.com/alloy-rs/alloy/issues/1384))

### Miscellaneous Tasks

- Release 0.4.0

### Other

- Make `gas_limit` u64 for transactions ([#1382](https://github.com/alloy-rs/alloy/issues/1382))
- Add `BlockHeader` getter trait ([#1302](https://github.com/alloy-rs/alloy/issues/1302))

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Miscellaneous Tasks

- Release 0.3.6

### Refactor

- Separate transaction builders for tx types ([#1259](https://github.com/alloy-rs/alloy/issues/1259))

## [0.3.5](https://github.com/alloy-rs/alloy/releases/tag/v0.3.5) - 2024-09-13

### Miscellaneous Tasks

- Release 0.3.5

## [0.3.4](https://github.com/alloy-rs/alloy/releases/tag/v0.3.4) - 2024-09-13

### Features

- [alloy-rpc-types-eth] Optional serde ([#1276](https://github.com/alloy-rs/alloy/issues/1276))

### Miscellaneous Tasks

- Release 0.3.4

## [0.3.3](https://github.com/alloy-rs/alloy/releases/tag/v0.3.3) - 2024-09-10

### Miscellaneous Tasks

- Release 0.3.3

## [0.3.2](https://github.com/alloy-rs/alloy/releases/tag/v0.3.2) - 2024-09-09

### Miscellaneous Tasks

- Release 0.3.2

## [0.3.1](https://github.com/alloy-rs/alloy/releases/tag/v0.3.1) - 2024-09-02

### Miscellaneous Tasks

- Release 0.3.1

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- Make `Block::hash` required ([#1205](https://github.com/alloy-rs/alloy/issues/1205))
- Change generics order for `Block` ([#1192](https://github.com/alloy-rs/alloy/issues/1192))

### Features

- Make block struct generic over header type ([#1179](https://github.com/alloy-rs/alloy/issues/1179))
- Network-parameterized block responses ([#1106](https://github.com/alloy-rs/alloy/issues/1106))
- Add 7702 tx enum ([#1059](https://github.com/alloy-rs/alloy/issues/1059))

### Miscellaneous Tasks

- Release 0.3.0
- Remove async_trait from NetworkWallet ([#1160](https://github.com/alloy-rs/alloy/issues/1160))
- Add missing 7702 check ([#1137](https://github.com/alloy-rs/alloy/issues/1137))
- Release 0.2.1
- Re-export and document network-primitives ([#1107](https://github.com/alloy-rs/alloy/issues/1107))
- Release 0.2.0
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))

### Refactor

- Add network-primitives ([#1101](https://github.com/alloy-rs/alloy/issues/1101))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Features

- [network] Block context in ReceiptResponse ([#1003](https://github.com/alloy-rs/alloy/issues/1003))

### Miscellaneous Tasks

- Release 0.1.4

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Features

- [network] Add `input` method to `TransactionResponse` ([#959](https://github.com/alloy-rs/alloy/issues/959))

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

- Populate hashes after setting sidecar ([#648](https://github.com/alloy-rs/alloy/issues/648))
- Checking if the eip1559 gas fields are not set on eip2930 check ([#635](https://github.com/alloy-rs/alloy/issues/635))
- Signer filler now propagates missing keys from builder ([#637](https://github.com/alloy-rs/alloy/issues/637))
- Override txtype during submission prep ([#556](https://github.com/alloy-rs/alloy/issues/556))
- Allow empty `to` field in `can_build` ([#489](https://github.com/alloy-rs/alloy/issues/489))
- Automatically set blob versioned hashes if missing ([#409](https://github.com/alloy-rs/alloy/issues/409))
- Mandatory `to` on `TxEip4844` ([#355](https://github.com/alloy-rs/alloy/issues/355))
- Change nonce from `U64` to `u64`  ([#341](https://github.com/alloy-rs/alloy/issues/341))
- Add encode_for_signing to Transaction, fix Ledger sign_transaction ([#161](https://github.com/alloy-rs/alloy/issues/161))
- Hide __ENFORCE_ZST

### Dependencies

- Deduplicate AccessList and Withdrawals types ([#324](https://github.com/alloy-rs/alloy/issues/324))
- Alloy-consensus crate ([#83](https://github.com/alloy-rs/alloy/issues/83))

### Documentation

- Update signer documentation ([#180](https://github.com/alloy-rs/alloy/issues/180))
- Fix comment

### Features

- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- Add UnbuiltTransactionError type ([#878](https://github.com/alloy-rs/alloy/issues/878))
- [rpc] Split off `eth` namespace in `alloy-rpc-types` to `alloy-rpc-types-eth` ([#847](https://github.com/alloy-rs/alloy/issues/847))
- Add TransactionBuilder::apply ([#842](https://github.com/alloy-rs/alloy/issues/842))
- [network] Constrain `TransactionResponse` ([#835](https://github.com/alloy-rs/alloy/issues/835))
- WalletProvider ([#569](https://github.com/alloy-rs/alloy/issues/569))
- Refactor request builder workflow ([#431](https://github.com/alloy-rs/alloy/issues/431))
- Joinable transaction fillers ([#426](https://github.com/alloy-rs/alloy/issues/426))
- Helpers for AnyNetwork ([#476](https://github.com/alloy-rs/alloy/issues/476))
- `std` feature flag for `alloy-consensus` ([#461](https://github.com/alloy-rs/alloy/issues/461))
- Use AnyReceiptEnvelope for AnyNetwork ([#457](https://github.com/alloy-rs/alloy/issues/457))
- Add `AnyNetwork` ([#383](https://github.com/alloy-rs/alloy/issues/383))
- Extend TransactionBuilder with BlobTransactionSideCar setters ([#411](https://github.com/alloy-rs/alloy/issues/411))
- Move local signers to a separate crate, fix wasm ([#306](https://github.com/alloy-rs/alloy/issues/306))
- Network abstraction and transaction builder ([#190](https://github.com/alloy-rs/alloy/issues/190))
- Allow type-erased rpc client
- Generic request
- Manual future for json rpc to avoid higher-ranked lifetime
- DummyNetwork compile check
- Some cool combinators on rpccall

### Miscellaneous Tasks

- [clippy] Apply lint suggestions ([#903](https://github.com/alloy-rs/alloy/issues/903))
- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [docs] Add doc aliases ([#843](https://github.com/alloy-rs/alloy/issues/843))
- Fix warnings, check-cfg ([#776](https://github.com/alloy-rs/alloy/issues/776))
- [`signers`] Fix errors from primitives upgrade, avoid passing `B256` by val ([#152](https://github.com/alloy-rs/alloy/issues/152))

### Other

- Add `status` method to `ReceiptResponse` trait ([#846](https://github.com/alloy-rs/alloy/issues/846))
- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Use Self when possible ([#711](https://github.com/alloy-rs/alloy/issues/711))
- Use `From<Address>` for `TxKind` ([#651](https://github.com/alloy-rs/alloy/issues/651))
- Prestwich/signer multiplex ([#515](https://github.com/alloy-rs/alloy/issues/515))
- TypeTransaction conversion trait impls ([#472](https://github.com/alloy-rs/alloy/issues/472))
- TransactionList and BlockResponse ([#444](https://github.com/alloy-rs/alloy/issues/444))
- Numeric type audit: network, consensus, provider, rpc-types ([#454](https://github.com/alloy-rs/alloy/issues/454))
- Rename `alloy-providers` to `alloy-provider` ([#278](https://github.com/alloy-rs/alloy/issues/278))
- Some clippy and stuff
- Some middleware noodling
- Fuck jsonrpsee
- Mware and combinator stuff

### Refactor

- [signers] Use `signer` for single credentials and `wallet` for credential stores  ([#883](https://github.com/alloy-rs/alloy/issues/883))
- Remove `async_trait` in tx builder ([#279](https://github.com/alloy-rs/alloy/issues/279))
- More crate
- Transport future aliases
- Minor legibility
- Remove Params type from RpcCall
- More stuff
- RpcResult type

### Styling

- [Blocked] Update TransactionRequest's `to` field to TxKind ([#553](https://github.com/alloy-rs/alloy/issues/553))
- [Feature] Receipt trait in alloy-consensus ([#477](https://github.com/alloy-rs/alloy/issues/477))
- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))

### Testing

- Dummynet compile checks

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
