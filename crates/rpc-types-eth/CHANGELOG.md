# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.3](https://github.com/alloy-rs/alloy/releases/tag/v1.4.3) - 2026-01-14

### Miscellaneous Tasks

- Release 1.4.2

### Refactor

- [rpc-types-eth] Remove redundant clones in block tests ([#3514](https://github.com/alloy-rs/alloy/issues/3514))

## [1.4.1](https://github.com/alloy-rs/alloy/releases/tag/v1.4.1) - 2026-01-13

### Bug Fixes

- Support Eip7594 blob format for tx build ([#3446](https://github.com/alloy-rs/alloy/issues/3446))

### Features

- [rpc-types-eth] Add Params::from_json_value ([#3466](https://github.com/alloy-rs/alloy/issues/3466))
- [rpc-types-eth] Implement FromStr for SubscriptionKind ([#3465](https://github.com/alloy-rs/alloy/issues/3465))

### Miscellaneous Tasks

- Release 1.4.1
- Release 1.4.0

### Refactor

- [rpc-types-eth] Remove redundant clones in serde tests ([#3491](https://github.com/alloy-rs/alloy/issues/3491))

## [1.3.0](https://github.com/alloy-rs/alloy/releases/tag/v1.3.0) - 2026-01-06

### Documentation

- `s/EIP-4337/ERC-4337/g;` ([#3431](https://github.com/alloy-rs/alloy/issues/3431))

### Features

- [`contract`] Add sidecar_7594 to CallBuilder ([#3424](https://github.com/alloy-rs/alloy/issues/3424))

### Miscellaneous Tasks

- Release 1.3.0

## [1.2.1](https://github.com/alloy-rs/alloy/releases/tag/v1.2.1) - 2025-12-23

### Bug Fixes

- Saturate gas price in deser for unknown networks ([#3095](https://github.com/alloy-rs/alloy/issues/3095))
- [rpc-types-eth] Correct build_7702 panic documentation ([#3332](https://github.com/alloy-rs/alloy/issues/3332))
- Align EIP1186AccountProofResponse::is_empty with EIP-161 ([#3303](https://github.com/alloy-rs/alloy/issues/3303))
- More flexible `BadBlock` type ([#3322](https://github.com/alloy-rs/alloy/issues/3322))

### Documentation

- Fix swapped filter doc comments ([#3308](https://github.com/alloy-rs/alloy/issues/3308))
- [rpc-types-eth] Fix swapped docs for get_to_block/get_from_block ([#3307](https://github.com/alloy-rs/alloy/issues/3307))

### Features

- Add `transactionReceipts` into SubscriptionKind ([#2974](https://github.com/alloy-rs/alloy/issues/2974))
- Add bincode compat support for BlobTransactionSidecarVariant ([#3325](https://github.com/alloy-rs/alloy/issues/3325))
- Allow fusaka sidecars in the tx request ([#3321](https://github.com/alloy-rs/alloy/issues/3321))

### Miscellaneous Tasks

- Release 1.2.1
- Remove cyclic dev dep ([#3411](https://github.com/alloy-rs/alloy/issues/3411))
- Make receipt generic ([#3357](https://github.com/alloy-rs/alloy/issues/3357))
- Rm all deprecations ([#3341](https://github.com/alloy-rs/alloy/issues/3341))

### Other

- Remove transactionReceipts subscription kind ([#3409](https://github.com/alloy-rs/alloy/issues/3409))

## [1.1.3](https://github.com/alloy-rs/alloy/releases/tag/v1.1.3) - 2025-12-06

### Bug Fixes

- Correct SyncInfo.stages doc to list of  Stage  entries ([#3226](https://github.com/alloy-rs/alloy/issues/3226))

### Features

- Add extract_block_range for Filter ([#3300](https://github.com/alloy-rs/alloy/issues/3300))
- Add into-hashes-vec ([#3257](https://github.com/alloy-rs/alloy/issues/3257))

### Miscellaneous Tasks

- Release 1.1.3

## [1.1.2](https://github.com/alloy-rs/alloy/releases/tag/v1.1.2) - 2025-11-20

### Features

- [rpc-types] Add `FillTransaction` response type ([#3210](https://github.com/alloy-rs/alloy/issues/3210))

### Miscellaneous Tasks

- Release 1.1.2

## [1.1.1](https://github.com/alloy-rs/alloy/releases/tag/v1.1.1) - 2025-11-13

### Miscellaneous Tasks

- Release 1.1.1

### Other

- Avoid cloning EIP-4844 sidecar during request build ([#3179](https://github.com/alloy-rs/alloy/issues/3179))

## [1.1.0](https://github.com/alloy-rs/alloy/releases/tag/v1.1.0) - 2025-11-04

### Miscellaneous Tasks

- Release 1.1.0

## [1.0.42](https://github.com/alloy-rs/alloy/releases/tag/v1.0.42) - 2025-10-31

### Miscellaneous Tasks

- Release 1.0.42
- Release 1.0.41

### Refactor

- [rpc-types-eth] Remove duplicate block range filtering logic ([#3077](https://github.com/alloy-rs/alloy/issues/3077))

## [1.0.40](https://github.com/alloy-rs/alloy/releases/tag/v1.0.40) - 2025-10-17

### Miscellaneous Tasks

- Release 1.0.40
- Release 1.0.40

## [1.0.39](https://github.com/alloy-rs/alloy/releases/tag/v1.0.39) - 2025-10-16

### Miscellaneous Tasks

- Release 1.0.39
- Aggregate PRs ([#3011](https://github.com/alloy-rs/alloy/issues/3011))

## [1.0.38](https://github.com/alloy-rs/alloy/releases/tag/v1.0.38) - 2025-10-08

### Documentation

- [rpc-types-eth] Clarify EIP-4844 preferred_type docs to include blob_versioned_hashes ([#2978](https://github.com/alloy-rs/alloy/issues/2978))

### Miscellaneous Tasks

- Release 1.0.38 ([#3007](https://github.com/alloy-rs/alloy/issues/3007))

## [1.0.37](https://github.com/alloy-rs/alloy/releases/tag/v1.0.37) - 2025-09-30

### Miscellaneous Tasks

- Release 1.0.37
- Remove feature(doc_auto_cfg) ([#2941](https://github.com/alloy-rs/alloy/issues/2941))
- [rpc-types-eth] Remove useless serde deny_unknown_fields on enums ([#2927](https://github.com/alloy-rs/alloy/issues/2927))

## [1.0.36](https://github.com/alloy-rs/alloy/releases/tag/v1.0.36) - 2025-09-24

### Miscellaneous Tasks

- Release 1.0.36

## [1.0.35](https://github.com/alloy-rs/alloy/releases/tag/v1.0.35) - 2025-09-22

### Miscellaneous Tasks

- Release 1.0.35

## [1.0.34](https://github.com/alloy-rs/alloy/releases/tag/v1.0.34) - 2025-09-21

### Miscellaneous Tasks

- Release 1.0.34

## [1.0.33](https://github.com/alloy-rs/alloy/releases/tag/v1.0.33) - 2025-09-19

### Miscellaneous Tasks

- Release 1.0.33

## [1.0.32](https://github.com/alloy-rs/alloy/releases/tag/v1.0.32) - 2025-09-16

### Miscellaneous Tasks

- Release 1.0.32

## [1.0.31](https://github.com/alloy-rs/alloy/releases/tag/v1.0.31) - 2025-09-15

### Miscellaneous Tasks

- Release 1.0.31
- Fix unused warning ([#2849](https://github.com/alloy-rs/alloy/issues/2849))

## [1.0.30](https://github.com/alloy-rs/alloy/releases/tag/v1.0.30) - 2025-09-03

### Bug Fixes

- [rpc] Add missing error code `eth_sendRawTransactionSync` timeout ([#2846](https://github.com/alloy-rs/alloy/issues/2846))

### Miscellaneous Tasks

- Release 1.0.30

## [1.0.29](https://github.com/alloy-rs/alloy/releases/tag/v1.0.29) - 2025-09-03

### Miscellaneous Tasks

- Release 1.0.29

## [1.0.28](https://github.com/alloy-rs/alloy/releases/tag/v1.0.28) - 2025-09-02

### Miscellaneous Tasks

- Release 1.0.28

## [1.0.27](https://github.com/alloy-rs/alloy/releases/tag/v1.0.27) - 2025-08-26

### Features

- Fusaka changes ([#2821](https://github.com/alloy-rs/alloy/issues/2821))
- Add helper methods to decode logs in TransactionReceipt ([#2811](https://github.com/alloy-rs/alloy/issues/2811))
- Add fromstr for TransactionInputKind ([#2805](https://github.com/alloy-rs/alloy/issues/2805))
- Add convenience fn for setting 7702 delegation designator ([#2802](https://github.com/alloy-rs/alloy/issues/2802))

### Miscellaneous Tasks

- Release 1.0.27 ([#2822](https://github.com/alloy-rs/alloy/issues/2822))
- Release 1.0.26

## [1.0.25](https://github.com/alloy-rs/alloy/releases/tag/v1.0.25) - 2025-08-19

### Features

- Add authorization list to CallBuilder ([#2798](https://github.com/alloy-rs/alloy/issues/2798))

### Miscellaneous Tasks

- Release 1.0.25
- Release 1.0.25
- Add typos ([#2787](https://github.com/alloy-rs/alloy/issues/2787))

## [1.0.24](https://github.com/alloy-rs/alloy/releases/tag/v1.0.24) - 2025-08-06

### Miscellaneous Tasks

- Release 1.0.24

## [1.0.23](https://github.com/alloy-rs/alloy/releases/tag/v1.0.23) - 2025-07-22

### Miscellaneous Tasks

- Release 1.0.23
- Add helper to collect rpc logs ([#2712](https://github.com/alloy-rs/alloy/issues/2712))

## [1.0.22](https://github.com/alloy-rs/alloy/releases/tag/v1.0.22) - 2025-07-14

### Bug Fixes

- No-std for serde-bincode-compat ([#2711](https://github.com/alloy-rs/alloy/issues/2711))

### Features

- Add filter_receipts iterator for filtering logs from receipts ([#2701](https://github.com/alloy-rs/alloy/issues/2701))

### Miscellaneous Tasks

- Release 1.0.22

## [1.0.21](https://github.com/alloy-rs/alloy/releases/tag/v1.0.21) - 2025-07-14

### Bug Fixes

- Flaky bincode sigs ([#2694](https://github.com/alloy-rs/alloy/issues/2694))

### Features

- Impl AsRef<Self> for TransactionRequest ([#2708](https://github.com/alloy-rs/alloy/issues/2708))
- Added bincodable version of a TransactionRequest struct ([#2687](https://github.com/alloy-rs/alloy/issues/2687))

### Miscellaneous Tasks

- Release 1.0.21

## [1.0.20](https://github.com/alloy-rs/alloy/releases/tag/v1.0.20) - 2025-07-09

### Miscellaneous Tasks

- Release 1.0.20

## [1.0.19](https://github.com/alloy-rs/alloy/releases/tag/v1.0.19) - 2025-07-08

### Miscellaneous Tasks

- Release 1.0.19

### Refactor

- [rpc] Add handwritten bounds on generic `TxReq` using `serde` attributes ([#2674](https://github.com/alloy-rs/alloy/issues/2674))

## [1.0.18](https://github.com/alloy-rs/alloy/releases/tag/v1.0.18) - 2025-07-08

### Features

- [rpc] Implement `Default` for types with `TxReq` generic without `Default` bound ([#2662](https://github.com/alloy-rs/alloy/issues/2662))
- Make build_{eip} functions public ([#2519](https://github.com/alloy-rs/alloy/issues/2519))
- [rpc] Add generic `TxReq` to `SimulatePayload` ([#2631](https://github.com/alloy-rs/alloy/issues/2631))

### Miscellaneous Tasks

- Release 1.0.18
- Release 1.0.17

### Other

- Make tx build fns public ([#2635](https://github.com/alloy-rs/alloy/issues/2635))

## [1.0.16](https://github.com/alloy-rs/alloy/releases/tag/v1.0.16) - 2025-06-27

### Miscellaneous Tasks

- Release 1.0.16

## [1.0.15](https://github.com/alloy-rs/alloy/releases/tag/v1.0.15) - 2025-06-27

### Miscellaneous Tasks

- Release 1.0.15

## [1.0.14](https://github.com/alloy-rs/alloy/releases/tag/v1.0.14) - 2025-06-27

### Features

- [rpc] Add generic `TxReq` to `Bundle` ([#2623](https://github.com/alloy-rs/alloy/issues/2623))
- [rpc] Add generic `TxReq` to `SimBlock` ([#2622](https://github.com/alloy-rs/alloy/issues/2622))

### Miscellaneous Tasks

- Release 1.0.14

## [1.0.13](https://github.com/alloy-rs/alloy/releases/tag/v1.0.13) - 2025-06-26

### Features

- Add better conversions for AnyRpcBlock ([#2614](https://github.com/alloy-rs/alloy/issues/2614))
- Add log filtering methods to Filter ([#2607](https://github.com/alloy-rs/alloy/issues/2607))
- Add block number helper getter ([#2608](https://github.com/alloy-rs/alloy/issues/2608))

### Miscellaneous Tasks

- Release 1.0.13

## [1.0.12](https://github.com/alloy-rs/alloy/releases/tag/v1.0.12) - 2025-06-18

### Bug Fixes

- Move `Transaction::from_transaction` ([#2590](https://github.com/alloy-rs/alloy/issues/2590))

### Miscellaneous Tasks

- Release 1.0.12
- Release 1.0.11

## [1.0.10](https://github.com/alloy-rs/alloy/releases/tag/v1.0.10) - 2025-06-17

### Bug Fixes

- Fix Typo in Function Name ([#2582](https://github.com/alloy-rs/alloy/issues/2582))

### Dependencies

- Bump MSRV to 1.85 ([#2547](https://github.com/alloy-rs/alloy/issues/2547))

### Documentation

- Add examples for TransactionRequest::preferred_type() ([#2568](https://github.com/alloy-rs/alloy/issues/2568))
- Add examples for TransactionRequest::minimal_tx_type() ([#2566](https://github.com/alloy-rs/alloy/issues/2566))

### Features

- [rpc] Convert into RPC transaction from generic `Transaction` ([#2586](https://github.com/alloy-rs/alloy/issues/2586))
- [rpc-types-eth] Add helper methods to AccountInfo ([#2578](https://github.com/alloy-rs/alloy/issues/2578))
- [rpc-types-eth] Add PrunedHistory error code 4444 ([#2575](https://github.com/alloy-rs/alloy/issues/2575))
- Add BlockOverrides::is_empty() method ([#2571](https://github.com/alloy-rs/alloy/issues/2571))
- Add missing gas_price setter to TransactionRequest ([#2567](https://github.com/alloy-rs/alloy/issues/2567))
- Added `log_decode_validate` method ([#2546](https://github.com/alloy-rs/alloy/issues/2546))
- Add additional qol block functions ([#2534](https://github.com/alloy-rs/alloy/issues/2534))

### Miscellaneous Tasks

- Release 1.0.10
- Release 1.0.10
- Relax receipt fn bounds ([#2538](https://github.com/alloy-rs/alloy/issues/2538))

## [1.0.9](https://github.com/alloy-rs/alloy/releases/tag/v1.0.9) - 2025-05-28

### Features

- Introduce serde feature for network-primitives ([#2529](https://github.com/alloy-rs/alloy/issues/2529))

### Miscellaneous Tasks

- Release 1.0.9

### Styling

- Added helper fn for building typed simulate transaction in TransactionRequest ([#2531](https://github.com/alloy-rs/alloy/issues/2531))

## [1.0.8](https://github.com/alloy-rs/alloy/releases/tag/v1.0.8) - 2025-05-27

### Features

- Add missing from impl ([#2514](https://github.com/alloy-rs/alloy/issues/2514))
- Added Transaction conversion from consensus for rpc ([#2511](https://github.com/alloy-rs/alloy/issues/2511))

### Miscellaneous Tasks

- Release 1.0.8
- Generalize rpc tx type conversions ([#2513](https://github.com/alloy-rs/alloy/issues/2513))

## [1.0.7](https://github.com/alloy-rs/alloy/releases/tag/v1.0.7) - 2025-05-24

### Features

- From tx for withotherfields ([#2500](https://github.com/alloy-rs/alloy/issues/2500))
- Introducing builder fn for BlockOverrides ([#2492](https://github.com/alloy-rs/alloy/issues/2492))
- Add option to always set input+data in MulticallBuilder ([#2491](https://github.com/alloy-rs/alloy/issues/2491))

### Miscellaneous Tasks

- Release 1.0.7

## [1.0.6](https://github.com/alloy-rs/alloy/releases/tag/v1.0.6) - 2025-05-21

### Miscellaneous Tasks

- Release 1.0.6

## [1.0.5](https://github.com/alloy-rs/alloy/releases/tag/v1.0.5) - 2025-05-20

### Bug Fixes

- Check each bloom ([#2480](https://github.com/alloy-rs/alloy/issues/2480))

### Miscellaneous Tasks

- Release 1.0.5

## [1.0.4](https://github.com/alloy-rs/alloy/releases/tag/v1.0.4) - 2025-05-19

### Dependencies

- Add auth deserde test ([#2468](https://github.com/alloy-rs/alloy/issues/2468))

### Miscellaneous Tasks

- Release 1.0.4
- Warn missing-const-for-fn ([#2418](https://github.com/alloy-rs/alloy/issues/2418))

## [1.0.3](https://github.com/alloy-rs/alloy/releases/tag/v1.0.3) - 2025-05-15

### Miscellaneous Tasks

- Release 1.0.3 ([#2460](https://github.com/alloy-rs/alloy/issues/2460))
- Release 1.0.2
- Add a new fn for TxType derivation ([#2451](https://github.com/alloy-rs/alloy/issues/2451))
- Use has_eip4884 fields ([#2448](https://github.com/alloy-rs/alloy/issues/2448))

## [1.0.1](https://github.com/alloy-rs/alloy/releases/tag/v1.0.1) - 2025-05-13

### Miscellaneous Tasks

- Release 1.0.1

## [1.0.0](https://github.com/alloy-rs/alloy/releases/tag/v1.0.0) - 2025-05-13

### Dependencies

- Bump jsonrpsee types ([#2439](https://github.com/alloy-rs/alloy/issues/2439))

### Features

- Add helpers to check set fields ([#2431](https://github.com/alloy-rs/alloy/issues/2431))

### Miscellaneous Tasks

- Release 1.0.0

## [0.15.11](https://github.com/alloy-rs/alloy/releases/tag/v0.15.11) - 2025-05-12

### Bug Fixes

- Ensure mandatory to field ([#2412](https://github.com/alloy-rs/alloy/issues/2412))

### Miscellaneous Tasks

- Release 0.15.11
- Add back filteredparams ([#2421](https://github.com/alloy-rs/alloy/issues/2421))

### Refactor

- Improve and simplify event filters ([#2140](https://github.com/alloy-rs/alloy/issues/2140))

## [0.15.10](https://github.com/alloy-rs/alloy/releases/tag/v0.15.10) - 2025-05-07

### Miscellaneous Tasks

- Release 0.15.10

### Styling

- Introducing eth_getAccountInfo ([#2402](https://github.com/alloy-rs/alloy/issues/2402))

## [0.15.9](https://github.com/alloy-rs/alloy/releases/tag/v0.15.9) - 2025-05-05

### Features

- Add input data helpers ([#2393](https://github.com/alloy-rs/alloy/issues/2393))

### Miscellaneous Tasks

- Release 0.15.9

## [0.15.8](https://github.com/alloy-rs/alloy/releases/tag/v0.15.8) - 2025-05-02

### Documentation

- Add a note about transaction input ([#2380](https://github.com/alloy-rs/alloy/issues/2380))

### Miscellaneous Tasks

- Release 0.15.8

## [0.15.7](https://github.com/alloy-rs/alloy/releases/tag/v0.15.7) - 2025-04-30

### Documentation

- Minor correction ([#2374](https://github.com/alloy-rs/alloy/issues/2374))

### Miscellaneous Tasks

- Release 0.15.7
- Add helpers to rpc block type ([#2355](https://github.com/alloy-rs/alloy/issues/2355))

### Other

- Deleted duplicate `for for` to `for` request.rs ([#2347](https://github.com/alloy-rs/alloy/issues/2347))

## [0.15.6](https://github.com/alloy-rs/alloy/releases/tag/v0.15.6) - 2025-04-24

### Bug Fixes

- Use correct type in conversion ([#2346](https://github.com/alloy-rs/alloy/issues/2346))

### Miscellaneous Tasks

- Release 0.15.6

## [0.15.5](https://github.com/alloy-rs/alloy/releases/tag/v0.15.5) - 2025-04-24

### Miscellaneous Tasks

- Release 0.15.5
- Relax rpc tx conversions ([#2345](https://github.com/alloy-rs/alloy/issues/2345))
- Release 0.15.4

## [0.15.3](https://github.com/alloy-rs/alloy/releases/tag/v0.15.3) - 2025-04-24

### Miscellaneous Tasks

- Release 0.15.3

## [0.15.2](https://github.com/alloy-rs/alloy/releases/tag/v0.15.2) - 2025-04-23

### Miscellaneous Tasks

- Release 0.15.2

## [0.15.1](https://github.com/alloy-rs/alloy/releases/tag/v0.15.1) - 2025-04-23

### Miscellaneous Tasks

- Release 0.15.1

## [0.15.0](https://github.com/alloy-rs/alloy/releases/tag/v0.15.0) - 2025-04-23

### Miscellaneous Tasks

- Release 0.15.0

## [0.14.0](https://github.com/alloy-rs/alloy/releases/tag/v0.14.0) - 2025-04-09

### Dependencies

- [deps] Core 1.0 ([#2184](https://github.com/alloy-rs/alloy/issues/2184))

### Features

- Filterset topics extend ([#2258](https://github.com/alloy-rs/alloy/issues/2258))
- Make it easier to configure non u256 topics in filterset ([#2257](https://github.com/alloy-rs/alloy/issues/2257))

### Miscellaneous Tasks

- Release 0.14.0

## [0.13.0](https://github.com/alloy-rs/alloy/releases/tag/v0.13.0) - 2025-03-28

### Features

- Add EIP1186AccountProofResponse::is_empty ([#2224](https://github.com/alloy-rs/alloy/issues/2224))

### Miscellaneous Tasks

- Release 0.13.0
- Expect instead of allow ([#2228](https://github.com/alloy-rs/alloy/issues/2228))
- Propagate arbitrary feature ([#2227](https://github.com/alloy-rs/alloy/issues/2227))

### Other

- Add more details on FilterSet ([#2229](https://github.com/alloy-rs/alloy/issues/2229))

## [0.12.6](https://github.com/alloy-rs/alloy/releases/tag/v0.12.6) - 2025-03-18

### Features

- Ad helper append fn ([#2186](https://github.com/alloy-rs/alloy/issues/2186))

### Miscellaneous Tasks

- Release 0.12.6

## [0.12.5](https://github.com/alloy-rs/alloy/releases/tag/v0.12.5) - 2025-03-12

### Miscellaneous Tasks

- Release 0.12.5
- Add fromiter helper for stateoverridesbuilder ([#2182](https://github.com/alloy-rs/alloy/issues/2182))
- Add with capacity helper ([#2183](https://github.com/alloy-rs/alloy/issues/2183))

## [0.12.4](https://github.com/alloy-rs/alloy/releases/tag/v0.12.4) - 2025-03-07

### Miscellaneous Tasks

- Release 0.12.4

## [0.12.3](https://github.com/alloy-rs/alloy/releases/tag/v0.12.3) - 2025-03-07

### Miscellaneous Tasks

- Release 0.12.3

## [0.12.2](https://github.com/alloy-rs/alloy/releases/tag/v0.12.2) - 2025-03-07

### Miscellaneous Tasks

- Release 0.12.2
- Release 0.12.1

## [0.12.0](https://github.com/alloy-rs/alloy/releases/tag/v0.12.0) - 2025-03-07

### Bug Fixes

- [`provider`] Custom deser for pending blocks ([#2146](https://github.com/alloy-rs/alloy/issues/2146))
- Run zepter checks for features of non-workspace dependencies ([#2144](https://github.com/alloy-rs/alloy/issues/2144))
- [`rpc-types`] Allow missing `effectiveGasPrice` in TxReceipt ([#2143](https://github.com/alloy-rs/alloy/issues/2143))

### Features

- More helper conversions ([#2159](https://github.com/alloy-rs/alloy/issues/2159))
- Integrate `Recovered` into more types ([#2151](https://github.com/alloy-rs/alloy/issues/2151))
- Introduce dedicated types for Any type aliases ([#2046](https://github.com/alloy-rs/alloy/issues/2046))
- Create StateOverridesBuilder ([#2106](https://github.com/alloy-rs/alloy/issues/2106))
- Add more transaction conversion helpers ([#2103](https://github.com/alloy-rs/alloy/issues/2103))
- Add helper rpc to block body conversion ([#2055](https://github.com/alloy-rs/alloy/issues/2055))
- [`rpc-types`] Decode log from receipt ([#2086](https://github.com/alloy-rs/alloy/issues/2086))
- Add optional builder APIs for AccountOverride ([#2064](https://github.com/alloy-rs/alloy/issues/2064))

### Miscellaneous Tasks

- Release 0.12.0
- Support static error msg ([#2158](https://github.com/alloy-rs/alloy/issues/2158))
- [`consensus`] Rename `Recovered` methods ([#2155](https://github.com/alloy-rs/alloy/issues/2155))
- Use impl Into StateOverride ([#2145](https://github.com/alloy-rs/alloy/issues/2145))
- Add blob gas method to TransactionRequest impl ([#2122](https://github.com/alloy-rs/alloy/issues/2122))
- Smol typo ([#2069](https://github.com/alloy-rs/alloy/issues/2069))
- Additional From TryFrom conversion helpers ([#2054](https://github.com/alloy-rs/alloy/issues/2054))

## [0.11.1](https://github.com/alloy-rs/alloy/releases/tag/v0.11.1) - 2025-02-12

### Features

- Add builder style account override helpers ([#2039](https://github.com/alloy-rs/alloy/issues/2039))
- Add Block::apply ([#2006](https://github.com/alloy-rs/alloy/issues/2006))

### Miscellaneous Tasks

- Release 0.11.1

## [0.11.0](https://github.com/alloy-rs/alloy/releases/tag/v0.11.0) - 2025-01-31

### Documentation

- Enable some useful rustdoc features on docs.rs ([#1890](https://github.com/alloy-rs/alloy/issues/1890))

### Features

- Add TxRequest::from_recovered_transaction helper ([#1960](https://github.com/alloy-rs/alloy/issues/1960))
- Add into sealed for rpc header ([#1956](https://github.com/alloy-rs/alloy/issues/1956))
- Add helpers for tx conditional ([#1953](https://github.com/alloy-rs/alloy/issues/1953))
- Add calc tx root fn for rpc types ([#1950](https://github.com/alloy-rs/alloy/issues/1950))
- Add map fns to rpc transaction type ([#1936](https://github.com/alloy-rs/alloy/issues/1936))
- Rm 7702 auth items from receipt response ([#1897](https://github.com/alloy-rs/alloy/issues/1897))
- Remove T: Transport from public APIs ([#1859](https://github.com/alloy-rs/alloy/issues/1859))

### Miscellaneous Tasks

- Release 0.11.0
- Add receipt conversion fns ([#1949](https://github.com/alloy-rs/alloy/issues/1949))
- Release 0.10.0

### Other

- Add zepter and propagate features ([#1951](https://github.com/alloy-rs/alloy/issues/1951))

## [0.9.2](https://github.com/alloy-rs/alloy/releases/tag/v0.9.2) - 2025-01-03

### Features

- Add conversions from rpc block to consensus ([#1869](https://github.com/alloy-rs/alloy/issues/1869))

### Miscellaneous Tasks

- Release 0.9.2

## [0.9.1](https://github.com/alloy-rs/alloy/releases/tag/v0.9.1) - 2024-12-30

### Bug Fixes

- Use u64 for all gas values ([#1848](https://github.com/alloy-rs/alloy/issues/1848))
- Support hex values for conditional options ([#1824](https://github.com/alloy-rs/alloy/issues/1824))

### Features

- Add more builder style fns ([#1850](https://github.com/alloy-rs/alloy/issues/1850))
- Add match functions ([#1847](https://github.com/alloy-rs/alloy/issues/1847))
- EIP-7840 ([#1828](https://github.com/alloy-rs/alloy/issues/1828))
- Add map transactions to rpc block type ([#1835](https://github.com/alloy-rs/alloy/issues/1835))
- [pectra] Revert EIP-7742 ([#1807](https://github.com/alloy-rs/alloy/issues/1807))
- Add cost fn for conditional opts ([#1823](https://github.com/alloy-rs/alloy/issues/1823))

### Miscellaneous Tasks

- Release 0.9.1
- Make clippy happy ([#1849](https://github.com/alloy-rs/alloy/issues/1849))
- Rm non exhaustive from ReceiptEnvelope ([#1843](https://github.com/alloy-rs/alloy/issues/1843))
- Rm non exhaustive for envelope ([#1842](https://github.com/alloy-rs/alloy/issues/1842))
- Map header fns ([#1840](https://github.com/alloy-rs/alloy/issues/1840))
- Rename ConditionalOptions ([#1825](https://github.com/alloy-rs/alloy/issues/1825))
- Replace derive_more with thiserror ([#1822](https://github.com/alloy-rs/alloy/issues/1822))

## [0.8.3](https://github.com/alloy-rs/alloy/releases/tag/v0.8.3) - 2024-12-20

### Miscellaneous Tasks

- Release 0.8.3

## [0.8.2](https://github.com/alloy-rs/alloy/releases/tag/v0.8.2) - 2024-12-19

### Bug Fixes

- Relax legacy chain id check ([#1809](https://github.com/alloy-rs/alloy/issues/1809))

### Miscellaneous Tasks

- Release 0.8.2
- Misc clippy ([#1812](https://github.com/alloy-rs/alloy/issues/1812))

## [0.8.1](https://github.com/alloy-rs/alloy/releases/tag/v0.8.1) - 2024-12-16

### Documentation

- Add note about deprecated total difficulty ([#1784](https://github.com/alloy-rs/alloy/issues/1784))

### Features

- Add info tx types ([#1793](https://github.com/alloy-rs/alloy/issues/1793))

### Miscellaneous Tasks

- Release 0.8.1

### Other

- Improve doc clarity around build functions ([#1782](https://github.com/alloy-rs/alloy/issues/1782))

## [0.8.0](https://github.com/alloy-rs/alloy/releases/tag/v0.8.0) - 2024-12-10

### Bug Fixes

- Use asref impl for receipt ([#1758](https://github.com/alloy-rs/alloy/issues/1758))

### Features

- [consensus] Require typed2718 for transaction ([#1746](https://github.com/alloy-rs/alloy/issues/1746))
- Relax RPC `Block` bounds ([#1757](https://github.com/alloy-rs/alloy/issues/1757))

### Miscellaneous Tasks

- Release 0.8.0 ([#1778](https://github.com/alloy-rs/alloy/issues/1778))
- Improve Display and Debug for BlockId ([#1765](https://github.com/alloy-rs/alloy/issues/1765))

### Other

- Reapply "feat(consensus): require typed2718 for transaction ([#1746](https://github.com/alloy-rs/alloy/issues/1746))" ([#1773](https://github.com/alloy-rs/alloy/issues/1773))
- Revert "feat(consensus): require typed2718 for transaction ([#1746](https://github.com/alloy-rs/alloy/issues/1746))" ([#1772](https://github.com/alloy-rs/alloy/issues/1772))

## [0.7.3](https://github.com/alloy-rs/alloy/releases/tag/v0.7.3) - 2024-12-05

### Bug Fixes

- Remove `Borrow` impl for RPC receipt ([#1721](https://github.com/alloy-rs/alloy/issues/1721))

### Dependencies

- [general] Bump MSRV to 1.81, use `core::error::Error` on `no-std` compatible crates ([#1552](https://github.com/alloy-rs/alloy/issues/1552))

### Features

- Feat(rpc-types-eth) add test for syncing ([#1724](https://github.com/alloy-rs/alloy/issues/1724))

### Miscellaneous Tasks

- Release 0.7.3
- Release 0.7.2 ([#1729](https://github.com/alloy-rs/alloy/issues/1729))

## [0.7.0](https://github.com/alloy-rs/alloy/releases/tag/v0.7.0) - 2024-11-28

### Features

- EIP-7742 ([#1600](https://github.com/alloy-rs/alloy/issues/1600))
- Add helpers to initialize Tx request ([#1690](https://github.com/alloy-rs/alloy/issues/1690))
- Modifiy ReceiptWithBloom and associated impls to use with Reth ([#1672](https://github.com/alloy-rs/alloy/issues/1672))
- [consensus-tx] Enable fast `is_create` ([#1683](https://github.com/alloy-rs/alloy/issues/1683))
- Move `AnyReceipt` and `AnyHeader` to `alloy-consensus-any` ([#1609](https://github.com/alloy-rs/alloy/issues/1609))

### Miscellaneous Tasks

- Release 0.7.0
- Release 0.7.0
- Release 0.7.0
- Move from impls to where they belong ([#1691](https://github.com/alloy-rs/alloy/issues/1691))
- Add new fn to eip1186 ([#1692](https://github.com/alloy-rs/alloy/issues/1692))
- Make clippy happy ([#1677](https://github.com/alloy-rs/alloy/issues/1677))

### Other

- Add unit tests for pubsub ([#1663](https://github.com/alloy-rs/alloy/issues/1663))

### Testing

- Add test for 7702 with v ([#1644](https://github.com/alloy-rs/alloy/issues/1644))

## [0.6.4](https://github.com/alloy-rs/alloy/releases/tag/v0.6.4) - 2024-11-12

### Miscellaneous Tasks

- Release 0.6.4

### Other

- Add trait method `Transaction::effective_gas_price` ([#1640](https://github.com/alloy-rs/alloy/issues/1640))

## [0.6.3](https://github.com/alloy-rs/alloy/releases/tag/v0.6.3) - 2024-11-12

### Bug Fixes

- Serde for transactions ([#1630](https://github.com/alloy-rs/alloy/issues/1630))
- [`rpc-types`] `FeeHistory` deser ([#1629](https://github.com/alloy-rs/alloy/issues/1629))

### Miscellaneous Tasks

- Release 0.6.3
- Release 0.6.2 ([#1632](https://github.com/alloy-rs/alloy/issues/1632))

### Other

- Add trait method `Transaction::is_dynamic_fee` ([#1638](https://github.com/alloy-rs/alloy/issues/1638))

## [0.6.1](https://github.com/alloy-rs/alloy/releases/tag/v0.6.1) - 2024-11-06

### Bug Fixes

- Re-introduce HeaderResponse trait ([#1627](https://github.com/alloy-rs/alloy/issues/1627))

### Miscellaneous Tasks

- Release 0.6.1

## [0.6.0](https://github.com/alloy-rs/alloy/releases/tag/v0.6.0) - 2024-11-06

### Bug Fixes

- Hash handling ([#1604](https://github.com/alloy-rs/alloy/issues/1604))
- Update AnyNetwork type aliases ([#1591](https://github.com/alloy-rs/alloy/issues/1591))

### Features

- Implement Arbitrary for transaction types ([#1603](https://github.com/alloy-rs/alloy/issues/1603))
- Embed consensus header into RPC ([#1573](https://github.com/alloy-rs/alloy/issues/1573))

### Miscellaneous Tasks

- Release 0.6.0

### Other

- Embed TxEnvelope into `rpc-types-eth::Transaction` ([#1460](https://github.com/alloy-rs/alloy/issues/1460))
- Add `BadBlock` type to `debug_getbadblocks` return type ([#1566](https://github.com/alloy-rs/alloy/issues/1566))
- Add `uncle_block_from_header` impl and test ([#1554](https://github.com/alloy-rs/alloy/issues/1554))
- Impl `From<Sealed<alloy_consensus::Header>>` for `Header` ([#1532](https://github.com/alloy-rs/alloy/issues/1532))

### Styling

- Move txtype-specific builders to network-primitives ([#1602](https://github.com/alloy-rs/alloy/issues/1602))

## [0.5.4](https://github.com/alloy-rs/alloy/releases/tag/v0.5.4) - 2024-10-23

### Miscellaneous Tasks

- Release 0.5.4

## [0.5.3](https://github.com/alloy-rs/alloy/releases/tag/v0.5.3) - 2024-10-22

### Bug Fixes

- [filter] Treat null fields as null ([#1529](https://github.com/alloy-rs/alloy/issues/1529))

### Dependencies

- Bump alloy-eip7702 ([#1547](https://github.com/alloy-rs/alloy/issues/1547))

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

- [`rpc-types-eth`] Receipt deser ([#1506](https://github.com/alloy-rs/alloy/issues/1506))
- Remove signature assoc type from tx response trait ([#1451](https://github.com/alloy-rs/alloy/issues/1451))

### Features

- BuildTransactionErr abstract over builder type ([#1452](https://github.com/alloy-rs/alloy/issues/1452))

### Miscellaneous Tasks

- Release 0.5.0
- Flatten eip-7685 requests into a single opaque list ([#1383](https://github.com/alloy-rs/alloy/issues/1383))
- Rename requests root to requests hash ([#1379](https://github.com/alloy-rs/alloy/issues/1379))
- Refactor some match with same arms ([#1463](https://github.com/alloy-rs/alloy/issues/1463))
- More simplifications ([#1469](https://github.com/alloy-rs/alloy/issues/1469))
- Some lifetime simplifications ([#1467](https://github.com/alloy-rs/alloy/issues/1467))
- Some small improvements ([#1461](https://github.com/alloy-rs/alloy/issues/1461))
- [rpc] Make TransactionRequest conversions exhaustive ([#1427](https://github.com/alloy-rs/alloy/issues/1427))
- Apply same member order ([#1408](https://github.com/alloy-rs/alloy/issues/1408))

### Other

- Replace `to` by `kind` in Transaction trait ([#1484](https://github.com/alloy-rs/alloy/issues/1484))
- Revert test: update test cases with addresses ([#1358](https://github.com/alloy-rs/alloy/issues/1358)) ([#1444](https://github.com/alloy-rs/alloy/issues/1444))
- Replace assert_eq! with similar_asserts::assert_eq! ([#1429](https://github.com/alloy-rs/alloy/issues/1429))

### Refactor

- Change input output to Bytes ([#1487](https://github.com/alloy-rs/alloy/issues/1487))

## [0.4.2](https://github.com/alloy-rs/alloy/releases/tag/v0.4.2) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.2

## [0.4.1](https://github.com/alloy-rs/alloy/releases/tag/v0.4.1) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.1

## [0.4.0](https://github.com/alloy-rs/alloy/releases/tag/v0.4.0) - 2024-09-30

### Bug Fixes

- `eth_simulateV1` serde ([#1345](https://github.com/alloy-rs/alloy/issues/1345))

### Features

- Replace std/hashbrown with alloy_primitives::map ([#1384](https://github.com/alloy-rs/alloy/issues/1384))
- [serde] Remove deprecated `num` module ([#1350](https://github.com/alloy-rs/alloy/issues/1350))
- [provider] Subscribe to new blocks if possible in heartbeat ([#1321](https://github.com/alloy-rs/alloy/issues/1321))
- Add getters into TransactionResponse and update implementations  ([#1328](https://github.com/alloy-rs/alloy/issues/1328))
- Add builder style function to simulate payload args ([#1324](https://github.com/alloy-rs/alloy/issues/1324))

### Miscellaneous Tasks

- Release 0.4.0
- Fix warnings on no_std ([#1355](https://github.com/alloy-rs/alloy/issues/1355))

### Other

- Add supertrait alloy_consensus::Transaction to RPC TransactionResponse ([#1387](https://github.com/alloy-rs/alloy/issues/1387))
- Make `gas_limit` u64 for transactions ([#1382](https://github.com/alloy-rs/alloy/issues/1382))
- Make `Header` blob fees u64 ([#1377](https://github.com/alloy-rs/alloy/issues/1377))
- Make `Header` `base_fee_per_gas` u64 ([#1375](https://github.com/alloy-rs/alloy/issues/1375))
- Make `Header` gas limit u64 ([#1333](https://github.com/alloy-rs/alloy/issues/1333))
- Make factory and paymaster fields optional in `PackedUserOperation` ([#1330](https://github.com/alloy-rs/alloy/issues/1330))
- Remove repetitive as_ref ([#1329](https://github.com/alloy-rs/alloy/issues/1329))

### Testing

- Update test cases with addresses ([#1358](https://github.com/alloy-rs/alloy/issues/1358))

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Bug Fixes

- [types-eth] Optional Alloy Serde ([#1284](https://github.com/alloy-rs/alloy/issues/1284))
- `eth_simulateV1` ([#1289](https://github.com/alloy-rs/alloy/issues/1289))

### Miscellaneous Tasks

- Release 0.3.6
- Release 0.3.5

## [0.3.4](https://github.com/alloy-rs/alloy/releases/tag/v0.3.4) - 2024-09-13

### Bug Fixes

- `debug_traceCallMany` and `trace_callMany` ([#1278](https://github.com/alloy-rs/alloy/issues/1278))
- Serde for `eth_simulateV1` ([#1273](https://github.com/alloy-rs/alloy/issues/1273))

### Features

- [alloy-rpc-types-eth] Optional serde ([#1276](https://github.com/alloy-rs/alloy/issues/1276))
- No_std eth rpc types ([#1252](https://github.com/alloy-rs/alloy/issues/1252))

### Miscellaneous Tasks

- Release 0.3.4

### Other

- Add trait methods `cumulative_gas_used` and `state_root` to `ReceiptResponse` ([#1275](https://github.com/alloy-rs/alloy/issues/1275))

## [0.3.3](https://github.com/alloy-rs/alloy/releases/tag/v0.3.3) - 2024-09-10

### Miscellaneous Tasks

- Release 0.3.3
- Require destination for 7702 ([#1262](https://github.com/alloy-rs/alloy/issues/1262))

## [0.3.2](https://github.com/alloy-rs/alloy/releases/tag/v0.3.2) - 2024-09-09

### Bug Fixes

- [consensus] Remove Unused Alloc Vecs ([#1250](https://github.com/alloy-rs/alloy/issues/1250))

### Features

- No_std network primitives ([#1248](https://github.com/alloy-rs/alloy/issues/1248))
- [rpc-types-eth] AnyBlock ([#1243](https://github.com/alloy-rs/alloy/issues/1243))
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
- Remove optimism-related types ([#1203](https://github.com/alloy-rs/alloy/issues/1203))
- Use `impl From<RangeInclusive> for FilterBlockOption` instead of `Range`  ([#1199](https://github.com/alloy-rs/alloy/issues/1199))
- Serde for `depositReceiptVersion` ([#1196](https://github.com/alloy-rs/alloy/issues/1196))
- Change generics order for `Block` ([#1192](https://github.com/alloy-rs/alloy/issues/1192))
- Add missing op fields ([#1187](https://github.com/alloy-rs/alloy/issues/1187))
- Remove `OtherFields` from Transaction and Block ([#1154](https://github.com/alloy-rs/alloy/issues/1154))
- [rpc-types-eth] Match 7702 in TxReceipt.status() ([#1149](https://github.com/alloy-rs/alloy/issues/1149))
- Trim conflicting key `max_fee_per_blob_gas` from Eip1559 tx type ([#1064](https://github.com/alloy-rs/alloy/issues/1064))

### Dependencies

- Rm 2930 and 7702 - use alloy-rs/eips ([#1181](https://github.com/alloy-rs/alloy/issues/1181))
- Bump core and rm ssz feat ([#1167](https://github.com/alloy-rs/alloy/issues/1167))
- Bump jsonrpsee 0.24 ([#1067](https://github.com/alloy-rs/alloy/issues/1067))

### Features

- Add erc4337 endpoint methods to provider ([#1176](https://github.com/alloy-rs/alloy/issues/1176))
- Make block struct generic over header type ([#1179](https://github.com/alloy-rs/alloy/issues/1179))
- Network-parameterized block responses ([#1106](https://github.com/alloy-rs/alloy/issues/1106))
- Add 7702 tx enum ([#1059](https://github.com/alloy-rs/alloy/issues/1059))
- Add authorization list to TransactionRequest ([#1125](https://github.com/alloy-rs/alloy/issues/1125))
- Eth_simulateV1 Request / Response types ([#1042](https://github.com/alloy-rs/alloy/issues/1042))
- Feat(rpc-type-eth) convert vec TxReq to bundle ([#1091](https://github.com/alloy-rs/alloy/issues/1091))
- Feat(provider) : introduction to eth_sendRawTransactionConditional  RPC endpoint type ([#1009](https://github.com/alloy-rs/alloy/issues/1009))
- [rpc-types-eth] Serde flatten `BlobTransactionSidecar` in tx req ([#1054](https://github.com/alloy-rs/alloy/issues/1054))
- Add authorization list to rpc transaction and tx receipt types ([#1051](https://github.com/alloy-rs/alloy/issues/1051))

### Miscellaneous Tasks

- Release 0.3.0
- Rm Rich type ([#1195](https://github.com/alloy-rs/alloy/issues/1195))
- Clippy für docs ([#1194](https://github.com/alloy-rs/alloy/issues/1194))
- Remove RichBlock and RichHeader types ([#1185](https://github.com/alloy-rs/alloy/issues/1185))
- Add deposit receipt version ([#1188](https://github.com/alloy-rs/alloy/issues/1188))
- [eip7702] Devnet3 changes ([#1056](https://github.com/alloy-rs/alloy/issues/1056))
- Release 0.2.1
- [rpc] Make `Deserialize` impl for `FilterChanges` generic over transaction ([#1118](https://github.com/alloy-rs/alloy/issues/1118))
- Export rpc account type ([#1075](https://github.com/alloy-rs/alloy/issues/1075))
- Release 0.2.0
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))

### Other

- Implement conversion between signature types ([#1198](https://github.com/alloy-rs/alloy/issues/1198))
- Rm `PeerCount` ([#1140](https://github.com/alloy-rs/alloy/issues/1140))
- TxRequest into EIP-4844 without sidecar ([#1093](https://github.com/alloy-rs/alloy/issues/1093))
- Make `alloy_rpc_types_eth::SubscriptionResult` generic over tx ([#1123](https://github.com/alloy-rs/alloy/issues/1123))
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
- Add eip-7702 helpers ([#950](https://github.com/alloy-rs/alloy/issues/950))
- [contract] Implement Filter's builder methods on Event ([#960](https://github.com/alloy-rs/alloy/issues/960))

### Miscellaneous Tasks

- Release 0.1.4
- Convert rcp-types-eth block Header to consensus Header ([#1014](https://github.com/alloy-rs/alloy/issues/1014))
- Make wrapped index value pub ([#988](https://github.com/alloy-rs/alloy/issues/988))
- Release 0.1.3
- Nightly clippy ([#947](https://github.com/alloy-rs/alloy/issues/947))

### Other

- Add range test in `FilterBlockOption` ([#939](https://github.com/alloy-rs/alloy/issues/939))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Dependencies

- [deps] Bump all ([#864](https://github.com/alloy-rs/alloy/issues/864))

### Documentation

- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- Add `is_` and `as_` utils for `FilterBlockOption` ([#927](https://github.com/alloy-rs/alloy/issues/927))
- Add utils to `ValueOrArray` ([#924](https://github.com/alloy-rs/alloy/issues/924))
- Add `is_` utils to `FilterChanges` ([#923](https://github.com/alloy-rs/alloy/issues/923))
- Integrate `EvmOverrides` to rpc types ([#906](https://github.com/alloy-rs/alloy/issues/906))
- Add getter methods for `FilterChanges` ([#899](https://github.com/alloy-rs/alloy/issues/899))
- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- [rpc] Split off `eth` namespace in `alloy-rpc-types` to `alloy-rpc-types-eth` ([#847](https://github.com/alloy-rs/alloy/issues/847))

### Miscellaneous Tasks

- Release 0.1.2
- [rpc-types] Remove duplicate `Index` definition in `rpc-types-anvil` in favor of the one in `rpc-types-eth` ([#943](https://github.com/alloy-rs/alloy/issues/943))
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))
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
