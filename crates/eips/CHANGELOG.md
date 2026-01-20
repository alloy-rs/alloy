# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.3](https://github.com/alloy-rs/alloy/releases/tag/v1.4.3) - 2026-01-14

### Miscellaneous Tasks

- Release 1.4.2

## [1.4.1](https://github.com/alloy-rs/alloy/releases/tag/v1.4.1) - 2026-01-13

### Bug Fixes

- [eips] Use for loop in blob conversion to avoid stack overflow ([#3499](https://github.com/alloy-rs/alloy/issues/3499))

### Features

- [rpc-types-engine] Add ExecutionPayloadEnvelope V4/V5 conversions ([#3510](https://github.com/alloy-rs/alloy/issues/3510))

### Miscellaneous Tasks

- Release 1.4.1
- Release 1.4.0

### Other

- [eips] Make Blob import conditional ([#3472](https://github.com/alloy-rs/alloy/issues/3472))

## [1.3.0](https://github.com/alloy-rs/alloy/releases/tag/v1.3.0) - 2026-01-06

### Bug Fixes

- Update `SidecarBuilder::build` to allow 7594 ([#3428](https://github.com/alloy-rs/alloy/issues/3428))

### Features

- Add try_from_blobs for BlobTransactionSidecarEip7594 ([#3425](https://github.com/alloy-rs/alloy/issues/3425))

### Miscellaneous Tasks

- Release 1.3.0

## [1.2.1](https://github.com/alloy-rs/alloy/releases/tag/v1.2.1) - 2025-12-23

### Bug Fixes

- Simplify size functions ([#3403](https://github.com/alloy-rs/alloy/issues/3403))

### Features

- Add bincode compat support for BlobTransactionSidecarVariant ([#3325](https://github.com/alloy-rs/alloy/issues/3325))

### Miscellaneous Tasks

- Release 1.2.1
- Rm all deprecations ([#3341](https://github.com/alloy-rs/alloy/issues/3341))

## [1.1.3](https://github.com/alloy-rs/alloy/releases/tag/v1.1.3) - 2025-12-06

### Bug Fixes

- [eip1898] RpcBlockHash serde to use rename_all = \"camelCase\" ([#3255](https://github.com/alloy-rs/alloy/issues/3255))

### Miscellaneous Tasks

- Release 1.1.3

## [1.1.2](https://github.com/alloy-rs/alloy/releases/tag/v1.1.2) - 2025-11-20

### Miscellaneous Tasks

- Release 1.1.2

## [1.1.1](https://github.com/alloy-rs/alloy/releases/tag/v1.1.1) - 2025-11-13

### Features

- Add EIP-7594 conversion and sidecar manipulation API ([#3144](https://github.com/alloy-rs/alloy/issues/3144))
- [consensus,eips,genesis] Add Borsh support ([#2946](https://github.com/alloy-rs/alloy/issues/2946))

### Miscellaneous Tasks

- Release 1.1.1

### Refactor

- [consensus] Remove Borsh skip attributes from tx structs ([#3155](https://github.com/alloy-rs/alloy/issues/3155))

## [1.1.0](https://github.com/alloy-rs/alloy/releases/tag/v1.1.0) - 2025-11-04

### Bug Fixes

- BlobParams bincode deserialization ([#3132](https://github.com/alloy-rs/alloy/issues/3132))

### Dependencies

- Bump MSRV to 1.88 ([#3123](https://github.com/alloy-rs/alloy/issues/3123))

### Miscellaneous Tasks

- Release 1.1.0

## [1.0.42](https://github.com/alloy-rs/alloy/releases/tag/v1.0.42) - 2025-10-31

### Features

- Add blobs & into_blobs methods ([#3072](https://github.com/alloy-rs/alloy/issues/3072))
- Add methods to build EIP-7594 sidecars with default and custom settings ([#3036](https://github.com/alloy-rs/alloy/issues/3036))

### Miscellaneous Tasks

- Release 1.0.42
- Add number value to MAX_TX_GAS_LIMIT_OSAKA ([#3093](https://github.com/alloy-rs/alloy/issues/3093))
- Release 1.0.41
- Release 1.0.40

## [1.0.39](https://github.com/alloy-rs/alloy/releases/tag/v1.0.39) - 2025-10-16

### Features

- Add helper for legacy -> 7594 sidecar conversion ([#3013](https://github.com/alloy-rs/alloy/issues/3013))

### Miscellaneous Tasks

- Release 1.0.39
- Fix unused import ([#3015](https://github.com/alloy-rs/alloy/issues/3015))

## [1.0.38](https://github.com/alloy-rs/alloy/releases/tag/v1.0.38) - 2025-10-08

### Features

- [beacon-types] `GetBlobsResponse` ([#2994](https://github.com/alloy-rs/alloy/issues/2994))

### Miscellaneous Tasks

- Release 1.0.38 ([#3007](https://github.com/alloy-rs/alloy/issues/3007))

## [1.0.37](https://github.com/alloy-rs/alloy/releases/tag/v1.0.37) - 2025-09-30

### Bug Fixes

- Use correct base update fraction ([#2958](https://github.com/alloy-rs/alloy/issues/2958))

### Miscellaneous Tasks

- Release 1.0.37
- Remove feature(doc_auto_cfg) ([#2941](https://github.com/alloy-rs/alloy/issues/2941))
- [eip-7702] Remove the leading whitespace of predeployed contract ([#2937](https://github.com/alloy-rs/alloy/issues/2937))

## [1.0.36](https://github.com/alloy-rs/alloy/releases/tag/v1.0.36) - 2025-09-24

### Miscellaneous Tasks

- Release 1.0.36

## [1.0.35](https://github.com/alloy-rs/alloy/releases/tag/v1.0.35) - 2025-09-22

### Features

- Add bpo initalizers ([#2914](https://github.com/alloy-rs/alloy/issues/2914))

### Miscellaneous Tasks

- Release 1.0.35
- Add helper for init blobparams ([#2913](https://github.com/alloy-rs/alloy/issues/2913))

## [1.0.34](https://github.com/alloy-rs/alloy/releases/tag/v1.0.34) - 2025-09-21

### Features

- [eips] Add `MAX_TX_GAS_LIMIT_OSAKA` for EIP-7825 ([#2906](https://github.com/alloy-rs/alloy/issues/2906))

### Miscellaneous Tasks

- Release 1.0.34

## [1.0.33](https://github.com/alloy-rs/alloy/releases/tag/v1.0.33) - 2025-09-19

### Bug Fixes

- [eip4844] Clippy no warning ([#2898](https://github.com/alloy-rs/alloy/issues/2898))

### Miscellaneous Tasks

- Release 1.0.33

## [1.0.32](https://github.com/alloy-rs/alloy/releases/tag/v1.0.32) - 2025-09-16

### Miscellaneous Tasks

- Release 1.0.32

## [1.0.31](https://github.com/alloy-rs/alloy/releases/tag/v1.0.31) - 2025-09-15

### Miscellaneous Tasks

- Release 1.0.31
- Mark legacy blob gas fn deprecated ([#2863](https://github.com/alloy-rs/alloy/issues/2863))

### Refactor

- Consolidate effective gas price calculation into eip1559 module ([#2872](https://github.com/alloy-rs/alloy/issues/2872))

## [1.0.30](https://github.com/alloy-rs/alloy/releases/tag/v1.0.30) - 2025-09-03

### Miscellaneous Tasks

- Release 1.0.30

## [1.0.29](https://github.com/alloy-rs/alloy/releases/tag/v1.0.29) - 2025-09-03

### Miscellaneous Tasks

- Release 1.0.29

## [1.0.28](https://github.com/alloy-rs/alloy/releases/tag/v1.0.28) - 2025-09-02

### Features

- Add Asref for recovered withencoded ([#2828](https://github.com/alloy-rs/alloy/issues/2828))

### Miscellaneous Tasks

- Release 1.0.28

## [1.0.27](https://github.com/alloy-rs/alloy/releases/tag/v1.0.27) - 2025-08-26

### Bug Fixes

- [eip4844] Prevent overflow panic in fake_exponential with large excess blob gas ([#2806](https://github.com/alloy-rs/alloy/issues/2806))
- [docs] Correct typos in EIP reference ([#2759](https://github.com/alloy-rs/alloy/issues/2759))

### Features

- Fusaka changes ([#2821](https://github.com/alloy-rs/alloy/issues/2821))
- Add scale helper ([#2797](https://github.com/alloy-rs/alloy/issues/2797))

### Miscellaneous Tasks

- Release 1.0.27 ([#2822](https://github.com/alloy-rs/alloy/issues/2822))
- Release 1.0.26
- Release 1.0.25
- Add encode helper ([#2789](https://github.com/alloy-rs/alloy/issues/2789))

## [1.0.24](https://github.com/alloy-rs/alloy/releases/tag/v1.0.24) - 2025-08-06

### Bug Fixes

- Fix simple error `therefor` - `therefore` in eip1898.rs ([#2739](https://github.com/alloy-rs/alloy/issues/2739))

### Miscellaneous Tasks

- Release 1.0.24
- Feature gate serde test ([#2765](https://github.com/alloy-rs/alloy/issues/2765))

## [1.0.23](https://github.com/alloy-rs/alloy/releases/tag/v1.0.23) - 2025-07-22

### Bug Fixes

- Don't stack overflow when deserring new sidecars ([#2713](https://github.com/alloy-rs/alloy/issues/2713))

### Dependencies

- Bump precompute default to 8 ([#2732](https://github.com/alloy-rs/alloy/issues/2732))

### Miscellaneous Tasks

- Release 1.0.23

## [1.0.22](https://github.com/alloy-rs/alloy/releases/tag/v1.0.22) - 2025-07-14

### Miscellaneous Tasks

- Release 1.0.22

## [1.0.21](https://github.com/alloy-rs/alloy/releases/tag/v1.0.21) - 2025-07-14

### Features

- Added  Eip7594 support to Simplecoder for creating blob sidecars ([#2653](https://github.com/alloy-rs/alloy/issues/2653))

### Miscellaneous Tasks

- Release 1.0.21
- Sidecar helper fns ([#2700](https://github.com/alloy-rs/alloy/issues/2700))

## [1.0.20](https://github.com/alloy-rs/alloy/releases/tag/v1.0.20) - 2025-07-09

### Miscellaneous Tasks

- Release 1.0.20

## [1.0.19](https://github.com/alloy-rs/alloy/releases/tag/v1.0.19) - 2025-07-08

### Miscellaneous Tasks

- Release 1.0.19

## [1.0.18](https://github.com/alloy-rs/alloy/releases/tag/v1.0.18) - 2025-07-08

### Miscellaneous Tasks

- Release 1.0.18
- Release 1.0.17

## [1.0.16](https://github.com/alloy-rs/alloy/releases/tag/v1.0.16) - 2025-06-27

### Miscellaneous Tasks

- Release 1.0.16

## [1.0.15](https://github.com/alloy-rs/alloy/releases/tag/v1.0.15) - 2025-06-27

### Miscellaneous Tasks

- Release 1.0.15

## [1.0.14](https://github.com/alloy-rs/alloy/releases/tag/v1.0.14) - 2025-06-27

### Miscellaneous Tasks

- Release 1.0.14

## [1.0.13](https://github.com/alloy-rs/alloy/releases/tag/v1.0.13) - 2025-06-26

### Features

- Added convinient fn decode_2718_exact ([#2603](https://github.com/alloy-rs/alloy/issues/2603))

### Miscellaneous Tasks

- Release 1.0.13

## [1.0.12](https://github.com/alloy-rs/alloy/releases/tag/v1.0.12) - 2025-06-18

### Bug Fixes

- Fix misleading doc comment ([#2545](https://github.com/alloy-rs/alloy/issues/2545))

### Features

- Implement `TransactionEnvelope` derive macro ([#2585](https://github.com/alloy-rs/alloy/issues/2585))
- BlobParams::max_blobs_per_tx ([#2564](https://github.com/alloy-rs/alloy/issues/2564))
- Implement support for BPO forks ([#2542](https://github.com/alloy-rs/alloy/issues/2542))

### Miscellaneous Tasks

- Release 1.0.12
- Release 1.0.11
- Release 1.0.10
- Remove fulu blob constants ([#2563](https://github.com/alloy-rs/alloy/issues/2563))

## [1.0.9](https://github.com/alloy-rs/alloy/releases/tag/v1.0.9) - 2025-05-28

### Miscellaneous Tasks

- Release 1.0.9
- Add from impl ([#2522](https://github.com/alloy-rs/alloy/issues/2522))

## [1.0.8](https://github.com/alloy-rs/alloy/releases/tag/v1.0.8) - 2025-05-27

### Documentation

- Add some kzgsettings docs ([#2518](https://github.com/alloy-rs/alloy/issues/2518))

### Miscellaneous Tasks

- Release 1.0.8
- Add serialize impl ([#2521](https://github.com/alloy-rs/alloy/issues/2521))

## [1.0.7](https://github.com/alloy-rs/alloy/releases/tag/v1.0.7) - 2025-05-24

### Features

- Add lenient_block_number_or_tag to support raw integers ([#2488](https://github.com/alloy-rs/alloy/issues/2488))
- Encodable2718:into_encoded ([#2486](https://github.com/alloy-rs/alloy/issues/2486))

### Miscellaneous Tasks

- Release 1.0.7

## [1.0.6](https://github.com/alloy-rs/alloy/releases/tag/v1.0.6) - 2025-05-21

### Miscellaneous Tasks

- Release 1.0.6
- Rm redundant commitment copy ([#2484](https://github.com/alloy-rs/alloy/issues/2484))

### Refactor

- Create VersionedHashIter to remove unnecessary collect() ([#2483](https://github.com/alloy-rs/alloy/issues/2483))

## [1.0.5](https://github.com/alloy-rs/alloy/releases/tag/v1.0.5) - 2025-05-20

### Miscellaneous Tasks

- Release 1.0.5

## [1.0.4](https://github.com/alloy-rs/alloy/releases/tag/v1.0.4) - 2025-05-19

### Features

- [eips] Sidecar conversion methods ([#2464](https://github.com/alloy-rs/alloy/issues/2464))

### Miscellaneous Tasks

- Release 1.0.4
- Warn missing-const-for-fn ([#2418](https://github.com/alloy-rs/alloy/issues/2418))

### Styling

- Introducing manual deserde for BlobTransactionSidecarVariant ([#2440](https://github.com/alloy-rs/alloy/issues/2440))

## [1.0.3](https://github.com/alloy-rs/alloy/releases/tag/v1.0.3) - 2025-05-15

### Features

- [consensus] Relax `TxEip4844WithSidecar` trait implementations ([#2446](https://github.com/alloy-rs/alloy/issues/2446))

### Miscellaneous Tasks

- Release 1.0.3 ([#2460](https://github.com/alloy-rs/alloy/issues/2460))
- Release 1.0.2
- Add sidecar helpers ([#2445](https://github.com/alloy-rs/alloy/issues/2445))

## [1.0.1](https://github.com/alloy-rs/alloy/releases/tag/v1.0.1) - 2025-05-13

### Miscellaneous Tasks

- Release 1.0.1

## [1.0.0](https://github.com/alloy-rs/alloy/releases/tag/v1.0.0) - 2025-05-13

### Features

- [eips] Add `BlobTransactionSidecarVariant` ([#2430](https://github.com/alloy-rs/alloy/issues/2430))
- [eips] `BlobTransactionSidecarEip7594` ([#2428](https://github.com/alloy-rs/alloy/issues/2428))
- [eips] Osaka blob params ([#2427](https://github.com/alloy-rs/alloy/issues/2427))
- [eips] Add more EIP-7594 constants ([#2425](https://github.com/alloy-rs/alloy/issues/2425))

### Miscellaneous Tasks

- Release 1.0.0

## [0.15.11](https://github.com/alloy-rs/alloy/releases/tag/v0.15.11) - 2025-05-12

### Documentation

- Should be decoded ([#2414](https://github.com/alloy-rs/alloy/issues/2414))

### Features

- Add some either impls ([#2409](https://github.com/alloy-rs/alloy/issues/2409))

### Miscellaneous Tasks

- Release 0.15.11

## [0.15.10](https://github.com/alloy-rs/alloy/releases/tag/v0.15.10) - 2025-05-07

### Miscellaneous Tasks

- Release 0.15.10

### Other

- Propagate arb feature ([#2407](https://github.com/alloy-rs/alloy/issues/2407))

### Styling

- Chore : fix typos ([#2398](https://github.com/alloy-rs/alloy/issues/2398))

## [0.15.9](https://github.com/alloy-rs/alloy/releases/tag/v0.15.9) - 2025-05-05

### Features

- Add Arbitrary Support for payload types ([#2392](https://github.com/alloy-rs/alloy/issues/2392))
- Add IsTyped2718  ([#2394](https://github.com/alloy-rs/alloy/issues/2394))

### Miscellaneous Tasks

- Release 0.15.9
- Add default to blob schedule ([#2389](https://github.com/alloy-rs/alloy/issues/2389))

## [0.15.8](https://github.com/alloy-rs/alloy/releases/tag/v0.15.8) - 2025-05-02

### Features

- Add 7623 consts ([#2383](https://github.com/alloy-rs/alloy/issues/2383))

### Miscellaneous Tasks

- Release 0.15.8
- Add 0x prefix to eip addresses ([#2382](https://github.com/alloy-rs/alloy/issues/2382))

### Styling

- Added  helpers for blob schedule format ([#2375](https://github.com/alloy-rs/alloy/issues/2375))

## [0.15.7](https://github.com/alloy-rs/alloy/releases/tag/v0.15.7) - 2025-04-30

### Miscellaneous Tasks

- Release 0.15.7
- Clippy happy ([#2370](https://github.com/alloy-rs/alloy/issues/2370))

## [0.15.6](https://github.com/alloy-rs/alloy/releases/tag/v0.15.6) - 2025-04-24

### Miscellaneous Tasks

- Release 0.15.6

## [0.15.5](https://github.com/alloy-rs/alloy/releases/tag/v0.15.5) - 2025-04-24

### Miscellaneous Tasks

- Release 0.15.5
- Release 0.15.4
- Mark 4844 constants deprecated ([#2341](https://github.com/alloy-rs/alloy/issues/2341))

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

### Documentation

- Remove consecutive duplicate words ([#2337](https://github.com/alloy-rs/alloy/issues/2337))

### Miscellaneous Tasks

- Release 0.15.0

## [0.14.0](https://github.com/alloy-rs/alloy/releases/tag/v0.14.0) - 2025-04-09

### Bug Fixes

- `BlobAndProofV2` ([#2283](https://github.com/alloy-rs/alloy/issues/2283))

### Miscellaneous Tasks

- Release 0.14.0

## [0.13.0](https://github.com/alloy-rs/alloy/releases/tag/v0.13.0) - 2025-03-28

### Dependencies

- [deps] C-kzg 2.0 ([#2240](https://github.com/alloy-rs/alloy/issues/2240))

### Features

- Eip7594 constants ([#2245](https://github.com/alloy-rs/alloy/issues/2245))

### Miscellaneous Tasks

- Release 0.13.0
- Expect instead of allow ([#2228](https://github.com/alloy-rs/alloy/issues/2228))

### Other

- Auto_impl(&) for Encodable2718 ([#2230](https://github.com/alloy-rs/alloy/issues/2230))

## [0.12.6](https://github.com/alloy-rs/alloy/releases/tag/v0.12.6) - 2025-03-18

### Bug Fixes

- Broken links `eip1559/constants.rs` ([#2190](https://github.com/alloy-rs/alloy/issues/2190))

### Features

- [eips] Serde untagged for EIP-7685 `RequestsOrHash` ([#2216](https://github.com/alloy-rs/alloy/issues/2216))
- Add BlobAndProofV2 ([#2202](https://github.com/alloy-rs/alloy/issues/2202))

### Miscellaneous Tasks

- Release 0.12.6

## [0.12.5](https://github.com/alloy-rs/alloy/releases/tag/v0.12.5) - 2025-03-12

### Bug Fixes

- Filter out requests with len 1 ([#2167](https://github.com/alloy-rs/alloy/issues/2167))

### Miscellaneous Tasks

- Release 0.12.5

## [0.12.4](https://github.com/alloy-rs/alloy/releases/tag/v0.12.4) - 2025-03-07

### Miscellaneous Tasks

- Release 0.12.4

## [0.12.3](https://github.com/alloy-rs/alloy/releases/tag/v0.12.3) - 2025-03-07

### Miscellaneous Tasks

- Release 0.12.3

## [0.12.2](https://github.com/alloy-rs/alloy/releases/tag/v0.12.2) - 2025-03-07

### Bug Fixes

- Reduce stack for blob helpers ([#2161](https://github.com/alloy-rs/alloy/issues/2161))

### Miscellaneous Tasks

- Release 0.12.2
- Release 0.12.1

## [0.12.0](https://github.com/alloy-rs/alloy/releases/tag/v0.12.0) - 2025-03-07

### Bug Fixes

- Run zepter checks for features of non-workspace dependencies ([#2144](https://github.com/alloy-rs/alloy/issues/2144))

### Features

- Add encodable to either ([#2130](https://github.com/alloy-rs/alloy/issues/2130))
- Add into bytes ([#2109](https://github.com/alloy-rs/alloy/issues/2109))
- [`eip4844`] Heap allocated blob ([#2050](https://github.com/alloy-rs/alloy/issues/2050))
- Add helpers to create a BlobSidecar ([#2047](https://github.com/alloy-rs/alloy/issues/2047))

### Miscellaneous Tasks

- Release 0.12.0

### Other

- Implement Transaction type on Either type ([#2097](https://github.com/alloy-rs/alloy/issues/2097))
- Move WithEncoded helper type to alloy ([#2098](https://github.com/alloy-rs/alloy/issues/2098))

## [0.11.1](https://github.com/alloy-rs/alloy/releases/tag/v0.11.1) - 2025-02-12

### Features

- Add helpers for the blob gas ([#2009](https://github.com/alloy-rs/alloy/issues/2009))

### Miscellaneous Tasks

- Release 0.11.1
- Re-export kzgsettings ([#2034](https://github.com/alloy-rs/alloy/issues/2034))
- Camelcase serde ([#2018](https://github.com/alloy-rs/alloy/issues/2018))
- Add serde support for Eip1559Estimation ([#2012](https://github.com/alloy-rs/alloy/issues/2012))

### Other

- Increase default gas limit from 30M to 36M ([#1785](https://github.com/alloy-rs/alloy/issues/1785))

## [0.11.0](https://github.com/alloy-rs/alloy/releases/tag/v0.11.0) - 2025-01-31

### Documentation

- Enable some useful rustdoc features on docs.rs ([#1890](https://github.com/alloy-rs/alloy/issues/1890))

### Features

- Unify `BlobParams` and `BlobScheduleItem` ([#1919](https://github.com/alloy-rs/alloy/issues/1919))
- Reexport eip2124 ([#1900](https://github.com/alloy-rs/alloy/issues/1900))
- Add match_versioned_hashes ([#1882](https://github.com/alloy-rs/alloy/issues/1882))

### Miscellaneous Tasks

- Release 0.11.0
- Update system contract addresses for devnet 6 ([#1975](https://github.com/alloy-rs/alloy/issues/1975))
- Feature gate serde ([#1967](https://github.com/alloy-rs/alloy/issues/1967))
- Forward arbitrary feature ([#1941](https://github.com/alloy-rs/alloy/issues/1941))
- [eips] Add super trait `Typed2718` to `Encodable2718` ([#1913](https://github.com/alloy-rs/alloy/issues/1913))
- Release 0.10.0
- Improve FromStr for `BlockNumberOrTag` to be case-insensitive ([#1891](https://github.com/alloy-rs/alloy/issues/1891))
- Shift std::error impls to core ([#1888](https://github.com/alloy-rs/alloy/issues/1888))
- Use core::error for blob validation error ([#1887](https://github.com/alloy-rs/alloy/issues/1887))
- Use safe get api  ([#1886](https://github.com/alloy-rs/alloy/issues/1886))

### Other

- Add zepter and propagate features ([#1951](https://github.com/alloy-rs/alloy/issues/1951))

### Testing

- Require serde features for tests ([#1924](https://github.com/alloy-rs/alloy/issues/1924))
- Migrate eip1898 tests ([#1922](https://github.com/alloy-rs/alloy/issues/1922))

## [0.9.2](https://github.com/alloy-rs/alloy/releases/tag/v0.9.2) - 2025-01-03

### Bug Fixes

- [eip7251] Update contract address and bytecode ([#1877](https://github.com/alloy-rs/alloy/issues/1877))
- Skip empty request objects ([#1873](https://github.com/alloy-rs/alloy/issues/1873))

### Features

- Sort and skip empty requests for hash ([#1878](https://github.com/alloy-rs/alloy/issues/1878))

### Miscellaneous Tasks

- Release 0.9.2

## [0.9.1](https://github.com/alloy-rs/alloy/releases/tag/v0.9.1) - 2024-12-30

### Bug Fixes

- [alloy-eips] `SimpleCoder::decode_one()` should return `Ok(None)` ([#1818](https://github.com/alloy-rs/alloy/issues/1818))

### Features

- EIP-7840 ([#1828](https://github.com/alloy-rs/alloy/issues/1828))
- [pectra] Revert EIP-7742 ([#1807](https://github.com/alloy-rs/alloy/issues/1807))

### Miscellaneous Tasks

- Release 0.9.1
- Add history serve window ([#1865](https://github.com/alloy-rs/alloy/issues/1865))

### Other

- [Feature] update Display implementation on BlockNumberOrTag ([#1857](https://github.com/alloy-rs/alloy/issues/1857))
- [Bug] Request predeploy codes have diverged ([#1845](https://github.com/alloy-rs/alloy/issues/1845))
- Update contract bytecode & address ([#1838](https://github.com/alloy-rs/alloy/issues/1838))
- Update `CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS` ([#1836](https://github.com/alloy-rs/alloy/issues/1836))
- Update `WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS` ([#1834](https://github.com/alloy-rs/alloy/issues/1834))

## [0.8.3](https://github.com/alloy-rs/alloy/releases/tag/v0.8.3) - 2024-12-20

### Miscellaneous Tasks

- Release 0.8.3

## [0.8.2](https://github.com/alloy-rs/alloy/releases/tag/v0.8.2) - 2024-12-19

### Miscellaneous Tasks

- Release 0.8.2

## [0.8.1](https://github.com/alloy-rs/alloy/releases/tag/v0.8.1) - 2024-12-16

### Features

- [relay] ExecutionRequestsV4 with eip7685::Requests conversion ([#1787](https://github.com/alloy-rs/alloy/issues/1787))
- Add requests with capacity ([#1794](https://github.com/alloy-rs/alloy/issues/1794))

### Miscellaneous Tasks

- Release 0.8.1
- Port calc block gas limit ([#1798](https://github.com/alloy-rs/alloy/issues/1798))
- Add helper for loading custom trusted setup ([#1779](https://github.com/alloy-rs/alloy/issues/1779))

### Other

- Calc_blob_gasprice made const ([#1788](https://github.com/alloy-rs/alloy/issues/1788))

## [0.8.0](https://github.com/alloy-rs/alloy/releases/tag/v0.8.0) - 2024-12-10

### Features

- Add arbitrary for alloy types ([#1777](https://github.com/alloy-rs/alloy/issues/1777))
- EIP-7691 ([#1762](https://github.com/alloy-rs/alloy/issues/1762))

### Miscellaneous Tasks

- Release 0.8.0 ([#1778](https://github.com/alloy-rs/alloy/issues/1778))
- Derive Copy for BlockWithParent ([#1776](https://github.com/alloy-rs/alloy/issues/1776))
- Improve Display and Debug for BlockId ([#1765](https://github.com/alloy-rs/alloy/issues/1765))

## [0.7.3](https://github.com/alloy-rs/alloy/releases/tag/v0.7.3) - 2024-12-05

### Bug Fixes

- Adjust EIP-7742 to latest spec ([#1713](https://github.com/alloy-rs/alloy/issues/1713))
- Omit empty requests ([#1706](https://github.com/alloy-rs/alloy/issues/1706))
- Use B256::new instead of from ([#1701](https://github.com/alloy-rs/alloy/issues/1701))
- EIP-7742 fixes ([#1697](https://github.com/alloy-rs/alloy/issues/1697))

### Dependencies

- [general] Bump MSRV to 1.81, use `core::error::Error` on `no-std` compatible crates ([#1552](https://github.com/alloy-rs/alloy/issues/1552))

### Documentation

- Update docs for eip7685 `Requests` ([#1714](https://github.com/alloy-rs/alloy/issues/1714))

### Features

- Impl `Encodable2718` for `ReceiptWithBloom` ([#1719](https://github.com/alloy-rs/alloy/issues/1719))
- EIP-7685 requests helpers ([#1699](https://github.com/alloy-rs/alloy/issues/1699))
- [eips] Make prague field an enum ([#1574](https://github.com/alloy-rs/alloy/issues/1574))
- EIP-7742 ([#1600](https://github.com/alloy-rs/alloy/issues/1600))

### Miscellaneous Tasks

- Release 0.7.3
- Release 0.7.2 ([#1729](https://github.com/alloy-rs/alloy/issues/1729))
- Release 0.7.0
- EIP-7685 changes ([#1599](https://github.com/alloy-rs/alloy/issues/1599))

### Other

- Add `BlockWithParent` ([#1650](https://github.com/alloy-rs/alloy/issues/1650))

## [0.6.4](https://github.com/alloy-rs/alloy/releases/tag/v0.6.4) - 2024-11-12

### Miscellaneous Tasks

- Release 0.6.4

## [0.6.3](https://github.com/alloy-rs/alloy/releases/tag/v0.6.3) - 2024-11-12

### Miscellaneous Tasks

- Release 0.6.3
- Release 0.6.2 ([#1632](https://github.com/alloy-rs/alloy/issues/1632))

## [0.6.1](https://github.com/alloy-rs/alloy/releases/tag/v0.6.1) - 2024-11-06

### Miscellaneous Tasks

- Release 0.6.1

## [0.6.0](https://github.com/alloy-rs/alloy/releases/tag/v0.6.0) - 2024-11-06

### Bug Fixes

- Add more rlp correctness checks ([#1595](https://github.com/alloy-rs/alloy/issues/1595))
- Make a sensible encoding api ([#1496](https://github.com/alloy-rs/alloy/issues/1496))

### Documentation

- Expand on what `Requests` contains ([#1564](https://github.com/alloy-rs/alloy/issues/1564))

### Features

- [eips] Indexed Blob Hash ([#1526](https://github.com/alloy-rs/alloy/issues/1526))

### Miscellaneous Tasks

- Release 0.6.0
- Make withdrawals pub ([#1623](https://github.com/alloy-rs/alloy/issues/1623))
- Fix some compile issues for no-std test ([#1606](https://github.com/alloy-rs/alloy/issues/1606))

### Other

- Add missing unit test for `MIN_PROTOCOL_BASE_FEE` ([#1558](https://github.com/alloy-rs/alloy/issues/1558))
- Rm `BEACON_CONSENSUS_REORG_UNWIND_DEPTH` ([#1556](https://github.com/alloy-rs/alloy/issues/1556))
- Add unit tests to secure all conversions and impl ([#1544](https://github.com/alloy-rs/alloy/issues/1544))

## [0.5.4](https://github.com/alloy-rs/alloy/releases/tag/v0.5.4) - 2024-10-23

### Bug Fixes

- Sidecar rlp decoding ([#1549](https://github.com/alloy-rs/alloy/issues/1549))

### Miscellaneous Tasks

- Release 0.5.4

### Other

- Add unit test for `amount_wei` `Withdrawal` ([#1551](https://github.com/alloy-rs/alloy/issues/1551))

## [0.5.3](https://github.com/alloy-rs/alloy/releases/tag/v0.5.3) - 2024-10-22

### Bug Fixes

- Correct implementations of Encodable and Decodable for sidecars ([#1528](https://github.com/alloy-rs/alloy/issues/1528))

### Miscellaneous Tasks

- Release 0.5.3

### Other

- Impl `From<RpcBlockHash>` for `BlockId` ([#1539](https://github.com/alloy-rs/alloy/issues/1539))
- Add unit tests and reduce paths ([#1531](https://github.com/alloy-rs/alloy/issues/1531))

## [0.5.2](https://github.com/alloy-rs/alloy/releases/tag/v0.5.2) - 2024-10-18

### Miscellaneous Tasks

- Release 0.5.2

## [0.5.1](https://github.com/alloy-rs/alloy/releases/tag/v0.5.1) - 2024-10-18

### Miscellaneous Tasks

- Release 0.5.1
- Add empty requests constant ([#1519](https://github.com/alloy-rs/alloy/issues/1519))
- Remove 7685 request variants ([#1515](https://github.com/alloy-rs/alloy/issues/1515))
- Remove redundant cfgs ([#1516](https://github.com/alloy-rs/alloy/issues/1516))

## [0.5.0](https://github.com/alloy-rs/alloy/releases/tag/v0.5.0) - 2024-10-18

### Bug Fixes

- [eips] Blob Sidecar Item Serde ([#1441](https://github.com/alloy-rs/alloy/issues/1441))

### Features

- [eip4895] Implement `Withdrawals` ([#1462](https://github.com/alloy-rs/alloy/issues/1462))
- Port generate_blob_sidecar ([#1511](https://github.com/alloy-rs/alloy/issues/1511))
- [eips] Arbitrary BaseFeeParams ([#1432](https://github.com/alloy-rs/alloy/issues/1432))
- `Encodable2718::network_len` ([#1431](https://github.com/alloy-rs/alloy/issues/1431))
- Add helper from impl ([#1407](https://github.com/alloy-rs/alloy/issues/1407))

### Miscellaneous Tasks

- Release 0.5.0
- Update pectra system contracts bytecodes & addresses ([#1512](https://github.com/alloy-rs/alloy/issues/1512))
- Refactor some match with same arms ([#1463](https://github.com/alloy-rs/alloy/issues/1463))
- Update eip-7251 bytecode and address ([#1380](https://github.com/alloy-rs/alloy/issues/1380))
- Remove redundant else ([#1468](https://github.com/alloy-rs/alloy/issues/1468))
- Some small improvements ([#1461](https://github.com/alloy-rs/alloy/issues/1461))

### Other

- Update fn encoded_2718 ([#1475](https://github.com/alloy-rs/alloy/issues/1475))
- Add unit tests for `ConsolidationRequest` ([#1497](https://github.com/alloy-rs/alloy/issues/1497))
- Add unit tests for `WithdrawalRequest` ([#1472](https://github.com/alloy-rs/alloy/issues/1472))
- Add more unit tests ([#1464](https://github.com/alloy-rs/alloy/issues/1464))
- Revert test: update test cases with addresses ([#1358](https://github.com/alloy-rs/alloy/issues/1358)) ([#1444](https://github.com/alloy-rs/alloy/issues/1444))

## [0.4.2](https://github.com/alloy-rs/alloy/releases/tag/v0.4.2) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.2

## [0.4.1](https://github.com/alloy-rs/alloy/releases/tag/v0.4.1) - 2024-10-01

### Bug Fixes

- Safe match for next base fee ([#1399](https://github.com/alloy-rs/alloy/issues/1399))

### Features

- [consensus] Bincode compatibility for EIP-7702 ([#1404](https://github.com/alloy-rs/alloy/issues/1404))

### Miscellaneous Tasks

- Release 0.4.1

## [0.4.0](https://github.com/alloy-rs/alloy/releases/tag/v0.4.0) - 2024-09-30

### Bug Fixes

- Support u64 hex from str for BlockId ([#1396](https://github.com/alloy-rs/alloy/issues/1396))
- Advance buffer during 2718 decoding ([#1367](https://github.com/alloy-rs/alloy/issues/1367))
- `Error::source` for `Eip2718Error` ([#1361](https://github.com/alloy-rs/alloy/issues/1361))

### Features

- Impl From<Eip2718Error> for alloy_rlp::Error ([#1359](https://github.com/alloy-rs/alloy/issues/1359))
- Blob Tx Sidecar Iterator ([#1334](https://github.com/alloy-rs/alloy/issues/1334))

### Miscellaneous Tasks

- Release 0.4.0
- Use std::error

### Other

- Make `Header` blob fees u64 ([#1377](https://github.com/alloy-rs/alloy/issues/1377))
- Make `Header` gas limit u64 ([#1333](https://github.com/alloy-rs/alloy/issues/1333))

### Testing

- Update test cases with addresses ([#1358](https://github.com/alloy-rs/alloy/issues/1358))

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Features

- [rpc-types-beacon] `SignedBidSubmissionV4` ([#1303](https://github.com/alloy-rs/alloy/issues/1303))
- Add blob and proof v1 ([#1300](https://github.com/alloy-rs/alloy/issues/1300))

### Miscellaneous Tasks

- Release 0.3.6
- Release 0.3.5

## [0.3.4](https://github.com/alloy-rs/alloy/releases/tag/v0.3.4) - 2024-09-13

### Features

- Add serde for NumHash ([#1277](https://github.com/alloy-rs/alloy/issues/1277))

### Miscellaneous Tasks

- Release 0.3.4
- Swap `BlockHashOrNumber` alias and struct name ([#1270](https://github.com/alloy-rs/alloy/issues/1270))

## [0.3.3](https://github.com/alloy-rs/alloy/releases/tag/v0.3.3) - 2024-09-10

### Miscellaneous Tasks

- Release 0.3.3
- Swap BlockNumHash alias and struct name ([#1265](https://github.com/alloy-rs/alloy/issues/1265))

## [0.3.2](https://github.com/alloy-rs/alloy/releases/tag/v0.3.2) - 2024-09-09

### Miscellaneous Tasks

- Release 0.3.2
- Add aliases for Num Hash ([#1253](https://github.com/alloy-rs/alloy/issues/1253))
- [eip1898] Display `RpcBlockHash` ([#1242](https://github.com/alloy-rs/alloy/issues/1242))
- Optional derive more ([#1239](https://github.com/alloy-rs/alloy/issues/1239))

## [0.3.1](https://github.com/alloy-rs/alloy/releases/tag/v0.3.1) - 2024-09-02

### Bug Fixes

- [eips] No-std compat ([#1222](https://github.com/alloy-rs/alloy/issues/1222))

### Miscellaneous Tasks

- Release 0.3.1

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

- Release 0.3.0
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

### Bug Fixes

- Deserialization of null storage keys in AccessListItem ([#955](https://github.com/alloy-rs/alloy/issues/955))

### Dependencies

- [eips] Make `alloy-serde` optional under `serde` ([#948](https://github.com/alloy-rs/alloy/issues/948))

### Features

- Add consolidation requests to v4 payload ([#1013](https://github.com/alloy-rs/alloy/issues/1013))
- [eip1559] Support Optimism Canyon hardfork ([#1010](https://github.com/alloy-rs/alloy/issues/1010))
- Impl `From<RpcBlockHash>` for `BlockHashOrNumber` ([#980](https://github.com/alloy-rs/alloy/issues/980))
- Add eip-7702 helpers ([#950](https://github.com/alloy-rs/alloy/issues/950))
- Add eip-7251 system contract address/code ([#956](https://github.com/alloy-rs/alloy/issues/956))

### Miscellaneous Tasks

- Release 0.1.4
- Add helper functions for destructuring auth types ([#1022](https://github.com/alloy-rs/alloy/issues/1022))
- Clean up 7702 encoding ([#1000](https://github.com/alloy-rs/alloy/issues/1000))
- Release 0.1.3
- [eips] Add serde to Authorization types ([#964](https://github.com/alloy-rs/alloy/issues/964))
- [eips] Make `sha2` optional, add `kzg-sidecar` feature ([#949](https://github.com/alloy-rs/alloy/issues/949))

### Testing

- Add missing unit test for op `calc_next_block_base_fee` ([#1008](https://github.com/alloy-rs/alloy/issues/1008))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

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

- Update alloy-eips supported eip list ([#942](https://github.com/alloy-rs/alloy/issues/942))
- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))
- Update descriptions and top level summary ([#128](https://github.com/alloy-rs/alloy/issues/128))

### Features

- Add eip-7251 consolidation request ([#919](https://github.com/alloy-rs/alloy/issues/919))
- Add `BlockId::as_u64` ([#916](https://github.com/alloy-rs/alloy/issues/916))
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

- Release 0.1.2
- Update eip-2935 bytecode and address ([#934](https://github.com/alloy-rs/alloy/issues/934))
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))
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
