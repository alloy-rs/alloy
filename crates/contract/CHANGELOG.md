# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.2](https://github.com/alloy-rs/alloy/releases/tag/v1.4.2) - 2026-01-14

### Miscellaneous Tasks

- Release 1.4.2
- Release 1.4.2

## [1.4.1](https://github.com/alloy-rs/alloy/releases/tag/v1.4.1) - 2026-01-13

### Bug Fixes

- [contract] Deduplicate clear_decoder method ([#3449](https://github.com/alloy-rs/alloy/issues/3449))

### Miscellaneous Tasks

- Release 1.4.1
- Release 1.4.0

### Performance

- [contract] Remove redundant allocation in TransportErrorExt ([#3477](https://github.com/alloy-rs/alloy/issues/3477))

## [1.3.0](https://github.com/alloy-rs/alloy/releases/tag/v1.3.0) - 2026-01-06

### Features

- [`contract`] Add sidecar_7594 to CallBuilder ([#3424](https://github.com/alloy-rs/alloy/issues/3424))

### Miscellaneous Tasks

- Release 1.3.0

## [1.2.1](https://github.com/alloy-rs/alloy/releases/tag/v1.2.1) - 2025-12-23

### Dependencies

- [deps] Run cargo shear ([#3405](https://github.com/alloy-rs/alloy/issues/3405))

### Miscellaneous Tasks

- Release 1.2.1

### Other

- Remove redundant std::self import from CallBuilder ([#3313](https://github.com/alloy-rs/alloy/issues/3313))

## [1.1.3](https://github.com/alloy-rs/alloy/releases/tag/v1.1.3) - 2025-12-06

### Miscellaneous Tasks

- Release 1.1.3
- Remove unnecessary Unpin bound on decoder in EthCall futures ([#3267](https://github.com/alloy-rs/alloy/issues/3267))

### Other

- Add contract eth_call block overrides ([#3233](https://github.com/alloy-rs/alloy/issues/3233))

### Refactor

- [contract] Remove redundant PhantomData from StorageSlotFinder ([#3277](https://github.com/alloy-rs/alloy/issues/3277))

## [1.1.2](https://github.com/alloy-rs/alloy/releases/tag/v1.1.2) - 2025-11-20

### Features

- Add CallBuilder::send,deploy_sync ([#3200](https://github.com/alloy-rs/alloy/issues/3200))

### Miscellaneous Tasks

- Release 1.1.2

## [1.1.1](https://github.com/alloy-rs/alloy/releases/tag/v1.1.1) - 2025-11-13

### Miscellaneous Tasks

- Release 1.1.1

### Styling

- Refactor StorageSlotFinder::find_slot to avoid redundant clones ([#3180](https://github.com/alloy-rs/alloy/issues/3180))

## [1.1.0](https://github.com/alloy-rs/alloy/releases/tag/v1.1.0) - 2025-11-04

### Miscellaneous Tasks

- Release 1.1.0

## [1.0.42](https://github.com/alloy-rs/alloy/releases/tag/v1.0.42) - 2025-10-31

### Features

- Implement CallBuilder::legacy() to prefer legacy by setting gas_price ([#3085](https://github.com/alloy-rs/alloy/issues/3085))

### Miscellaneous Tasks

- Release 1.0.42

## [1.0.41](https://github.com/alloy-rs/alloy/releases/tag/v1.0.41) - 2025-10-17

### Miscellaneous Tasks

- Release 1.0.41
- Release 1.0.41

## [1.0.40](https://github.com/alloy-rs/alloy/releases/tag/v1.0.40) - 2025-10-17

### Miscellaneous Tasks

- Release 1.0.40
- Release 1.0.40

## [1.0.39](https://github.com/alloy-rs/alloy/releases/tag/v1.0.39) - 2025-10-16

### Miscellaneous Tasks

- Release 1.0.39

## [1.0.38](https://github.com/alloy-rs/alloy/releases/tag/v1.0.38) - 2025-10-08

### Miscellaneous Tasks

- Release 1.0.38 ([#3007](https://github.com/alloy-rs/alloy/issues/3007))

## [1.0.37](https://github.com/alloy-rs/alloy/releases/tag/v1.0.37) - 2025-09-30

### Miscellaneous Tasks

- Release 1.0.37
- Remove feature(doc_auto_cfg) ([#2941](https://github.com/alloy-rs/alloy/issues/2941))

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

## [1.0.30](https://github.com/alloy-rs/alloy/releases/tag/v1.0.30) - 2025-09-03

### Features

- Add with_request to storage slot finder ([#2847](https://github.com/alloy-rs/alloy/issues/2847))

### Miscellaneous Tasks

- Release 1.0.30

## [1.0.29](https://github.com/alloy-rs/alloy/releases/tag/v1.0.29) - 2025-09-03

### Miscellaneous Tasks

- Release 1.0.29

## [1.0.28](https://github.com/alloy-rs/alloy/releases/tag/v1.0.28) - 2025-09-02

### Miscellaneous Tasks

- Release 1.0.28

## [1.0.27](https://github.com/alloy-rs/alloy/releases/tag/v1.0.27) - 2025-08-26

### Miscellaneous Tasks

- Release 1.0.27 ([#2822](https://github.com/alloy-rs/alloy/issues/2822))

## [1.0.26](https://github.com/alloy-rs/alloy/releases/tag/v1.0.26) - 2025-08-26

### Miscellaneous Tasks

- Release 1.0.26
- Release 1.0.26

## [1.0.25](https://github.com/alloy-rs/alloy/releases/tag/v1.0.25) - 2025-08-19

### Features

- Add authorization list to CallBuilder ([#2798](https://github.com/alloy-rs/alloy/issues/2798))

### Miscellaneous Tasks

- Release 1.0.25
- Release 1.0.25

## [1.0.24](https://github.com/alloy-rs/alloy/releases/tag/v1.0.24) - 2025-08-06

### Features

- Add value to Multicallitem trait ([#2746](https://github.com/alloy-rs/alloy/issues/2746))

### Miscellaneous Tasks

- Release 1.0.24

## [1.0.23](https://github.com/alloy-rs/alloy/releases/tag/v1.0.23) - 2025-07-22

### Miscellaneous Tasks

- Release 1.0.23

## [1.0.22](https://github.com/alloy-rs/alloy/releases/tag/v1.0.22) - 2025-07-14

### Miscellaneous Tasks

- Release 1.0.22

## [1.0.21](https://github.com/alloy-rs/alloy/releases/tag/v1.0.21) - 2025-07-14

### Miscellaneous Tasks

- Release 1.0.21

## [1.0.20](https://github.com/alloy-rs/alloy/releases/tag/v1.0.20) - 2025-07-09

### Miscellaneous Tasks

- Release 1.0.20

## [1.0.19](https://github.com/alloy-rs/alloy/releases/tag/v1.0.19) - 2025-07-08

### Miscellaneous Tasks

- Release 1.0.19

### Testing

- Add token tests ([#2675](https://github.com/alloy-rs/alloy/issues/2675))

## [1.0.18](https://github.com/alloy-rs/alloy/releases/tag/v1.0.18) - 2025-07-08

### Features

- Added FindStorageSlot  ([#2612](https://github.com/alloy-rs/alloy/issues/2612))

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

### Miscellaneous Tasks

- Release 1.0.13

## [1.0.12](https://github.com/alloy-rs/alloy/releases/tag/v1.0.12) - 2025-06-18

### Features

- [provider] Conversion between `MulticallItem` and `CallItem` ([#2589](https://github.com/alloy-rs/alloy/issues/2589))

### Miscellaneous Tasks

- Release 1.0.12
- Release 1.0.11

## [1.0.10](https://github.com/alloy-rs/alloy/releases/tag/v1.0.10) - 2025-06-17

### Dependencies

- Bump MSRV to 1.85 ([#2547](https://github.com/alloy-rs/alloy/issues/2547))

### Miscellaneous Tasks

- Release 1.0.10
- Release 1.0.10
- Added TryParseTransportErrorResult ([#2530](https://github.com/alloy-rs/alloy/issues/2530))

## [1.0.9](https://github.com/alloy-rs/alloy/releases/tag/v1.0.9) - 2025-05-28

### Features

- Add try decode into error ([#2524](https://github.com/alloy-rs/alloy/issues/2524))

### Miscellaneous Tasks

- Release 1.0.9

## [1.0.8](https://github.com/alloy-rs/alloy/releases/tag/v1.0.8) - 2025-05-27

### Documentation

- [provider] Use multicall.dynamic() in more places ([#2508](https://github.com/alloy-rs/alloy/issues/2508))

### Miscellaneous Tasks

- Release 1.0.8

## [1.0.7](https://github.com/alloy-rs/alloy/releases/tag/v1.0.7) - 2025-05-24

### Miscellaneous Tasks

- Release 1.0.7

## [1.0.6](https://github.com/alloy-rs/alloy/releases/tag/v1.0.6) - 2025-05-21

### Miscellaneous Tasks

- Release 1.0.6

## [1.0.5](https://github.com/alloy-rs/alloy/releases/tag/v1.0.5) - 2025-05-20

### Miscellaneous Tasks

- Release 1.0.5

## [1.0.4](https://github.com/alloy-rs/alloy/releases/tag/v1.0.4) - 2025-05-19

### Miscellaneous Tasks

- Release 1.0.4

## [1.0.3](https://github.com/alloy-rs/alloy/releases/tag/v1.0.3) - 2025-05-15

### Miscellaneous Tasks

- Release 1.0.3 ([#2460](https://github.com/alloy-rs/alloy/issues/2460))
- Release 1.0.2

## [1.0.1](https://github.com/alloy-rs/alloy/releases/tag/v1.0.1) - 2025-05-13

### Miscellaneous Tasks

- Release 1.0.1

## [1.0.0](https://github.com/alloy-rs/alloy/releases/tag/v1.0.0) - 2025-05-13

### Miscellaneous Tasks

- Release 1.0.0

## [0.15.11](https://github.com/alloy-rs/alloy/releases/tag/v0.15.11) - 2025-05-12

### Miscellaneous Tasks

- Release 0.15.11

## [0.15.10](https://github.com/alloy-rs/alloy/releases/tag/v0.15.10) - 2025-05-07

### Miscellaneous Tasks

- Release 0.15.10

### Styling

- Chore : fix typos ([#2398](https://github.com/alloy-rs/alloy/issues/2398))

## [0.15.9](https://github.com/alloy-rs/alloy/releases/tag/v0.15.9) - 2025-05-05

### Miscellaneous Tasks

- Release 0.15.9

## [0.15.8](https://github.com/alloy-rs/alloy/releases/tag/v0.15.8) - 2025-05-02

### Miscellaneous Tasks

- Release 0.15.8

## [0.15.7](https://github.com/alloy-rs/alloy/releases/tag/v0.15.7) - 2025-04-30

### Miscellaneous Tasks

- Release 0.15.7

## [0.15.6](https://github.com/alloy-rs/alloy/releases/tag/v0.15.6) - 2025-04-24

### Miscellaneous Tasks

- Release 0.15.6

## [0.15.5](https://github.com/alloy-rs/alloy/releases/tag/v0.15.5) - 2025-04-24

### Miscellaneous Tasks

- Release 0.15.5
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

### Bug Fixes

- Fix grammar typos in documentation ([#2333](https://github.com/alloy-rs/alloy/issues/2333))

### Miscellaneous Tasks

- Release 0.15.0

### Styling

- [`provider`] Rename `on_*` to `connect_*` ([#2225](https://github.com/alloy-rs/alloy/issues/2225))

## [0.14.0](https://github.com/alloy-rs/alloy/releases/tag/v0.14.0) - 2025-04-09

### Dependencies

- [deps] Core 1.0 ([#2184](https://github.com/alloy-rs/alloy/issues/2184))

### Features

- Relax ProviderBuilder bounds ([#2276](https://github.com/alloy-rs/alloy/issues/2276))

### Miscellaneous Tasks

- Release 0.14.0

### Testing

- Update error handling ([#2277](https://github.com/alloy-rs/alloy/issues/2277))

## [0.13.0](https://github.com/alloy-rs/alloy/releases/tag/v0.13.0) - 2025-03-28

### Miscellaneous Tasks

- Release 0.13.0
- Expect instead of allow ([#2228](https://github.com/alloy-rs/alloy/issues/2228))

### Testing

- Fix inference fail in test ([#2239](https://github.com/alloy-rs/alloy/issues/2239))

## [0.12.6](https://github.com/alloy-rs/alloy/releases/tag/v0.12.6) - 2025-03-18

### Miscellaneous Tasks

- Release 0.12.6

## [0.12.5](https://github.com/alloy-rs/alloy/releases/tag/v0.12.5) - 2025-03-12

### Features

- [`contract`] Build signed and usigned txs from CallBuilder ([#2178](https://github.com/alloy-rs/alloy/issues/2178))

### Miscellaneous Tasks

- Release 0.12.5

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

- [`provider`] Fill txs on `eth_call` ops ([#2092](https://github.com/alloy-rs/alloy/issues/2092))

### Features

- [`provider`] `decode_resp` for `EthCall` ([#2157](https://github.com/alloy-rs/alloy/issues/2157))
- [`eth-call`] Rm borrowing from provider api ([#2127](https://github.com/alloy-rs/alloy/issues/2127))
- [`contract`] Decode as `SolError` ([#2072](https://github.com/alloy-rs/alloy/issues/2072))
- [`contract`] Handle reverts ([#2058](https://github.com/alloy-rs/alloy/issues/2058))

### Miscellaneous Tasks

- Release 0.12.0
- Use impl Into StateOverride ([#2145](https://github.com/alloy-rs/alloy/issues/2145))
- Rename `on_builtin` to `connect` ([#2078](https://github.com/alloy-rs/alloy/issues/2078))
- Update url ([#2071](https://github.com/alloy-rs/alloy/issues/2071))

## [0.11.1](https://github.com/alloy-rs/alloy/releases/tag/v0.11.1) - 2025-02-12

### Bug Fixes

- [`multicall`] Impl Error for `Failure` +  clear returns `Empty` builder. ([#2043](https://github.com/alloy-rs/alloy/issues/2043))
- Don't validate when ABI decoding ([#2041](https://github.com/alloy-rs/alloy/issues/2041))

### Features

- [`provider`] Multicall ([#2010](https://github.com/alloy-rs/alloy/issues/2010))
- Add helpers for account overrides ([#2040](https://github.com/alloy-rs/alloy/issues/2040))

### Miscellaneous Tasks

- Release 0.11.1

## [0.11.0](https://github.com/alloy-rs/alloy/releases/tag/v0.11.0) - 2025-01-31

### Bug Fixes

- [`contract`] Rm IntoFuture for CallBuilder ([#1945](https://github.com/alloy-rs/alloy/issues/1945))

### Documentation

- Enable some useful rustdoc features on docs.rs ([#1890](https://github.com/alloy-rs/alloy/issues/1890))

### Features

- [`provider`] Instantiate recommended fillers by default ([#1901](https://github.com/alloy-rs/alloy/issues/1901))
- [contract] Improve 'no data' error message ([#1898](https://github.com/alloy-rs/alloy/issues/1898))
- Remove T: Transport from public APIs ([#1859](https://github.com/alloy-rs/alloy/issues/1859))

### Miscellaneous Tasks

- Release 0.11.0
- Release 0.10.0

## [0.9.2](https://github.com/alloy-rs/alloy/releases/tag/v0.9.2) - 2025-01-03

### Miscellaneous Tasks

- Release 0.9.2

## [0.9.1](https://github.com/alloy-rs/alloy/releases/tag/v0.9.1) - 2024-12-30

### Miscellaneous Tasks

- Release 0.9.1

## [0.9.0](https://github.com/alloy-rs/alloy/releases/tag/v0.9.0) - 2024-12-30

### Miscellaneous Tasks

- Release 0.9.0

## [0.8.3](https://github.com/alloy-rs/alloy/releases/tag/v0.8.3) - 2024-12-20

### Miscellaneous Tasks

- Release 0.8.3

## [0.8.2](https://github.com/alloy-rs/alloy/releases/tag/v0.8.2) - 2024-12-19

### Miscellaneous Tasks

- Release 0.8.2

## [0.8.1](https://github.com/alloy-rs/alloy/releases/tag/v0.8.1) - 2024-12-16

### Miscellaneous Tasks

- Release 0.8.1

## [0.8.0](https://github.com/alloy-rs/alloy/releases/tag/v0.8.0) - 2024-12-10

### Miscellaneous Tasks

- Release 0.8.0 ([#1778](https://github.com/alloy-rs/alloy/issues/1778))

## [0.7.3](https://github.com/alloy-rs/alloy/releases/tag/v0.7.3) - 2024-12-05

### Miscellaneous Tasks

- Release 0.7.3
- Release 0.7.2 ([#1729](https://github.com/alloy-rs/alloy/issues/1729))

## [0.7.0](https://github.com/alloy-rs/alloy/releases/tag/v0.7.0) - 2024-11-28

### Bug Fixes

- [provider] Use `BoxTransport` in `on_anvil_*` ([#1693](https://github.com/alloy-rs/alloy/issues/1693))

### Miscellaneous Tasks

- Release 0.7.0
- Release 0.7.0
- Release 0.7.0
- Make clippy happy ([#1677](https://github.com/alloy-rs/alloy/issues/1677))

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

### Miscellaneous Tasks

- Release 0.6.0

### Other

- Embed TxEnvelope into `rpc-types-eth::Transaction` ([#1460](https://github.com/alloy-rs/alloy/issues/1460))

## [0.5.4](https://github.com/alloy-rs/alloy/releases/tag/v0.5.4) - 2024-10-23

### Miscellaneous Tasks

- Release 0.5.4

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

### Features

- Make Pending transaction own the provider ([#1500](https://github.com/alloy-rs/alloy/issues/1500))

### Miscellaneous Tasks

- Release 0.5.0
- Some lifetime simplifications ([#1467](https://github.com/alloy-rs/alloy/issues/1467))

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

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Features

- ProviderCall ([#788](https://github.com/alloy-rs/alloy/issues/788))

### Miscellaneous Tasks

- Release 0.3.6
- Release 0.3.5

### Refactor

- Separate transaction builders for tx types ([#1259](https://github.com/alloy-rs/alloy/issues/1259))

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

- Return more user-friendly error on tx timeout ([#1145](https://github.com/alloy-rs/alloy/issues/1145))

### Miscellaneous Tasks

- Release 0.3.0
- Release 0.2.1
- Release 0.2.0
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))

### Refactor

- Add network-primitives ([#1101](https://github.com/alloy-rs/alloy/issues/1101))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Features

- [contract] Implement Filter's builder methods on Event ([#960](https://github.com/alloy-rs/alloy/issues/960))

### Miscellaneous Tasks

- Release 0.1.4
- Release 0.1.3

### Other

- Allow to convert CallBuilderTo TransactionRequest ([#981](https://github.com/alloy-rs/alloy/issues/981))
- [contract] Support state overrides for gas estimation ([#967](https://github.com/alloy-rs/alloy/issues/967))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Bug Fixes

- [contract] Set `to` when calling with ContractInstance ([#913](https://github.com/alloy-rs/alloy/issues/913))
- Correctly serialize eth_call params ([#778](https://github.com/alloy-rs/alloy/issues/778))
- Sol macro generated event filters were not filtering ([#600](https://github.com/alloy-rs/alloy/issues/600))
- Change nonce from `U64` to `u64`  ([#341](https://github.com/alloy-rs/alloy/issues/341))
- Alloy core patches

### Dependencies

- [deps] Bump `alloy-core` to `0.7.6` (latest), fix broken test and violated deny ([#862](https://github.com/alloy-rs/alloy/issues/862))

### Documentation

- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- [rpc] Split off `eth` namespace in `alloy-rpc-types` to `alloy-rpc-types-eth` ([#847](https://github.com/alloy-rs/alloy/issues/847))
- [rpc] Add remaining anvil rpc methods to provider ([#831](https://github.com/alloy-rs/alloy/issues/831))
- Feat(contract) : add reference to TransactionRequest type ([#828](https://github.com/alloy-rs/alloy/issues/828))
- Add overrides to eth_estimateGas ([#802](https://github.com/alloy-rs/alloy/issues/802))
- Duplicate funtions of  in crates/contract/src/call.rs ([#534](https://github.com/alloy-rs/alloy/issues/534)) ([#726](https://github.com/alloy-rs/alloy/issues/726))
- Eth_call builder  ([#645](https://github.com/alloy-rs/alloy/issues/645))
- Support changing CallBuilder decoders ([#641](https://github.com/alloy-rs/alloy/issues/641))
- Add set_sidecar to the callbuilder ([#594](https://github.com/alloy-rs/alloy/issues/594))
- Joinable transaction fillers ([#426](https://github.com/alloy-rs/alloy/issues/426))
- Default to Ethereum network in `alloy-provider` and `alloy-contract` ([#356](https://github.com/alloy-rs/alloy/issues/356))
- Embed primitives Log in rpc Log and consensus Receipt in rpc Receipt ([#396](https://github.com/alloy-rs/alloy/issues/396))
- Make HTTP provider optional ([#379](https://github.com/alloy-rs/alloy/issues/379))
- `Provider::subscribe_logs` ([#339](https://github.com/alloy-rs/alloy/issues/339))
- Merge Provider traits into one ([#297](https://github.com/alloy-rs/alloy/issues/297))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Network abstraction and transaction builder ([#190](https://github.com/alloy-rs/alloy/issues/190))
- Add `alloy` prelude crate ([#203](https://github.com/alloy-rs/alloy/issues/203))
- Alloy-contract ([#182](https://github.com/alloy-rs/alloy/issues/182))

### Miscellaneous Tasks

- Release 0.1.2
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))
- Fix warnings, check-cfg ([#776](https://github.com/alloy-rs/alloy/issues/776))
- Get_transaction_by_hash returns Option<Transaction> ([#714](https://github.com/alloy-rs/alloy/issues/714))
- Remove outdated comment ([#640](https://github.com/alloy-rs/alloy/issues/640))
- Document how state overrides work in `call` and `call_raw` ([#570](https://github.com/alloy-rs/alloy/issues/570))
- Clippy, warnings ([#504](https://github.com/alloy-rs/alloy/issues/504))
- Clippy ([#208](https://github.com/alloy-rs/alloy/issues/208))

### Other

- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Update clippy warnings ([#765](https://github.com/alloy-rs/alloy/issues/765))
- RpcWithBlock ([#674](https://github.com/alloy-rs/alloy/issues/674))
- Use Self when possible ([#711](https://github.com/alloy-rs/alloy/issues/711))
- [Refactor] Delete the internal-test-utils crate ([#632](https://github.com/alloy-rs/alloy/issues/632))
- [Call] Added more fields for call builder ([#625](https://github.com/alloy-rs/alloy/issues/625))
- Numeric type audit: network, consensus, provider, rpc-types ([#454](https://github.com/alloy-rs/alloy/issues/454))
- Check no_std in CI ([#367](https://github.com/alloy-rs/alloy/issues/367))
- Rename `alloy-providers` to `alloy-provider` ([#278](https://github.com/alloy-rs/alloy/issues/278))
- ClientRefs, Poller, and Streams ([#179](https://github.com/alloy-rs/alloy/issues/179))

### Refactor

- [signers] Use `signer` for single credentials and `wallet` for credential stores  ([#883](https://github.com/alloy-rs/alloy/issues/883))
- Make optional BlockId params required in provider functions ([#516](https://github.com/alloy-rs/alloy/issues/516))
- Dedupe `CallRequest`/`TransactionRequest` ([#178](https://github.com/alloy-rs/alloy/issues/178))

### Styling

- [Blocked] Update TransactionRequest's `to` field to TxKind ([#553](https://github.com/alloy-rs/alloy/issues/553))
- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))

### Testing

- Ignore instead of commenting a test ([#207](https://github.com/alloy-rs/alloy/issues/207))

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
