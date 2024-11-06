# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.3](https://github.com/alloy-rs/alloy/releases/tag/v0.5.3) - 2024-10-22

### Bug Fixes

- Correct implementations of Encodable and Decodable for sidecars ([#1528](https://github.com/alloy-rs/alloy/issues/1528))
- Maybetagged serde for typed transaction ([#1495](https://github.com/alloy-rs/alloy/issues/1495))

### Miscellaneous Tasks

- Release 0.5.3

### Other

- Add `Debug` trait bound for `Transaction` trait ([#1543](https://github.com/alloy-rs/alloy/issues/1543))
- Use `Withdrawals` wrapper in `BlockBody` ([#1525](https://github.com/alloy-rs/alloy/issues/1525))

## [0.5.2](https://github.com/alloy-rs/alloy/releases/tag/v0.5.2) - 2024-10-18

### Bug Fixes

- Fix requests root ([#1521](https://github.com/alloy-rs/alloy/issues/1521))
- Use Decodable directly ([#1522](https://github.com/alloy-rs/alloy/issues/1522))

### Miscellaneous Tasks

- Release 0.5.2
- Make Header encoding good ([#1524](https://github.com/alloy-rs/alloy/issues/1524))
- Reorder bincode modules ([#1520](https://github.com/alloy-rs/alloy/issues/1520))

### Testing

- Extend test with rlp ([#1523](https://github.com/alloy-rs/alloy/issues/1523))

## [0.5.1](https://github.com/alloy-rs/alloy/releases/tag/v0.5.1) - 2024-10-18

### Miscellaneous Tasks

- Release 0.5.1
- Remove 7685 request variants ([#1515](https://github.com/alloy-rs/alloy/issues/1515))

## [0.5.0](https://github.com/alloy-rs/alloy/releases/tag/v0.5.0) - 2024-10-18

### Bug Fixes

- [`rpc-types-eth`] Receipt deser ([#1506](https://github.com/alloy-rs/alloy/issues/1506))
- Use `requests_hash` ([#1508](https://github.com/alloy-rs/alloy/issues/1508))
- Allow missing-tag deser of tx envelope ([#1489](https://github.com/alloy-rs/alloy/issues/1489))
- Rename gas_limit to gas in serde def for txns ([#1486](https://github.com/alloy-rs/alloy/issues/1486))
- Enforce correct parity for legacy transactions ([#1428](https://github.com/alloy-rs/alloy/issues/1428))

### Features

- From impl for variant ([#1488](https://github.com/alloy-rs/alloy/issues/1488))
- `Encodable2718::network_len` ([#1431](https://github.com/alloy-rs/alloy/issues/1431))

### Miscellaneous Tasks

- Release 0.5.0
- Flatten eip-7685 requests into a single opaque list ([#1383](https://github.com/alloy-rs/alloy/issues/1383))
- Rename requests root to requests hash ([#1379](https://github.com/alloy-rs/alloy/issues/1379))
- [consensus] Test use Vec::with_capacity ([#1476](https://github.com/alloy-rs/alloy/issues/1476))
- Some lifetime simplifications ([#1467](https://github.com/alloy-rs/alloy/issues/1467))
- Some small improvements ([#1461](https://github.com/alloy-rs/alloy/issues/1461))
- Apply same member order ([#1408](https://github.com/alloy-rs/alloy/issues/1408))

### Other

- Rm redundant root hash definitions ([#1501](https://github.com/alloy-rs/alloy/issues/1501))
- Add more constraints to `TxReceipt` trait ([#1478](https://github.com/alloy-rs/alloy/issues/1478))
- Replace `to` by `kind` in Transaction trait ([#1484](https://github.com/alloy-rs/alloy/issues/1484))

### Refactor

- Change input output to Bytes ([#1487](https://github.com/alloy-rs/alloy/issues/1487))

## [0.4.2](https://github.com/alloy-rs/alloy/releases/tag/v0.4.2) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.2

### Styling

- Use alloc ([#1405](https://github.com/alloy-rs/alloy/issues/1405))

## [0.4.1](https://github.com/alloy-rs/alloy/releases/tag/v0.4.1) - 2024-10-01

### Features

- [consensus] Bincode compatibility for EIP-7702 ([#1404](https://github.com/alloy-rs/alloy/issues/1404))

### Miscellaneous Tasks

- Release 0.4.1
- [consensus] Less derives for bincode compatible types ([#1401](https://github.com/alloy-rs/alloy/issues/1401))

## [0.4.0](https://github.com/alloy-rs/alloy/releases/tag/v0.4.0) - 2024-09-30

### Bug Fixes

- Advance buffer during 2718 decoding ([#1367](https://github.com/alloy-rs/alloy/issues/1367))
- Correct `encode_2718_len` for legacy transactions ([#1360](https://github.com/alloy-rs/alloy/issues/1360))
- Enforce correct parity encoding for typed transactions ([#1305](https://github.com/alloy-rs/alloy/issues/1305))

### Features

- [consensus] Bincode compatibility for header and transaction types ([#1397](https://github.com/alloy-rs/alloy/issues/1397))
- Impl From<Eip2718Error> for alloy_rlp::Error ([#1359](https://github.com/alloy-rs/alloy/issues/1359))
- Add Header::num_hash_slow ([#1357](https://github.com/alloy-rs/alloy/issues/1357))
- [consensus] Generic Block Type ([#1319](https://github.com/alloy-rs/alloy/issues/1319))
- [consensus] Move requests struct definition from reth ([#1326](https://github.com/alloy-rs/alloy/issues/1326))

### Miscellaneous Tasks

- Release 0.4.0
- Rm outdated comments ([#1392](https://github.com/alloy-rs/alloy/issues/1392))

### Other

- Add supertrait alloy_consensus::Transaction to RPC TransactionResponse ([#1387](https://github.com/alloy-rs/alloy/issues/1387))
- Return static `Eip658Value` from `TxReceipt` trait method ([#1394](https://github.com/alloy-rs/alloy/issues/1394))
- Auto-impl `alloy_consensus::TxReceipt` for ref ([#1395](https://github.com/alloy-rs/alloy/issues/1395))
- Make `gas_limit` u64 for transactions ([#1382](https://github.com/alloy-rs/alloy/issues/1382))
- Make `Header` blob fees u64 ([#1377](https://github.com/alloy-rs/alloy/issues/1377))
- Make `Header` `base_fee_per_gas` u64 ([#1375](https://github.com/alloy-rs/alloy/issues/1375))
- Make `Header` gas limit u64 ([#1333](https://github.com/alloy-rs/alloy/issues/1333))
- Add `Receipts` struct ([#1247](https://github.com/alloy-rs/alloy/issues/1247))
- Add full feature to `derive_more` ([#1335](https://github.com/alloy-rs/alloy/issues/1335))
- Add `BlockHeader` getter trait ([#1302](https://github.com/alloy-rs/alloy/issues/1302))
- Implement custom default for `Account` representing a valid empty account ([#1313](https://github.com/alloy-rs/alloy/issues/1313))

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Miscellaneous Tasks

- Release 0.3.6

## [0.3.5](https://github.com/alloy-rs/alloy/releases/tag/v0.3.5) - 2024-09-13

### Miscellaneous Tasks

- Release 0.3.5

## [0.3.4](https://github.com/alloy-rs/alloy/releases/tag/v0.3.4) - 2024-09-13

### Miscellaneous Tasks

- Release 0.3.4
- [consensus] Remove Header Method ([#1271](https://github.com/alloy-rs/alloy/issues/1271))
- [consensus] Alloc by Default ([#1272](https://github.com/alloy-rs/alloy/issues/1272))

### Other

- Implement `seal` helper for `Header` ([#1269](https://github.com/alloy-rs/alloy/issues/1269))

## [0.3.3](https://github.com/alloy-rs/alloy/releases/tag/v0.3.3) - 2024-09-10

### Miscellaneous Tasks

- Release 0.3.3
- Require destination for 7702 ([#1262](https://github.com/alloy-rs/alloy/issues/1262))

### Other

- Implement `AsRef` for `Header` ([#1260](https://github.com/alloy-rs/alloy/issues/1260))

## [0.3.2](https://github.com/alloy-rs/alloy/releases/tag/v0.3.2) - 2024-09-09

### Bug Fixes

- [consensus] Remove Unused Alloc Vecs ([#1250](https://github.com/alloy-rs/alloy/issues/1250))

### Miscellaneous Tasks

- Release 0.3.2

### Other

- Impl `exceeds_allowed_future_timestamp` for `Header` ([#1237](https://github.com/alloy-rs/alloy/issues/1237))
- Impl `is_zero_difficulty` for `Header` ([#1236](https://github.com/alloy-rs/alloy/issues/1236))
- Impl parent_num_hash for Header ([#1238](https://github.com/alloy-rs/alloy/issues/1238))
- Implement `Arbitrary` for `Header` ([#1235](https://github.com/alloy-rs/alloy/issues/1235))

## [0.3.1](https://github.com/alloy-rs/alloy/releases/tag/v0.3.1) - 2024-09-02

### Bug Fixes

- Value of TxEip1559.ty ([#1210](https://github.com/alloy-rs/alloy/issues/1210))

### Features

- Derive `arbitrary::Arbitrary` for `TxEip7702` ([#1216](https://github.com/alloy-rs/alloy/issues/1216))
- Implement `tx_type` for `TxEip7702` ([#1214](https://github.com/alloy-rs/alloy/issues/1214))

### Miscellaneous Tasks

- Release 0.3.1

### Other

- Rm useless methods for `TxEip7702` ([#1221](https://github.com/alloy-rs/alloy/issues/1221))

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

- Release 0.3.0
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
