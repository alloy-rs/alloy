# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- [doc] Correct order of fields ([#1139](https://github.com/alloy-rs/alloy/issues/1139))
- Correctly trim eip7251 bytecode ([#1105](https://github.com/alloy-rs/alloy/issues/1105))
- [eips] Make SignedAuthorizationList arbitrary less fallible ([#1084](https://github.com/alloy-rs/alloy/issues/1084))
- Require storageKeys value broken bincode serialization from [#955](https://github.com/alloy-rs/alloy/issues/955) ([#1058](https://github.com/alloy-rs/alloy/issues/1058))
- Cargo fmt ([#1044](https://github.com/alloy-rs/alloy/issues/1044))
- [eip7702] Add correct rlp decode/encode ([#1034](https://github.com/alloy-rs/alloy/issues/1034))

### Dependencies

- Rm 2930 and 7702 - use alloy-rs/eips ([#1181](https://github.com/alloy-rs/alloy/issues/1181))
- Bump core and rm ssz feat ([#1167](https://github.com/alloy-rs/alloy/issues/1167))
- [deps] Bump some deps ([#1141](https://github.com/alloy-rs/alloy/issues/1141))

### Features

- [eip] Make 7702 auth recovery fallible ([#1082](https://github.com/alloy-rs/alloy/issues/1082))
- Add authorization list to rpc transaction and tx receipt types ([#1051](https://github.com/alloy-rs/alloy/issues/1051))
- Generate valid signed auth signatures ([#1041](https://github.com/alloy-rs/alloy/issues/1041))
- Add arbitrary to auth ([#1036](https://github.com/alloy-rs/alloy/issues/1036))
- Add hash for 7702 ([#1037](https://github.com/alloy-rs/alloy/issues/1037))

### Miscellaneous Tasks

- Clippy für docs ([#1194](https://github.com/alloy-rs/alloy/issues/1194))
- [eip7702] Devnet3 changes ([#1056](https://github.com/alloy-rs/alloy/issues/1056))
- Release 0.2.1
- Release 0.2.0
- Make auth mandatory in recovered auth ([#1047](https://github.com/alloy-rs/alloy/issues/1047))

### Other

- Add conversion from BlockHashOrNumber to BlockId ([#1127](https://github.com/alloy-rs/alloy/issues/1127))
- Add `AccessListResult` type (EIP-2930) ([#1110](https://github.com/alloy-rs/alloy/issues/1110))

### Styling

- Remove proptest in all crates and Arbitrary derives ([#966](https://github.com/alloy-rs/alloy/issues/966))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Features

- Add consolidation requests to v4 payload ([#1013](https://github.com/alloy-rs/alloy/issues/1013))
- [eip1559] Support Optimism Canyon hardfork ([#1010](https://github.com/alloy-rs/alloy/issues/1010))
- Impl `From<RpcBlockHash>` for `BlockHashOrNumber` ([#980](https://github.com/alloy-rs/alloy/issues/980))

### Miscellaneous Tasks

- Release 0.1.4
- Add helper functions for destructuring auth types ([#1022](https://github.com/alloy-rs/alloy/issues/1022))
- Clean up 7702 encoding ([#1000](https://github.com/alloy-rs/alloy/issues/1000))

### Testing

- Add missing unit test for op `calc_next_block_base_fee` ([#1008](https://github.com/alloy-rs/alloy/issues/1008))

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Bug Fixes

- Deserialization of null storage keys in AccessListItem ([#955](https://github.com/alloy-rs/alloy/issues/955))

### Dependencies

- [eips] Make `alloy-serde` optional under `serde` ([#948](https://github.com/alloy-rs/alloy/issues/948))

### Features

- Add eip-7702 helpers ([#950](https://github.com/alloy-rs/alloy/issues/950))
- Add eip-7251 system contract address/code ([#956](https://github.com/alloy-rs/alloy/issues/956))

### Miscellaneous Tasks

- Release 0.1.3
- [eips] Add serde to Authorization types ([#964](https://github.com/alloy-rs/alloy/issues/964))
- [eips] Make `sha2` optional, add `kzg-sidecar` feature ([#949](https://github.com/alloy-rs/alloy/issues/949))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Update alloy-eips supported eip list ([#942](https://github.com/alloy-rs/alloy/issues/942))
- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- Add eip-7251 consolidation request ([#919](https://github.com/alloy-rs/alloy/issues/919))
- Add `BlockId::as_u64` ([#916](https://github.com/alloy-rs/alloy/issues/916))

### Miscellaneous Tasks

- Release 0.1.2
- Update eip-2935 bytecode and address ([#934](https://github.com/alloy-rs/alloy/issues/934))
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- Non_exhaustive for 2718 error ([#837](https://github.com/alloy-rs/alloy/issues/837))
- Add proptest derives back ([#797](https://github.com/alloy-rs/alloy/issues/797))
- Serde rename camelcase ([#748](https://github.com/alloy-rs/alloy/issues/748))
- Correct exitV1 type ([#567](https://github.com/alloy-rs/alloy/issues/567))
- Infinite loop while decoding a list of transactions ([#432](https://github.com/alloy-rs/alloy/issues/432))
- Use enveloped encoding for typed transactions ([#239](https://github.com/alloy-rs/alloy/issues/239))
- [`eips`/`consensus`] Correctly decode txs on `TxEnvelope` ([#148](https://github.com/alloy-rs/alloy/issues/148))

### Dependencies

- Deduplicate AccessList and Withdrawals types ([#324](https://github.com/alloy-rs/alloy/issues/324))
- Alloy-consensus crate ([#83](https://github.com/alloy-rs/alloy/issues/83))

### Documentation

- Update descriptions and top level summary ([#128](https://github.com/alloy-rs/alloy/issues/128))

### Features

- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- Derive `Default` for `WithdrawalRequest` and `DepositRequest` ([#867](https://github.com/alloy-rs/alloy/issues/867))
- [serde] Deprecate individual num::* for a generic `quantity` module ([#855](https://github.com/alloy-rs/alloy/issues/855))
- [eips] EIP-2935 history storage contract ([#747](https://github.com/alloy-rs/alloy/issues/747))
- Rlp enc/dec for requests ([#728](https://github.com/alloy-rs/alloy/issues/728))
- [consensus, eips] EIP-7002 system contract ([#727](https://github.com/alloy-rs/alloy/issues/727))
- Add eth mainnet EL requests envelope ([#707](https://github.com/alloy-rs/alloy/issues/707))
- Add eip-7685 enc/decode traits ([#704](https://github.com/alloy-rs/alloy/issues/704))
- Rlp for eip-7002 requests ([#705](https://github.com/alloy-rs/alloy/issues/705))
- Manual blob deserialize ([#696](https://github.com/alloy-rs/alloy/issues/696))
- Derive arbitrary for BlobTransactionSidecar ([#679](https://github.com/alloy-rs/alloy/issues/679))
- Use alloy types for BlobTransactionSidecar ([#673](https://github.com/alloy-rs/alloy/issues/673))
- Add prague engine types ([#557](https://github.com/alloy-rs/alloy/issues/557))
- Add BaseFeeParams::new ([#525](https://github.com/alloy-rs/alloy/issues/525))
- Port helpers for accesslist ([#508](https://github.com/alloy-rs/alloy/issues/508))
- Joinable transaction fillers ([#426](https://github.com/alloy-rs/alloy/issues/426))
- Serde for consensus tx types ([#361](https://github.com/alloy-rs/alloy/issues/361))
- 4844 SidecarBuilder ([#250](https://github.com/alloy-rs/alloy/issues/250))
- Support no_std for `alloy-eips` ([#181](https://github.com/alloy-rs/alloy/issues/181))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Network abstraction and transaction builder ([#190](https://github.com/alloy-rs/alloy/issues/190))
- [`consensus`] Add extra EIP-4844 types needed ([#229](https://github.com/alloy-rs/alloy/issues/229))

### Miscellaneous Tasks

- Update EIP7002 withdrawal requests based on spec ([#885](https://github.com/alloy-rs/alloy/issues/885))
- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [eips] Compile tests with default features ([#860](https://github.com/alloy-rs/alloy/issues/860))
- [docs] Crate completeness and fix typos ([#861](https://github.com/alloy-rs/alloy/issues/861))
- [docs] Add doc aliases ([#843](https://github.com/alloy-rs/alloy/issues/843))
- Add Into for WithOtherFields in rpc types ([#813](https://github.com/alloy-rs/alloy/issues/813))
- Fix remaining warnings, add TODO for proptest-derive ([#819](https://github.com/alloy-rs/alloy/issues/819))
- Fix warnings, check-cfg ([#776](https://github.com/alloy-rs/alloy/issues/776))
- Rename deposit receipt to deposit request ([#693](https://github.com/alloy-rs/alloy/issues/693))
- Move blob validation to sidecar ([#677](https://github.com/alloy-rs/alloy/issues/677))
- Replace `ExitV1` with `WithdrawalRequest` ([#672](https://github.com/alloy-rs/alloy/issues/672))
- Move BlockId type to alloy-eip ([#565](https://github.com/alloy-rs/alloy/issues/565))
- Clippy, warnings ([#504](https://github.com/alloy-rs/alloy/issues/504))
- Add helper for next block base fee ([#494](https://github.com/alloy-rs/alloy/issues/494))
- Clean up kzg and features ([#386](https://github.com/alloy-rs/alloy/issues/386))
- Error when missing to field in transaction conversion ([#365](https://github.com/alloy-rs/alloy/issues/365))
- Clippy ([#251](https://github.com/alloy-rs/alloy/issues/251))

### Other

- [Fix] use Eip2718Error, add docs on different encodings ([#869](https://github.com/alloy-rs/alloy/issues/869))
- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Arbitrary Sidecar implementation + build. Closes [#680](https://github.com/alloy-rs/alloy/issues/680). ([#708](https://github.com/alloy-rs/alloy/issues/708))
- Use Self instead of BlockNumberOrTag ([#754](https://github.com/alloy-rs/alloy/issues/754))
- Use Self when possible ([#711](https://github.com/alloy-rs/alloy/issues/711))
- Small refactor ([#652](https://github.com/alloy-rs/alloy/issues/652))
- Move block hash types to alloy-eips ([#639](https://github.com/alloy-rs/alloy/issues/639))
- Add arbitrary derive for Withdrawal ([#501](https://github.com/alloy-rs/alloy/issues/501))
- Extension ([#474](https://github.com/alloy-rs/alloy/issues/474))
- Derive arbitrary for rpc `Header` and `Transaction` ([#458](https://github.com/alloy-rs/alloy/issues/458))
- Added MAINNET_KZG_TRUSTED_SETUP ([#385](https://github.com/alloy-rs/alloy/issues/385))
- Check no_std in CI ([#367](https://github.com/alloy-rs/alloy/issues/367))

### Refactor

- Clean up legacy serde helpers ([#624](https://github.com/alloy-rs/alloy/issues/624))

### Styling

- [Blocked] Update TransactionRequest's `to` field to TxKind ([#553](https://github.com/alloy-rs/alloy/issues/553))
- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))
- [Feature] Move Mainnet KZG group and Lazy<KzgSettings> ([#368](https://github.com/alloy-rs/alloy/issues/368))

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
