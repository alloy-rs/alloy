# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- Make `Block::hash` required ([#1205](https://github.com/alloy-rs/alloy/issues/1205))
- [provider] Serialize no parameters as `[]` instead of `null` ([#1193](https://github.com/alloy-rs/alloy/issues/1193))
- Use `server_id` when unsubscribing ([#1182](https://github.com/alloy-rs/alloy/issues/1182))
- Return more user-friendly error on tx timeout ([#1145](https://github.com/alloy-rs/alloy/issues/1145))
- Use `BlockId` superset over `BlockNumberOrTag` where applicable  ([#1135](https://github.com/alloy-rs/alloy/issues/1135))
- [provider] Prevent panic from having 0 keys when calling `on_anvil_with_wallet_and_config` ([#1055](https://github.com/alloy-rs/alloy/issues/1055))
- [provider] Do not overflow LRU cache capacity in ChainStreamPoller ([#1052](https://github.com/alloy-rs/alloy/issues/1052))
- [admin] Id in NodeInfo is string instead of B256 ([#1038](https://github.com/alloy-rs/alloy/issues/1038))

### Dependencies

- [deps] Bump some deps ([#1141](https://github.com/alloy-rs/alloy/issues/1141))
- Revert "chore(deps): bump some deps"
- [deps] Bump some deps

### Features

- Add erc4337 endpoint methods to provider ([#1176](https://github.com/alloy-rs/alloy/issues/1176))
- Add block and transaction generics to otterscan and txpool types ([#1183](https://github.com/alloy-rs/alloy/issues/1183))
- Network-parameterized block responses ([#1106](https://github.com/alloy-rs/alloy/issues/1106))
- Add get raw transaction by hash ([#1168](https://github.com/alloy-rs/alloy/issues/1168))
- Add rpc namespace ([#994](https://github.com/alloy-rs/alloy/issues/994))

### Miscellaneous Tasks

- Clippy für docs ([#1194](https://github.com/alloy-rs/alloy/issues/1194))
- Release 0.2.1
- Correctly cfg unused type ([#1117](https://github.com/alloy-rs/alloy/issues/1117))
- Release 0.2.0
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))

### Other

- Add `AccessListResult` type (EIP-2930) ([#1110](https://github.com/alloy-rs/alloy/issues/1110))
- Removing async get account ([#1080](https://github.com/alloy-rs/alloy/issues/1080))

### Refactor

- Add network-primitives ([#1101](https://github.com/alloy-rs/alloy/issues/1101))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Bug Fixes

- Fix watching already mined transactions ([#997](https://github.com/alloy-rs/alloy/issues/997))

### Features

- Add missing admin_* methods ([#991](https://github.com/alloy-rs/alloy/issues/991))
- Support web3_sha3 provider function ([#996](https://github.com/alloy-rs/alloy/issues/996))
- Add trace_get ([#987](https://github.com/alloy-rs/alloy/issues/987))
- Add net rpc namespace ([#989](https://github.com/alloy-rs/alloy/issues/989))
- Add missing debug_* rpc methods ([#986](https://github.com/alloy-rs/alloy/issues/986))

### Miscellaneous Tasks

- Release 0.1.4
- [provider] Simplify nonce filler ([#976](https://github.com/alloy-rs/alloy/issues/976))

### Testing

- Fix flaky anvil test ([#992](https://github.com/alloy-rs/alloy/issues/992))

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Bug Fixes

- Enable tls12 in rustls ([#952](https://github.com/alloy-rs/alloy/issues/952))

### Features

- Add trace_filter method ([#946](https://github.com/alloy-rs/alloy/issues/946))

### Miscellaneous Tasks

- Release 0.1.3
- Nightly clippy ([#947](https://github.com/alloy-rs/alloy/issues/947))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Documentation

- Update get_balance docs ([#938](https://github.com/alloy-rs/alloy/issues/938))
- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- Add trace_raw_transaction and trace_replay_block_transactions ([#925](https://github.com/alloy-rs/alloy/issues/925))
- [provider] Support ethCall optional blockId serialization ([#900](https://github.com/alloy-rs/alloy/issues/900))

### Miscellaneous Tasks

- Release 0.1.2
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- Downgrade tokio-tungstenite ([#881](https://github.com/alloy-rs/alloy/issues/881))
- Set minimal priority fee to 1 wei ([#808](https://github.com/alloy-rs/alloy/issues/808))
- Use envelopes in get_payload API ([#807](https://github.com/alloy-rs/alloy/issues/807))
- Return ExecutionPayloadV3 from get_payload_v3 ([#803](https://github.com/alloy-rs/alloy/issues/803))
- Correctly serialize eth_call params ([#778](https://github.com/alloy-rs/alloy/issues/778))
- Debug_trace arguments ([#730](https://github.com/alloy-rs/alloy/issues/730))
- Use U64 for feeHistory blocknumber ([#694](https://github.com/alloy-rs/alloy/issues/694))
- [provider] Map to primitive u128 ([#678](https://github.com/alloy-rs/alloy/issues/678))
- More abstraction for block transactions ([#666](https://github.com/alloy-rs/alloy/issues/666))
- Signer filler now propagates missing keys from builder ([#637](https://github.com/alloy-rs/alloy/issues/637))
- Better tx receipt mitigation ([#614](https://github.com/alloy-rs/alloy/issues/614))
- [provider] Uncle methods for block hash ([#587](https://github.com/alloy-rs/alloy/issues/587))
- [provider/debug] Arg type in debug_trace_call ([#585](https://github.com/alloy-rs/alloy/issues/585))
- Signer fills from if unset ([#555](https://github.com/alloy-rs/alloy/issues/555))
- Tmp fix for PendingTransactionBuilder::get_receipt ([#558](https://github.com/alloy-rs/alloy/issues/558))
- Conflict between to change and debug tests ([#550](https://github.com/alloy-rs/alloy/issues/550))
- Dont use fuse::select_next_some ([#532](https://github.com/alloy-rs/alloy/issues/532))
- Eip1559 estimator ([#509](https://github.com/alloy-rs/alloy/issues/509))
- Correctly treat `confirmation` for `watch_pending_transaction` ([#381](https://github.com/alloy-rs/alloy/issues/381))
- Remove app-layer usage of transport error ([#363](https://github.com/alloy-rs/alloy/issues/363))
- [provider] 0x prefix in sendRawTransaction ([#369](https://github.com/alloy-rs/alloy/issues/369))
- Change nonce from `U64` to `u64`  ([#341](https://github.com/alloy-rs/alloy/issues/341))
- Make `TransactionReceipt::transaction_hash` field mandatory ([#337](https://github.com/alloy-rs/alloy/issues/337))
- Fix subscribe blocks ([#330](https://github.com/alloy-rs/alloy/issues/330))

### Documentation

- [provider] Add examples to `raw_request{,dyn}` ([#486](https://github.com/alloy-rs/alloy/issues/486))
- Add aliases to `get_transaction_count` ([#420](https://github.com/alloy-rs/alloy/issues/420))
- More docs in `alloy-providers` ([#281](https://github.com/alloy-rs/alloy/issues/281))
- Add readmes

### Features

- Add trace_replay_transaction ([#908](https://github.com/alloy-rs/alloy/issues/908))
- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- [alloy] Add `"full"` feature flag ([#877](https://github.com/alloy-rs/alloy/issues/877))
- [provider] Expose `ProviderBuilder` via `fn builder()` ([#858](https://github.com/alloy-rs/alloy/issues/858))
- [rpc] Split off `eth` namespace in `alloy-rpc-types` to `alloy-rpc-types-eth` ([#847](https://github.com/alloy-rs/alloy/issues/847))
- Add engine API v4 methods ([#853](https://github.com/alloy-rs/alloy/issues/853))
- Send_envelope ([#851](https://github.com/alloy-rs/alloy/issues/851))
- [rpc] Add remaining anvil rpc methods to provider ([#831](https://github.com/alloy-rs/alloy/issues/831))
- [rpc] Use `BlockTransactionsKind` enum instead of bool for full arguments ([#840](https://github.com/alloy-rs/alloy/issues/840))
- Full block ambiguity ([#832](https://github.com/alloy-rs/alloy/issues/832))
- Method on `Provider` to make a new `N::TransactionRequest` ([#812](https://github.com/alloy-rs/alloy/issues/812))
- Add overrides to eth_estimateGas ([#802](https://github.com/alloy-rs/alloy/issues/802))
- [`provider`] `eth_getAccount` support ([#760](https://github.com/alloy-rs/alloy/issues/760))
- Set poll interval based on connected chain ([#767](https://github.com/alloy-rs/alloy/issues/767))
- Block id convenience functions ([#757](https://github.com/alloy-rs/alloy/issues/757))
- Add `EngineApi` extension trait ([#676](https://github.com/alloy-rs/alloy/issues/676))
- Eth_call builder  ([#645](https://github.com/alloy-rs/alloy/issues/645))
- AnvilProvider ([#611](https://github.com/alloy-rs/alloy/issues/611))
- Allow to only fill a transaction request ([#590](https://github.com/alloy-rs/alloy/issues/590))
- WalletProvider ([#569](https://github.com/alloy-rs/alloy/issues/569))
- Refactor request builder workflow ([#431](https://github.com/alloy-rs/alloy/issues/431))
- [provider] `debug_*` methods ([#548](https://github.com/alloy-rs/alloy/issues/548))
- [provider] Geth `txpool_*` methods ([#546](https://github.com/alloy-rs/alloy/issues/546))
- [provider] Get_uncle_count ([#524](https://github.com/alloy-rs/alloy/issues/524))
- Joinable transaction fillers ([#426](https://github.com/alloy-rs/alloy/issues/426))
- `std` feature flag for `alloy-consensus` ([#461](https://github.com/alloy-rs/alloy/issues/461))
- Rename alloy-rpc-*-types to alloy-rpc-types-* ([#435](https://github.com/alloy-rs/alloy/issues/435))
- Improve and complete `alloy` prelude crate feature flag compatiblity ([#421](https://github.com/alloy-rs/alloy/issues/421))
- Default to Ethereum network in `alloy-provider` and `alloy-contract` ([#356](https://github.com/alloy-rs/alloy/issues/356))
- Embed primitives Log in rpc Log and consensus Receipt in rpc Receipt ([#396](https://github.com/alloy-rs/alloy/issues/396))
- Make HTTP provider optional ([#379](https://github.com/alloy-rs/alloy/issues/379))
- Implement `admin_trait`  ([#405](https://github.com/alloy-rs/alloy/issues/405))
- Handle 4844 fee ([#412](https://github.com/alloy-rs/alloy/issues/412))
- [providers] Connect_boxed api ([#342](https://github.com/alloy-rs/alloy/issues/342))
- Convenience functions for nonce and gas on `ProviderBuilder` ([#378](https://github.com/alloy-rs/alloy/issues/378))
- Add eth_blobBaseFee and eth_maxPriorityFeePerGas ([#380](https://github.com/alloy-rs/alloy/issues/380))
- `Provider::subscribe_logs` ([#339](https://github.com/alloy-rs/alloy/issues/339))
- [layers] GasEstimationLayer ([#326](https://github.com/alloy-rs/alloy/issues/326))
- [json-rpc] Use `Cow` instead of `&'static str` for method names ([#319](https://github.com/alloy-rs/alloy/issues/319))
- Update priority fee estimator ([#316](https://github.com/alloy-rs/alloy/issues/316))
- Move local signers to a separate crate, fix wasm ([#306](https://github.com/alloy-rs/alloy/issues/306))
- Default to Ethereum network in `ProviderBuilder` ([#304](https://github.com/alloy-rs/alloy/issues/304))
- Merge Provider traits into one ([#297](https://github.com/alloy-rs/alloy/issues/297))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Nonce filling layer ([#276](https://github.com/alloy-rs/alloy/issues/276))
- `trace_call` and `trace_callMany` ([#277](https://github.com/alloy-rs/alloy/issues/277))

### Miscellaneous Tasks

- [clippy] Apply lint suggestions ([#903](https://github.com/alloy-rs/alloy/issues/903))
- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [provider] Reorder methods in `Provider` trait ([#863](https://github.com/alloy-rs/alloy/issues/863))
- [provider] Document privileged status of EIP-1559 ([#850](https://github.com/alloy-rs/alloy/issues/850))
- [docs] Crate completeness and fix typos ([#861](https://github.com/alloy-rs/alloy/issues/861))
- [docs] Add doc aliases ([#843](https://github.com/alloy-rs/alloy/issues/843))
- Move trace to extension trait ([#818](https://github.com/alloy-rs/alloy/issues/818))
- Fix remaining warnings, add TODO for proptest-derive ([#819](https://github.com/alloy-rs/alloy/issues/819))
- Get_transaction_by_hash returns Option<Transaction> ([#714](https://github.com/alloy-rs/alloy/issues/714))
- [general] Add CI workflow for Windows + fix IPC test ([#642](https://github.com/alloy-rs/alloy/issues/642))
- Add Default to GasEstimatorLayer ([#410](https://github.com/alloy-rs/alloy/issues/410))
- Rename `RpcClient::prepare` to `request` ([#299](https://github.com/alloy-rs/alloy/issues/299))

### Other

- [feat] Synchronous filling ([#841](https://github.com/alloy-rs/alloy/issues/841))
- RecommendFiller -> RecommendedFiller, move to fillers ([#825](https://github.com/alloy-rs/alloy/issues/825))
- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Update clippy warnings ([#765](https://github.com/alloy-rs/alloy/issues/765))
- RpcWithBlock ([#674](https://github.com/alloy-rs/alloy/issues/674))
- Use Self when possible ([#711](https://github.com/alloy-rs/alloy/issues/711))
- Small refactor ([#652](https://github.com/alloy-rs/alloy/issues/652))
- Use `From<Address>` for `TxKind` ([#651](https://github.com/alloy-rs/alloy/issues/651))
- [Refactor] Move Provider into its own module ([#644](https://github.com/alloy-rs/alloy/issues/644))
- [Refactor] Delete the internal-test-utils crate ([#632](https://github.com/alloy-rs/alloy/issues/632))
- Expose SendableTx in providers ([#601](https://github.com/alloy-rs/alloy/issues/601))
- Temp get_uncle fix ([#589](https://github.com/alloy-rs/alloy/issues/589))
- Revert "chore: remove outdated license ([#510](https://github.com/alloy-rs/alloy/issues/510))" ([#513](https://github.com/alloy-rs/alloy/issues/513))
- Enable default-tls for alloy-provider/reqwest feature ([#483](https://github.com/alloy-rs/alloy/issues/483))
- Extension ([#474](https://github.com/alloy-rs/alloy/issues/474))
- Removed reqwest prefix ([#462](https://github.com/alloy-rs/alloy/issues/462))
- Numeric type audit: network, consensus, provider, rpc-types ([#454](https://github.com/alloy-rs/alloy/issues/454))
- Adds `check -Zcheck-cfg ` job ([#419](https://github.com/alloy-rs/alloy/issues/419))
- Use latest stable
- Rename `alloy-providers` to `alloy-provider` ([#278](https://github.com/alloy-rs/alloy/issues/278))
- Merge pull request [#3](https://github.com/alloy-rs/alloy/issues/3) from alloy-rs/prestwich/readme-and-cleanup
- Merge pull request [#2](https://github.com/alloy-rs/alloy/issues/2) from alloy-rs/prestwich/transports
- Rename middleware to provider

### Performance

- Remove getBlock request in feeHistory ([#414](https://github.com/alloy-rs/alloy/issues/414))

### Refactor

- [rpc] Extract `admin` and `txpool` into their respective crate ([#898](https://github.com/alloy-rs/alloy/issues/898))
- [signers] Use `signer` for single credentials and `wallet` for credential stores  ([#883](https://github.com/alloy-rs/alloy/issues/883))
- Improve eth_call internals ([#763](https://github.com/alloy-rs/alloy/issues/763))
- Change u64 to Duration ([#636](https://github.com/alloy-rs/alloy/issues/636))
- Make optional BlockId params required in provider functions ([#516](https://github.com/alloy-rs/alloy/issues/516))
- Rename to reqd_confs ([#353](https://github.com/alloy-rs/alloy/issues/353))

### Styling

- [Blocked] Update TransactionRequest's `to` field to TxKind ([#553](https://github.com/alloy-rs/alloy/issues/553))
- Remove outdated license ([#510](https://github.com/alloy-rs/alloy/issues/510))
- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))
- Rename `ManagedNonceLayer` to `NonceManagerLayer` ([#415](https://github.com/alloy-rs/alloy/issues/415))
- Eip1559Estimation return type ([#352](https://github.com/alloy-rs/alloy/issues/352))

### Testing

- Add rand feature in providers ([#910](https://github.com/alloy-rs/alloy/issues/910))

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
