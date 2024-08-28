# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Dependencies

- Rm 2930 and 7702 - use alloy-rs/eips ([#1181](https://github.com/alloy-rs/alloy/issues/1181))

### Features

- Make signature methods generic over EncodableSignature ([#1138](https://github.com/alloy-rs/alloy/issues/1138))
- Add 7702 tx enum ([#1059](https://github.com/alloy-rs/alloy/issues/1059))
- Use EncodableSignature for tx encoding ([#1100](https://github.com/alloy-rs/alloy/issues/1100))
- [consensus] Add `From<ConsolidationRequest>` for `Request` ([#1083](https://github.com/alloy-rs/alloy/issues/1083))
- Expose encoded_len_with_signature() ([#1063](https://github.com/alloy-rs/alloy/issues/1063))
- Add 7702 tx type ([#1046](https://github.com/alloy-rs/alloy/issues/1046))
- Impl `arbitrary` for tx structs ([#1050](https://github.com/alloy-rs/alloy/issues/1050))

### Miscellaneous Tasks

- [consensus] Add missing getter trait methods for `alloy_consensus::Transaction` ([#1197](https://github.com/alloy-rs/alloy/issues/1197))
- Release 0.2.1
- Chore : fix typos ([#1087](https://github.com/alloy-rs/alloy/issues/1087))
- Release 0.2.0

### Other

- Add trait methods for constructing `alloy_rpc_types_eth::Transaction` to `alloy_consensus::Transaction` ([#1172](https://github.com/alloy-rs/alloy/issues/1172))
- Update TxType comment ([#1175](https://github.com/alloy-rs/alloy/issues/1175))
- Add payload length methods ([#1152](https://github.com/alloy-rs/alloy/issues/1152))
- `alloy-consensus` should use `alloy_primitives::Sealable` ([#1072](https://github.com/alloy-rs/alloy/issues/1072))

### Styling

- Remove proptest in all crates and Arbitrary derives ([#966](https://github.com/alloy-rs/alloy/issues/966))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Features

- Impl Transaction for TxEnvelope ([#1006](https://github.com/alloy-rs/alloy/issues/1006))

### Miscellaneous Tasks

- Release 0.1.4

### Other

- Remove signature.v parity before calculating tx hash ([#893](https://github.com/alloy-rs/alloy/issues/893))

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Documentation

- Copy/paste error of eip-7251 link ([#961](https://github.com/alloy-rs/alloy/issues/961))

### Features

- Add eip-7702 helpers ([#950](https://github.com/alloy-rs/alloy/issues/950))

### Miscellaneous Tasks

- Release 0.1.3
- [eips] Make `sha2` optional, add `kzg-sidecar` feature ([#949](https://github.com/alloy-rs/alloy/issues/949))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- Add eip-7251 consolidation request ([#919](https://github.com/alloy-rs/alloy/issues/919))

### Miscellaneous Tasks

- Release 0.1.2
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- Make test compile ([#873](https://github.com/alloy-rs/alloy/issues/873))
- Support pre-658 status codes ([#848](https://github.com/alloy-rs/alloy/issues/848))
- Add request mod back ([#796](https://github.com/alloy-rs/alloy/issues/796))
- Make eip-7685 req untagged ([#743](https://github.com/alloy-rs/alloy/issues/743))
- Account for requests root in header mem size ([#706](https://github.com/alloy-rs/alloy/issues/706))
- Add check before allocation in `SimpleCoder::decode_one()` ([#689](https://github.com/alloy-rs/alloy/issues/689))
- [consensus] `TxEip4844Variant::into_signed` RLP ([#596](https://github.com/alloy-rs/alloy/issues/596))
- Add more generics to any and receipt with bloom ([#559](https://github.com/alloy-rs/alloy/issues/559))
- Change `Header::nonce` to `B64` ([#485](https://github.com/alloy-rs/alloy/issues/485))
- Infinite loop while decoding a list of transactions ([#432](https://github.com/alloy-rs/alloy/issues/432))
- Mandatory `to` on `TxEip4844` ([#355](https://github.com/alloy-rs/alloy/issues/355))
- Use enveloped encoding for typed transactions ([#239](https://github.com/alloy-rs/alloy/issues/239))
- Add encode_for_signing to Transaction, fix Ledger sign_transaction ([#161](https://github.com/alloy-rs/alloy/issues/161))
- [`consensus`] Ensure into_signed forces correct format for eip1559/2930 txs ([#150](https://github.com/alloy-rs/alloy/issues/150))
- [`eips`/`consensus`] Correctly decode txs on `TxEnvelope` ([#148](https://github.com/alloy-rs/alloy/issues/148))
- [consensus] Correct TxType flag in EIP-2718 encoding ([#138](https://github.com/alloy-rs/alloy/issues/138))
- [`consensus`] Populate chain id when decoding signed legacy txs ([#137](https://github.com/alloy-rs/alloy/issues/137))

### Dependencies

- [deps] Update all dependencies ([#258](https://github.com/alloy-rs/alloy/issues/258))
- Alloy-consensus crate ([#83](https://github.com/alloy-rs/alloy/issues/83))

### Documentation

- Update descriptions and top level summary ([#128](https://github.com/alloy-rs/alloy/issues/128))

### Features

- Derive serde for header ([#902](https://github.com/alloy-rs/alloy/issues/902))
- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- Add as_ is_ functions to envelope ([#872](https://github.com/alloy-rs/alloy/issues/872))
- Put wasm-bindgen-futures dep behind the `wasm-bindgen` feature flag ([#795](https://github.com/alloy-rs/alloy/issues/795))
- [serde] Deprecate individual num::* for a generic `quantity` module ([#855](https://github.com/alloy-rs/alloy/issues/855))
- Feat(consensus) Add test for account  ([#801](https://github.com/alloy-rs/alloy/issues/801))
- Feat(consensus) implement RLP for Account information ([#789](https://github.com/alloy-rs/alloy/issues/789))
- [`provider`] `eth_getAccount` support ([#760](https://github.com/alloy-rs/alloy/issues/760))
- Derive proptest arbitrary for `Request` ([#732](https://github.com/alloy-rs/alloy/issues/732))
- Serde for `Request` ([#731](https://github.com/alloy-rs/alloy/issues/731))
- Derive arbitrary for `Request` ([#729](https://github.com/alloy-rs/alloy/issues/729))
- Rlp enc/dec for requests ([#728](https://github.com/alloy-rs/alloy/issues/728))
- [consensus, eips] EIP-7002 system contract ([#727](https://github.com/alloy-rs/alloy/issues/727))
- Add eth mainnet EL requests envelope ([#707](https://github.com/alloy-rs/alloy/issues/707))
- Add eip-7685 requests root to header ([#668](https://github.com/alloy-rs/alloy/issues/668))
- Use alloy types for BlobTransactionSidecar ([#673](https://github.com/alloy-rs/alloy/issues/673))
- Passthrough methods on txenvelope ([#598](https://github.com/alloy-rs/alloy/issues/598))
- Add the txhash getter. ([#574](https://github.com/alloy-rs/alloy/issues/574))
- Refactor request builder workflow ([#431](https://github.com/alloy-rs/alloy/issues/431))
- Export inner encoding / decoding functions from `Tx*` types ([#529](https://github.com/alloy-rs/alloy/issues/529))
- `std` feature flag for `alloy-consensus` ([#461](https://github.com/alloy-rs/alloy/issues/461))
- Receipt qol functions ([#459](https://github.com/alloy-rs/alloy/issues/459))
- Add AnyReceiptEnvelope ([#446](https://github.com/alloy-rs/alloy/issues/446))
- Embed primitives Log in rpc Log and consensus Receipt in rpc Receipt ([#396](https://github.com/alloy-rs/alloy/issues/396))
- Serde for consensus tx types ([#361](https://github.com/alloy-rs/alloy/issues/361))
- Re-export EnvKzgSettings ([#375](https://github.com/alloy-rs/alloy/issues/375))
- Versioned hashes without kzg ([#360](https://github.com/alloy-rs/alloy/issues/360))
- `impl TryFrom<Transaction> for TxEnvelope` ([#343](https://github.com/alloy-rs/alloy/issues/343))
- 4844 SidecarBuilder ([#250](https://github.com/alloy-rs/alloy/issues/250))
- Derive `Hash` for `TypedTransaction` ([#284](https://github.com/alloy-rs/alloy/issues/284))
- Network abstraction and transaction builder ([#190](https://github.com/alloy-rs/alloy/issues/190))
- [`consensus`] Add extra EIP-4844 types needed ([#229](https://github.com/alloy-rs/alloy/issues/229))
- [`alloy-consensus`] `EIP4844` tx support ([#185](https://github.com/alloy-rs/alloy/issues/185))

### Miscellaneous Tasks

- [clippy] Apply lint suggestions ([#903](https://github.com/alloy-rs/alloy/issues/903))
- Rm unused txtype mod ([#879](https://github.com/alloy-rs/alloy/issues/879))
- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [docs] Crate completeness and fix typos ([#861](https://github.com/alloy-rs/alloy/issues/861))
- [docs] Add doc aliases ([#843](https://github.com/alloy-rs/alloy/issues/843))
- Fix remaining warnings, add TODO for proptest-derive ([#819](https://github.com/alloy-rs/alloy/issues/819))
- [consensus] Re-export EIP-4844 transactions ([#777](https://github.com/alloy-rs/alloy/issues/777))
- Remove rlp encoding for `Request` ([#751](https://github.com/alloy-rs/alloy/issues/751))
- Move blob validation to sidecar ([#677](https://github.com/alloy-rs/alloy/issues/677))
- Clippy, warnings ([#504](https://github.com/alloy-rs/alloy/issues/504))
- Improve hyper http error messages ([#469](https://github.com/alloy-rs/alloy/issues/469))
- Dedupe blob in consensus and rpc ([#401](https://github.com/alloy-rs/alloy/issues/401))
- Clean up kzg and features ([#386](https://github.com/alloy-rs/alloy/issues/386))

### Other

- [Fix] use Eip2718Error, add docs on different encodings ([#869](https://github.com/alloy-rs/alloy/issues/869))
- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Update clippy warnings ([#765](https://github.com/alloy-rs/alloy/issues/765))
- Arbitrary Sidecar implementation + build. Closes [#680](https://github.com/alloy-rs/alloy/issues/680). ([#708](https://github.com/alloy-rs/alloy/issues/708))
- Use into instead of from ([#749](https://github.com/alloy-rs/alloy/issues/749))
- Correctly sign non legacy transaction without EIP155 ([#647](https://github.com/alloy-rs/alloy/issues/647))
- Some refactoring ([#739](https://github.com/alloy-rs/alloy/issues/739))
- Replace into_receipt by into ([#735](https://github.com/alloy-rs/alloy/issues/735))
- Replace into_tx by into ([#737](https://github.com/alloy-rs/alloy/issues/737))
- Use Self when possible ([#711](https://github.com/alloy-rs/alloy/issues/711))
- Use `From<Address>` for `TxKind` ([#651](https://github.com/alloy-rs/alloy/issues/651))
- Extension ([#474](https://github.com/alloy-rs/alloy/issues/474))
- TypeTransaction conversion trait impls ([#472](https://github.com/alloy-rs/alloy/issues/472))
- Mark envelopes non-exhaustive ([#456](https://github.com/alloy-rs/alloy/issues/456))
- Numeric type audit: network, consensus, provider, rpc-types ([#454](https://github.com/alloy-rs/alloy/issues/454))
- Check no_std in CI ([#367](https://github.com/alloy-rs/alloy/issues/367))

### Refactor

- Refactor around TxEip4844Variant ([#738](https://github.com/alloy-rs/alloy/issues/738))
- Clean up legacy serde helpers ([#624](https://github.com/alloy-rs/alloy/issues/624))

### Styling

- Make additional TxReceipt impls generic over T ([#617](https://github.com/alloy-rs/alloy/issues/617))
- [Feature] Receipt trait in alloy-consensus ([#477](https://github.com/alloy-rs/alloy/issues/477))
- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))
- Implement `arbitrary` for `TransactionReceipt` ([#449](https://github.com/alloy-rs/alloy/issues/449))

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
