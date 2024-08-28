# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/alloy-rs/alloy/releases/tag/v0.3.0) - 2024-08-28

### Bug Fixes

- Make `Block::hash` required ([#1205](https://github.com/alloy-rs/alloy/issues/1205))
- Remove optimism-related types ([#1203](https://github.com/alloy-rs/alloy/issues/1203))
- Use `impl From<RangeInclusive> for FilterBlockOption` instead of `Range`  ([#1199](https://github.com/alloy-rs/alloy/issues/1199))
- Serde for `depositReceiptVersion` ([#1196](https://github.com/alloy-rs/alloy/issues/1196))
- [provider] Serialize no parameters as `[]` instead of `null` ([#1193](https://github.com/alloy-rs/alloy/issues/1193))
- Change generics order for `Block` ([#1192](https://github.com/alloy-rs/alloy/issues/1192))
- Add missing op fields ([#1187](https://github.com/alloy-rs/alloy/issues/1187))
- Use `server_id` when unsubscribing ([#1182](https://github.com/alloy-rs/alloy/issues/1182))
- Allow arbitrary strings in subscription ids ([#1163](https://github.com/alloy-rs/alloy/issues/1163))
- Remove `OtherFields` from Transaction and Block ([#1154](https://github.com/alloy-rs/alloy/issues/1154))
- [rpc-types-eth] Match 7702 in TxReceipt.status() ([#1149](https://github.com/alloy-rs/alloy/issues/1149))
- Return more user-friendly error on tx timeout ([#1145](https://github.com/alloy-rs/alloy/issues/1145))
- [doc] Correct order of fields ([#1139](https://github.com/alloy-rs/alloy/issues/1139))
- Use `BlockId` superset over `BlockNumberOrTag` where applicable  ([#1135](https://github.com/alloy-rs/alloy/issues/1135))
- [rpc] Show data in when cast send result in custom error ([#1129](https://github.com/alloy-rs/alloy/issues/1129))
- Make Parity TraceResults output optional ([#1102](https://github.com/alloy-rs/alloy/issues/1102))
- Correctly trim eip7251 bytecode ([#1105](https://github.com/alloy-rs/alloy/issues/1105))
- [eips] Make SignedAuthorizationList arbitrary less fallible ([#1084](https://github.com/alloy-rs/alloy/issues/1084))
- [node-bindings] Backport fix from ethers-rs ([#1081](https://github.com/alloy-rs/alloy/issues/1081))
- Trim conflicting key `max_fee_per_blob_gas` from Eip1559 tx type ([#1064](https://github.com/alloy-rs/alloy/issues/1064))
- [provider] Prevent panic from having 0 keys when calling `on_anvil_with_wallet_and_config` ([#1055](https://github.com/alloy-rs/alloy/issues/1055))
- Require storageKeys value broken bincode serialization from [#955](https://github.com/alloy-rs/alloy/issues/955) ([#1058](https://github.com/alloy-rs/alloy/issues/1058))
- [provider] Do not overflow LRU cache capacity in ChainStreamPoller ([#1052](https://github.com/alloy-rs/alloy/issues/1052))
- [admin] Id in NodeInfo is string instead of B256 ([#1038](https://github.com/alloy-rs/alloy/issues/1038))
- Cargo fmt ([#1044](https://github.com/alloy-rs/alloy/issues/1044))
- [eip7702] Add correct rlp decode/encode ([#1034](https://github.com/alloy-rs/alloy/issues/1034))

### Dependencies

- Rm 2930 and 7702 - use alloy-rs/eips ([#1181](https://github.com/alloy-rs/alloy/issues/1181))
- Bump core and rm ssz feat ([#1167](https://github.com/alloy-rs/alloy/issues/1167))
- [deps] Bump some deps ([#1141](https://github.com/alloy-rs/alloy/issues/1141))
- Revert "chore(deps): bump some deps"
- [deps] Bump some deps
- Bump jsonrpsee 0.24 ([#1067](https://github.com/alloy-rs/alloy/issues/1067))
- [deps] Bump Trezor client to `=0.1.4` to fix signing bug ([#1045](https://github.com/alloy-rs/alloy/issues/1045))

### Documentation

- Readme fix ([#1114](https://github.com/alloy-rs/alloy/issues/1114))
- Update links to use docs.rs ([#1066](https://github.com/alloy-rs/alloy/issues/1066))

### Features

- Add error for pre prague requests ([#1204](https://github.com/alloy-rs/alloy/issues/1204))
- [transport] Retry http errors with 503 status code ([#1164](https://github.com/alloy-rs/alloy/issues/1164))
- Add erc4337 endpoint methods to provider ([#1176](https://github.com/alloy-rs/alloy/issues/1176))
- Add block and transaction generics to otterscan and txpool types ([#1183](https://github.com/alloy-rs/alloy/issues/1183))
- Make block struct generic over header type ([#1179](https://github.com/alloy-rs/alloy/issues/1179))
- [rpc-types] `debug_executionWitness` ([#1178](https://github.com/alloy-rs/alloy/issues/1178))
- Network-parameterized block responses ([#1106](https://github.com/alloy-rs/alloy/issues/1106))
- Add get raw transaction by hash ([#1168](https://github.com/alloy-rs/alloy/issues/1168))
- [geth/trace] Add field log.position ([#1150](https://github.com/alloy-rs/alloy/issues/1150))
- Make signature methods generic over EncodableSignature ([#1138](https://github.com/alloy-rs/alloy/issues/1138))
- Add 7702 tx enum ([#1059](https://github.com/alloy-rs/alloy/issues/1059))
- Add authorization list to TransactionRequest ([#1125](https://github.com/alloy-rs/alloy/issues/1125))
- [engine-types] `PayloadError::PrePragueBlockWithEip7702Transactions` ([#1116](https://github.com/alloy-rs/alloy/issues/1116))
- Use EncodableSignature for tx encoding ([#1100](https://github.com/alloy-rs/alloy/issues/1100))
- Eth_simulateV1 Request / Response types ([#1042](https://github.com/alloy-rs/alloy/issues/1042))
- Add helper for decoding custom errors ([#1098](https://github.com/alloy-rs/alloy/issues/1098))
- Enable more features transitively in meta crate ([#1097](https://github.com/alloy-rs/alloy/issues/1097))
- [rpc/trace] Filter matches with trace ([#1090](https://github.com/alloy-rs/alloy/issues/1090))
- Feat(rpc-type-eth) convert vec TxReq to bundle ([#1091](https://github.com/alloy-rs/alloy/issues/1091))
- [eip] Make 7702 auth recovery fallible ([#1082](https://github.com/alloy-rs/alloy/issues/1082))
- [json-rpc] Implement `From<u64> for Id` and `From<String> for Id` ([#1088](https://github.com/alloy-rs/alloy/issues/1088))
- [consensus] Add `From<ConsolidationRequest>` for `Request` ([#1083](https://github.com/alloy-rs/alloy/issues/1083))
- Feat(provider) : introduction to eth_sendRawTransactionConditional  RPC endpoint type ([#1009](https://github.com/alloy-rs/alloy/issues/1009))
- Expose encoded_len_with_signature() ([#1063](https://github.com/alloy-rs/alloy/issues/1063))
- Add 7702 tx type ([#1046](https://github.com/alloy-rs/alloy/issues/1046))
- [rpc-types-eth] Serde flatten `BlobTransactionSidecar` in tx req ([#1054](https://github.com/alloy-rs/alloy/issues/1054))
- Add authorization list to rpc transaction and tx receipt types ([#1051](https://github.com/alloy-rs/alloy/issues/1051))
- Impl `arbitrary` for tx structs ([#1050](https://github.com/alloy-rs/alloy/issues/1050))
- [core] Update core version ([#1049](https://github.com/alloy-rs/alloy/issues/1049))
- [otterscan] Add ots slim block and serialze OperationType to int ([#1043](https://github.com/alloy-rs/alloy/issues/1043))
- Generate valid signed auth signatures ([#1041](https://github.com/alloy-rs/alloy/issues/1041))
- Add `rpc-types-mev` feature to meta crate ([#1040](https://github.com/alloy-rs/alloy/issues/1040))
- Add arbitrary to auth ([#1036](https://github.com/alloy-rs/alloy/issues/1036))
- [genesis] Rm EIP150Hash ([#1039](https://github.com/alloy-rs/alloy/issues/1039))
- Add hash for 7702 ([#1037](https://github.com/alloy-rs/alloy/issues/1037))
- Add rpc namespace ([#994](https://github.com/alloy-rs/alloy/issues/994))

### Miscellaneous Tasks

- [consensus] Add missing getter trait methods for `alloy_consensus::Transaction` ([#1197](https://github.com/alloy-rs/alloy/issues/1197))
- Rm Rich type ([#1195](https://github.com/alloy-rs/alloy/issues/1195))
- Clippy für docs ([#1194](https://github.com/alloy-rs/alloy/issues/1194))
- Remove RichBlock and RichHeader types ([#1185](https://github.com/alloy-rs/alloy/issues/1185))
- Add deposit receipt version ([#1188](https://github.com/alloy-rs/alloy/issues/1188))
- Remove async_trait from NetworkWallet ([#1160](https://github.com/alloy-rs/alloy/issues/1160))
- JSON-RPC 2.0 spelling ([#1146](https://github.com/alloy-rs/alloy/issues/1146))
- Add missing 7702 check ([#1137](https://github.com/alloy-rs/alloy/issues/1137))
- [eip7702] Devnet3 changes ([#1056](https://github.com/alloy-rs/alloy/issues/1056))
- [dep] Feature gate jwt in engine types ([#1131](https://github.com/alloy-rs/alloy/issues/1131))
- Release 0.2.1
- [rpc] Make `Deserialize` impl for `FilterChanges` generic over transaction ([#1118](https://github.com/alloy-rs/alloy/issues/1118))
- Correctly cfg unused type ([#1117](https://github.com/alloy-rs/alloy/issues/1117))
- Re-export and document network-primitives ([#1107](https://github.com/alloy-rs/alloy/issues/1107))
- Allow override all group ([#1104](https://github.com/alloy-rs/alloy/issues/1104))
- Chore : fix typos ([#1087](https://github.com/alloy-rs/alloy/issues/1087))
- Export rpc account type ([#1075](https://github.com/alloy-rs/alloy/issues/1075))
- Release 0.2.0
- Make auth mandatory in recovered auth ([#1047](https://github.com/alloy-rs/alloy/issues/1047))
- Trace output utils ([#1027](https://github.com/alloy-rs/alloy/issues/1027))
- Fix unnameable types ([#1029](https://github.com/alloy-rs/alloy/issues/1029))
- Add payloadbodies v2 to capabilities set ([#1025](https://github.com/alloy-rs/alloy/issues/1025))

### Other

- Implement conversion between signature types ([#1198](https://github.com/alloy-rs/alloy/issues/1198))
- Add emhane to codeowners ([#1189](https://github.com/alloy-rs/alloy/issues/1189))
- Add trait methods for constructing `alloy_rpc_types_eth::Transaction` to `alloy_consensus::Transaction` ([#1172](https://github.com/alloy-rs/alloy/issues/1172))
- Update TxType comment ([#1175](https://github.com/alloy-rs/alloy/issues/1175))
- Add payload length methods ([#1152](https://github.com/alloy-rs/alloy/issues/1152))
- Export types engine default features ([#1143](https://github.com/alloy-rs/alloy/issues/1143))
- Rm `PeerCount` ([#1140](https://github.com/alloy-rs/alloy/issues/1140))
- TxRequest into EIP-4844 without sidecar ([#1093](https://github.com/alloy-rs/alloy/issues/1093))
- Add conversion from BlockHashOrNumber to BlockId ([#1127](https://github.com/alloy-rs/alloy/issues/1127))
- Make `alloy_rpc_types_eth::SubscriptionResult` generic over tx ([#1123](https://github.com/alloy-rs/alloy/issues/1123))
- Add `AccessListResult` type (EIP-2930) ([#1110](https://github.com/alloy-rs/alloy/issues/1110))
- Derive arbitrary for `TransactionRequest` ([#1113](https://github.com/alloy-rs/alloy/issues/1113))
- Fix typo in genesis ([#1096](https://github.com/alloy-rs/alloy/issues/1096))
- Removing async get account ([#1080](https://github.com/alloy-rs/alloy/issues/1080))
- Added stages to the sync info rpc type ([#1079](https://github.com/alloy-rs/alloy/issues/1079))
- `alloy-consensus` should use `alloy_primitives::Sealable` ([#1072](https://github.com/alloy-rs/alloy/issues/1072))

### Refactor

- Add network-primitives ([#1101](https://github.com/alloy-rs/alloy/issues/1101))
- Replace `U64` with `u64`  ([#1057](https://github.com/alloy-rs/alloy/issues/1057))

### Styling

- Remove proptest in all crates and Arbitrary derives ([#966](https://github.com/alloy-rs/alloy/issues/966))

### Testing

- Flaky rpc ([#1180](https://github.com/alloy-rs/alloy/issues/1180))

## [0.1.4](https://github.com/alloy-rs/alloy/releases/tag/v0.1.4) - 2024-07-08

### Bug Fixes

- Fix watching already mined transactions ([#997](https://github.com/alloy-rs/alloy/issues/997))
- Ots_getContractCreater has field hash instead of tx ([#999](https://github.com/alloy-rs/alloy/issues/999))
- [signer-trezor] Fix zero gas price when sending legacy tx with trezor ([#977](https://github.com/alloy-rs/alloy/issues/977))

### Dependencies

- [deps] Remove reqwest and hyper from meta crate ([#974](https://github.com/alloy-rs/alloy/issues/974))

### Documentation

- Add release checklist ([#972](https://github.com/alloy-rs/alloy/issues/972))

### Features

- Add helper to set both input and data fields ([#1019](https://github.com/alloy-rs/alloy/issues/1019))
- [transport] Retry layer ([#849](https://github.com/alloy-rs/alloy/issues/849))
- Add execution payloadbodyv2 ([#1012](https://github.com/alloy-rs/alloy/issues/1012))
- Add consolidation requests to v4 payload ([#1013](https://github.com/alloy-rs/alloy/issues/1013))
- [rpc-types-eth] Add more utils to `TransactionIndex` ([#1007](https://github.com/alloy-rs/alloy/issues/1007))
- Impl Transaction for TxEnvelope ([#1006](https://github.com/alloy-rs/alloy/issues/1006))
- [eip1559] Support Optimism Canyon hardfork ([#1010](https://github.com/alloy-rs/alloy/issues/1010))
- Add missing admin_* methods ([#991](https://github.com/alloy-rs/alloy/issues/991))
- [network] Block context in ReceiptResponse ([#1003](https://github.com/alloy-rs/alloy/issues/1003))
- [otterscan] Add output for TraceEntry ([#1001](https://github.com/alloy-rs/alloy/issues/1001))
- Support web3_sha3 provider function ([#996](https://github.com/alloy-rs/alloy/issues/996))
- Add submit block request query ([#995](https://github.com/alloy-rs/alloy/issues/995))
- Add trace_get ([#987](https://github.com/alloy-rs/alloy/issues/987))
- Add net rpc namespace ([#989](https://github.com/alloy-rs/alloy/issues/989))
- Add missing debug_* rpc methods ([#986](https://github.com/alloy-rs/alloy/issues/986))
- Add into transactions iterator ([#984](https://github.com/alloy-rs/alloy/issues/984))
- Add helpers for trace action ([#982](https://github.com/alloy-rs/alloy/issues/982))
- Impl `From<RpcBlockHash>` for `BlockHashOrNumber` ([#980](https://github.com/alloy-rs/alloy/issues/980))
- Add missing eth bundle args ([#978](https://github.com/alloy-rs/alloy/issues/978))

### Miscellaneous Tasks

- Release 0.1.4
- Update release config
- Add helper functions for destructuring auth types ([#1022](https://github.com/alloy-rs/alloy/issues/1022))
- Convert rcp-types-eth block Header to consensus Header ([#1014](https://github.com/alloy-rs/alloy/issues/1014))
- [docs] Add the missing crate `rpc-types-mev` ([#1011](https://github.com/alloy-rs/alloy/issues/1011))
- Clean up 7702 encoding ([#1000](https://github.com/alloy-rs/alloy/issues/1000))
- Make wrapped index value pub ([#988](https://github.com/alloy-rs/alloy/issues/988))
- [provider] Simplify nonce filler ([#976](https://github.com/alloy-rs/alloy/issues/976))
- Release 0.1.3 (-p alloy)

### Other

- Remove signature.v parity before calculating tx hash ([#893](https://github.com/alloy-rs/alloy/issues/893))
- Fix wasi job ([#993](https://github.com/alloy-rs/alloy/issues/993))
- Update builders to vector of strings in privacy struct ([#983](https://github.com/alloy-rs/alloy/issues/983))
- Allow to convert CallBuilderTo TransactionRequest ([#981](https://github.com/alloy-rs/alloy/issues/981))
- [hotfix] Typo change pub(crate) to pub ([#979](https://github.com/alloy-rs/alloy/issues/979))
- Add range test in `FilterBlockOption` ([#939](https://github.com/alloy-rs/alloy/issues/939))

### Testing

- Add missing unit test for op `calc_next_block_base_fee` ([#1008](https://github.com/alloy-rs/alloy/issues/1008))
- Fix flaky anvil test ([#992](https://github.com/alloy-rs/alloy/issues/992))

## [0.1.3](https://github.com/alloy-rs/alloy/releases/tag/v0.1.3) - 2024-06-25

### Bug Fixes

- Continue reading ipc on large data ([#958](https://github.com/alloy-rs/alloy/issues/958))
- Deserialization of null storage keys in AccessListItem ([#955](https://github.com/alloy-rs/alloy/issues/955))
- Enable tls12 in rustls ([#952](https://github.com/alloy-rs/alloy/issues/952))

### Dependencies

- [eips] Make `alloy-serde` optional under `serde` ([#948](https://github.com/alloy-rs/alloy/issues/948))

### Documentation

- Copy/paste error of eip-7251 link ([#961](https://github.com/alloy-rs/alloy/issues/961))

### Features

- [network] Add `input` method to `TransactionResponse` ([#959](https://github.com/alloy-rs/alloy/issues/959))
- Move mev.rs from reth to rpc-types-mev ([#970](https://github.com/alloy-rs/alloy/issues/970))
- [alloy] Forward `rustls` & `native` reqwest TLS configuration to Alloy's metacrate ([#969](https://github.com/alloy-rs/alloy/issues/969))
- Add eip-7702 helpers ([#950](https://github.com/alloy-rs/alloy/issues/950))
- [contract] Implement Filter's builder methods on Event ([#960](https://github.com/alloy-rs/alloy/issues/960))
- Add eip-7251 system contract address/code ([#956](https://github.com/alloy-rs/alloy/issues/956))
- Add trace_filter method ([#946](https://github.com/alloy-rs/alloy/issues/946))

### Miscellaneous Tasks

- Release 0.1.3
- Release 0.1.3
- [eips] Add serde to Authorization types ([#964](https://github.com/alloy-rs/alloy/issues/964))
- Add more features to meta crate ([#953](https://github.com/alloy-rs/alloy/issues/953))
- [eips] Make `sha2` optional, add `kzg-sidecar` feature ([#949](https://github.com/alloy-rs/alloy/issues/949))
- Nightly clippy ([#947](https://github.com/alloy-rs/alloy/issues/947))

### Other

- [contract] Support state overrides for gas estimation ([#967](https://github.com/alloy-rs/alloy/issues/967))

## [0.1.2](https://github.com/alloy-rs/alloy/releases/tag/v0.1.2) - 2024-06-19

### Dependencies

- Relax version in workspace dependencies ([#940](https://github.com/alloy-rs/alloy/issues/940))

### Documentation

- Update alloy-eips supported eip list ([#942](https://github.com/alloy-rs/alloy/issues/942))
- Update get_balance docs ([#938](https://github.com/alloy-rs/alloy/issues/938))
- Touch up docs, TODOs ([#918](https://github.com/alloy-rs/alloy/issues/918))
- Add per-crate changelogs ([#914](https://github.com/alloy-rs/alloy/issues/914))

### Features

- Add TryFrom for GethTrace for all inner variants ([#933](https://github.com/alloy-rs/alloy/issues/933))
- [genesis] Update `extra_fields` to use `OtherFields` ([#936](https://github.com/alloy-rs/alloy/issues/936))
- [rpc-types-anvil] Add `Index`, fix compatibility ([#931](https://github.com/alloy-rs/alloy/issues/931))
- Add trace_raw_transaction and trace_replay_block_transactions ([#925](https://github.com/alloy-rs/alloy/issues/925))
- Add `is_` and `as_` utils for `FilterBlockOption` ([#927](https://github.com/alloy-rs/alloy/issues/927))
- [provider] Support ethCall optional blockId serialization ([#900](https://github.com/alloy-rs/alloy/issues/900))
- Add utils to `ValueOrArray` ([#924](https://github.com/alloy-rs/alloy/issues/924))
- Add `is_` utils to `FilterChanges` ([#923](https://github.com/alloy-rs/alloy/issues/923))
- Add eip-7251 consolidation request ([#919](https://github.com/alloy-rs/alloy/issues/919))
- Add `BlockId::as_u64` ([#916](https://github.com/alloy-rs/alloy/issues/916))

### Miscellaneous Tasks

- Release 0.1.2
- [rpc-types] Remove duplicate `Index` definition in `rpc-types-anvil` in favor of the one in `rpc-types-eth` ([#943](https://github.com/alloy-rs/alloy/issues/943))
- Update eip-2935 bytecode and address ([#934](https://github.com/alloy-rs/alloy/issues/934))
- Don't self-host documentation anymore ([#920](https://github.com/alloy-rs/alloy/issues/920))
- Update changelogs for v0.1.1 ([#922](https://github.com/alloy-rs/alloy/issues/922))
- Use 'dep:' syntax in rpc-types ([#921](https://github.com/alloy-rs/alloy/issues/921))
- Add docs.rs metadata to all manifests ([#917](https://github.com/alloy-rs/alloy/issues/917))

## [0.1.1](https://github.com/alloy-rs/alloy/releases/tag/v0.1.1) - 2024-06-17

### Bug Fixes

- Remove bad serde default and replace with manual default for chainconfig ([#915](https://github.com/alloy-rs/alloy/issues/915))
- [contract] Set `to` when calling with ContractInstance ([#913](https://github.com/alloy-rs/alloy/issues/913))
- Downgrade tokio-tungstenite ([#881](https://github.com/alloy-rs/alloy/issues/881))
- Make test compile ([#873](https://github.com/alloy-rs/alloy/issues/873))
- [rpc-types] Additionally export on `eth` namespace as well as * ([#866](https://github.com/alloy-rs/alloy/issues/866))
- Support pre-658 status codes ([#848](https://github.com/alloy-rs/alloy/issues/848))
- Add "google-longrunning" ([#839](https://github.com/alloy-rs/alloy/issues/839))
- Non_exhaustive for 2718 error ([#837](https://github.com/alloy-rs/alloy/issues/837))
- Set minimal priority fee to 1 wei ([#808](https://github.com/alloy-rs/alloy/issues/808))
- Use envelopes in get_payload API ([#807](https://github.com/alloy-rs/alloy/issues/807))
- Return ExecutionPayloadV3 from get_payload_v3 ([#803](https://github.com/alloy-rs/alloy/issues/803))
- Add proptest derives back ([#797](https://github.com/alloy-rs/alloy/issues/797))
- Add request mod back ([#796](https://github.com/alloy-rs/alloy/issues/796))
- Overrides are B256 ([#783](https://github.com/alloy-rs/alloy/issues/783))
- Correctly serialize eth_call params ([#778](https://github.com/alloy-rs/alloy/issues/778))
- Include auth token in display ([#772](https://github.com/alloy-rs/alloy/issues/772))
- Parse deposit contract in chain config ([#750](https://github.com/alloy-rs/alloy/issues/750))
- Serde rename camelcase ([#748](https://github.com/alloy-rs/alloy/issues/748))
- Make eip-7685 req untagged ([#743](https://github.com/alloy-rs/alloy/issues/743))
- Debug_trace arguments ([#730](https://github.com/alloy-rs/alloy/issues/730))
- `FeeHistory` deserialization ([#722](https://github.com/alloy-rs/alloy/issues/722))
- Required fields for transactions and receipts ([#719](https://github.com/alloy-rs/alloy/issues/719))
- Account for requests root in header mem size ([#706](https://github.com/alloy-rs/alloy/issues/706))
- Include `alloy-contract?/pubsub` in `pubsub` feature ([#703](https://github.com/alloy-rs/alloy/issues/703))
- Implement `sign_dynamic_typed_data` for ledger signers ([#701](https://github.com/alloy-rs/alloy/issues/701))
- Use U64 for feeHistory blocknumber ([#694](https://github.com/alloy-rs/alloy/issues/694))
- Add check before allocation in `SimpleCoder::decode_one()` ([#689](https://github.com/alloy-rs/alloy/issues/689))
- [provider] Map to primitive u128 ([#678](https://github.com/alloy-rs/alloy/issues/678))
- More abstraction for block transactions ([#666](https://github.com/alloy-rs/alloy/issues/666))
- [`README.md`] Add `alloy-signer-wallet` to crate list in readme ([#663](https://github.com/alloy-rs/alloy/issues/663))
- Expose kzg feat via alloy namespace ([#660](https://github.com/alloy-rs/alloy/issues/660))
- Populate hashes after setting sidecar ([#648](https://github.com/alloy-rs/alloy/issues/648))
- Checking if the eip1559 gas fields are not set on eip2930 check ([#635](https://github.com/alloy-rs/alloy/issues/635))
- Signer filler now propagates missing keys from builder ([#637](https://github.com/alloy-rs/alloy/issues/637))
- Better tx receipt mitigation ([#614](https://github.com/alloy-rs/alloy/issues/614))
- Admin_peerInfo, bump geth ([#620](https://github.com/alloy-rs/alloy/issues/620))
- Don't serialize nulls in tx request ([#621](https://github.com/alloy-rs/alloy/issues/621))
- Continue reading ipc on data error ([#605](https://github.com/alloy-rs/alloy/issues/605))
- Sol macro generated event filters were not filtering ([#600](https://github.com/alloy-rs/alloy/issues/600))
- [consensus] `TxEip4844Variant::into_signed` RLP ([#596](https://github.com/alloy-rs/alloy/issues/596))
- [provider] Uncle methods for block hash ([#587](https://github.com/alloy-rs/alloy/issues/587))
- [provider/debug] Arg type in debug_trace_call ([#585](https://github.com/alloy-rs/alloy/issues/585))
- Correct exitV1 type ([#567](https://github.com/alloy-rs/alloy/issues/567))
- Override txtype during submission prep ([#556](https://github.com/alloy-rs/alloy/issues/556))
- Signer fills from if unset ([#555](https://github.com/alloy-rs/alloy/issues/555))
- Add more generics to any and receipt with bloom ([#559](https://github.com/alloy-rs/alloy/issues/559))
- Tmp fix for PendingTransactionBuilder::get_receipt ([#558](https://github.com/alloy-rs/alloy/issues/558))
- Add back transaction type ([#552](https://github.com/alloy-rs/alloy/issues/552))
- Conflict between to change and debug tests ([#550](https://github.com/alloy-rs/alloy/issues/550))
- [rpc-types] Rm Option from `to` builder method of TxRequest. Consistent with others ([#505](https://github.com/alloy-rs/alloy/issues/505))
- Dont use fuse::select_next_some ([#532](https://github.com/alloy-rs/alloy/issues/532))
- Correctly parse IPC sockets in builtin connections ([#522](https://github.com/alloy-rs/alloy/issues/522))
- Tx receipt inclusion context ([#523](https://github.com/alloy-rs/alloy/issues/523))
- Eip1559 estimator ([#509](https://github.com/alloy-rs/alloy/issues/509))
- Workaround for `WithOtherFields` ([#495](https://github.com/alloy-rs/alloy/issues/495))
- Allow empty `to` field in `can_build` ([#489](https://github.com/alloy-rs/alloy/issues/489))
- Change `Header::nonce` to `B64` ([#485](https://github.com/alloy-rs/alloy/issues/485))
- Infinite loop while decoding a list of transactions ([#432](https://github.com/alloy-rs/alloy/issues/432))
- Automatically set blob versioned hashes if missing ([#409](https://github.com/alloy-rs/alloy/issues/409))
- Correctly treat `confirmation` for `watch_pending_transaction` ([#381](https://github.com/alloy-rs/alloy/issues/381))
- Small fixes for `Transaction` ([#388](https://github.com/alloy-rs/alloy/issues/388))
- Remove app-layer usage of transport error ([#363](https://github.com/alloy-rs/alloy/issues/363))
- Missing to in 4844 conversion ([#366](https://github.com/alloy-rs/alloy/issues/366))
- Correctly process chainId field ([#370](https://github.com/alloy-rs/alloy/issues/370))
- [provider] 0x prefix in sendRawTransaction ([#369](https://github.com/alloy-rs/alloy/issues/369))
- Mandatory `to` on `TxEip4844` ([#355](https://github.com/alloy-rs/alloy/issues/355))
- [rpc-engine-types] Use proper crate name in README.md ([#362](https://github.com/alloy-rs/alloy/issues/362))
- [transaction-request] Support HEX TransactionRequest.chain_id as per Ethereum JSON-RPC specification. ([#344](https://github.com/alloy-rs/alloy/issues/344))
- Change nonce from `U64` to `u64`  ([#341](https://github.com/alloy-rs/alloy/issues/341))
- Make `TransactionReceipt::transaction_hash` field mandatory ([#337](https://github.com/alloy-rs/alloy/issues/337))
- Force clippy to stable ([#331](https://github.com/alloy-rs/alloy/issues/331))
- Signer implementations for object-safe smart pointers ([#334](https://github.com/alloy-rs/alloy/issues/334))
- Fix subscribe blocks ([#330](https://github.com/alloy-rs/alloy/issues/330))
- Use enveloped encoding for typed transactions ([#239](https://github.com/alloy-rs/alloy/issues/239))
- Alloy core patches
- Alloy-sol-macro hash
- Early return for `JsonStorageKey` to `String` ([#261](https://github.com/alloy-rs/alloy/issues/261))
- Enable reqwest default-tls feature in transport-http ([#248](https://github.com/alloy-rs/alloy/issues/248))
- Ensure camel case for untagged ([#240](https://github.com/alloy-rs/alloy/issues/240))
- Map deserde error to ErrorResp if it is an error payload ([#236](https://github.com/alloy-rs/alloy/issues/236))
- Add deposit_receipt_version field in OptimismTransactionReceiptFields ([#211](https://github.com/alloy-rs/alloy/issues/211))
- Make l1_fee_scalar f64 ([#209](https://github.com/alloy-rs/alloy/issues/209))
- [`rpc-types`] Do not deny additional fields ([#195](https://github.com/alloy-rs/alloy/issues/195))
- Handle IPC unreadable socket ([#167](https://github.com/alloy-rs/alloy/issues/167))
- Add encode_for_signing to Transaction, fix Ledger sign_transaction ([#161](https://github.com/alloy-rs/alloy/issues/161))
- Skip ipc eof error on deserialize ([#160](https://github.com/alloy-rs/alloy/issues/160))
- [pubsub] Handle subscription response on reconnects ([#105](https://github.com/alloy-rs/alloy/issues/105)) ([#107](https://github.com/alloy-rs/alloy/issues/107))
- [`consensus`] Ensure into_signed forces correct format for eip1559/2930 txs ([#150](https://github.com/alloy-rs/alloy/issues/150))
- [`eips`/`consensus`] Correctly decode txs on `TxEnvelope` ([#148](https://github.com/alloy-rs/alloy/issues/148))
- [consensus] Correct TxType flag in EIP-2718 encoding ([#138](https://github.com/alloy-rs/alloy/issues/138))
- [`consensus`] Populate chain id when decoding signed legacy txs ([#137](https://github.com/alloy-rs/alloy/issues/137))
- Use U256 for eth_getStorageAt ([#133](https://github.com/alloy-rs/alloy/issues/133))
- Use port 0 for anvil by default ([#135](https://github.com/alloy-rs/alloy/issues/135))
- Add ssz feature back to engine types ([#131](https://github.com/alloy-rs/alloy/issues/131))
- [providers] Receipts of unmined blocks should be null ([#104](https://github.com/alloy-rs/alloy/issues/104))
- [providers] Some methods have invalid formats for parameters ([#103](https://github.com/alloy-rs/alloy/issues/103))
- [`rpc-types`] Set Uncle as default for BlockTransactions ([#98](https://github.com/alloy-rs/alloy/issues/98))
- Deserialize EthNotification from params field ([#93](https://github.com/alloy-rs/alloy/issues/93))
- Correct signature type for transaction rpc object ([#51](https://github.com/alloy-rs/alloy/issues/51))
- Modify transport crate name in documents ([#53](https://github.com/alloy-rs/alloy/issues/53))
- Name lifetime in reference to self in TransportConnect ([#49](https://github.com/alloy-rs/alloy/issues/49))
- Remove the cow ([#34](https://github.com/alloy-rs/alloy/issues/34))
- Dep tokio
- 1 url type
- Url in deps
- Impl PubSubConnect for WsConnect in wasm
- Cargo hack
- Tokio rt on non-wasm
- Tests for provider
- Clippy all-features
- Turn ws off by default
- Clippy
- Manually impl deser of pubsubitem
- Reconnect in pubsubservice
- [`rpc-types`/`providers`] Use `U64` in block-number related types, make storage keys U256 ([#22](https://github.com/alloy-rs/alloy/issues/22))
- Use type params
- Don't make mod public
- Some imports
- A spawnable that isn't dumb
- Simplify deser_ok
- Remove unnecessary functions
- Wasm update for new result
- Remove commented bounds
- Hyper
- Add client feature to hyper
- Sync deny with alloy-core, add version to cargo.toml
- Qualify url
- Build without reqwest
- Rust 1.65, disable wasm, don't print secrets
- Lint
- Lifetimes for rpc calls
- Hide __ENFORCE_ZST
- Add debug bounds
- Remove extra to_json_raw_value

### Dependencies

- [deps] Bump all ([#864](https://github.com/alloy-rs/alloy/issues/864))
- [deps] Bump `alloy-core` to `0.7.6` (latest), fix broken test and violated deny ([#862](https://github.com/alloy-rs/alloy/issues/862))
- Bump `coins-bip32` and `coins-bip39` deps ([#856](https://github.com/alloy-rs/alloy/issues/856))
- [deps] Update to interprocess 2 ([#687](https://github.com/alloy-rs/alloy/issues/687))
- Bump version of alloy core ([#669](https://github.com/alloy-rs/alloy/issues/669))
- Bump jsonrpsee 0.22 ([#467](https://github.com/alloy-rs/alloy/issues/467))
- [deps] Bump alloy 0.7.0 ([#430](https://github.com/alloy-rs/alloy/issues/430))
- [deps] Update to hyper 1.0 ([#55](https://github.com/alloy-rs/alloy/issues/55))
- Bump core ([#372](https://github.com/alloy-rs/alloy/issues/372))
- Deduplicate AccessList and Withdrawals types ([#324](https://github.com/alloy-rs/alloy/issues/324))
- [deps] Update all dependencies ([#258](https://github.com/alloy-rs/alloy/issues/258))
- [deps] Bump trezor-client ([#206](https://github.com/alloy-rs/alloy/issues/206))
- [deps] Bumps ([#108](https://github.com/alloy-rs/alloy/issues/108))
- [deps] Unpatch core ([#102](https://github.com/alloy-rs/alloy/issues/102))
- Alloy-consensus crate ([#83](https://github.com/alloy-rs/alloy/issues/83))
- Deploy documentation to GitHub Pages ([#56](https://github.com/alloy-rs/alloy/issues/56))
- [deps] Bump core ([#54](https://github.com/alloy-rs/alloy/issues/54))
- Bump alloy version
- Bump Cargo.toml

### Documentation

- Correct a comment
- Update MSRV policy ([#912](https://github.com/alloy-rs/alloy/issues/912))
- Move rpc client from transport readme ([#782](https://github.com/alloy-rs/alloy/issues/782))
- Add section contributions related to spelling ([#764](https://github.com/alloy-rs/alloy/issues/764))
- Unhide `sol!` wrapper in meta crate ([#654](https://github.com/alloy-rs/alloy/issues/654))
- Fix docs link in README.md ([#629](https://github.com/alloy-rs/alloy/issues/629))
- Add required softwares to run tests in Contributing.md ([#627](https://github.com/alloy-rs/alloy/issues/627))
- Fix 404s on docs site via absolute paths ([#537](https://github.com/alloy-rs/alloy/issues/537))
- Redirect index.html to alloy meta crate ([#520](https://github.com/alloy-rs/alloy/issues/520))
- Update txtype docs ([#497](https://github.com/alloy-rs/alloy/issues/497))
- [provider] Add examples to `raw_request{,dyn}` ([#486](https://github.com/alloy-rs/alloy/issues/486))
- Add aliases to `get_transaction_count` ([#420](https://github.com/alloy-rs/alloy/issues/420))
- Update incorrect comment ([#329](https://github.com/alloy-rs/alloy/issues/329))
- Remaining missing docs ([#317](https://github.com/alloy-rs/alloy/issues/317))
- Do not accept grammar prs ([#310](https://github.com/alloy-rs/alloy/issues/310))
- More docs in `alloy-providers` ([#281](https://github.com/alloy-rs/alloy/issues/281))
- Update docs ([#189](https://github.com/alloy-rs/alloy/issues/189))
- Update signer documentation ([#180](https://github.com/alloy-rs/alloy/issues/180))
- Add some prestate docs ([#157](https://github.com/alloy-rs/alloy/issues/157))
- Update descriptions and top level summary ([#128](https://github.com/alloy-rs/alloy/issues/128))
- Fix some backticks
- Resolve broken links
- Comments for deser impl
- Add more docs to transport
- Make not suck
- Doc fix
- Note about not wanting this crate
- Nits
- Fix link
- A couple lines
- Hyper in http doc
- Resolve links
- Improve readme
- Add readmes
- More of em
- Docs and misc convenience
- Fix comment

### Features

- Integrate `EvmOverrides` to rpc types ([#906](https://github.com/alloy-rs/alloy/issues/906))
- Add trace_replay_transaction ([#908](https://github.com/alloy-rs/alloy/issues/908))
- Derive serde for header ([#902](https://github.com/alloy-rs/alloy/issues/902))
- Add getter methods for `FilterChanges` ([#899](https://github.com/alloy-rs/alloy/issues/899))
- Move `{,With}OtherFields` to serde crate ([#892](https://github.com/alloy-rs/alloy/issues/892))
- [alloy] Add `"full"` feature flag ([#877](https://github.com/alloy-rs/alloy/issues/877))
- [transport] HttpError ([#882](https://github.com/alloy-rs/alloy/issues/882))
- Add UnbuiltTransactionError type ([#878](https://github.com/alloy-rs/alloy/issues/878))
- Add as_ is_ functions to envelope ([#872](https://github.com/alloy-rs/alloy/issues/872))
- [provider] Expose `ProviderBuilder` via `fn builder()` ([#858](https://github.com/alloy-rs/alloy/issues/858))
- Derive `Default` for `WithdrawalRequest` and `DepositRequest` ([#867](https://github.com/alloy-rs/alloy/issues/867))
- Put wasm-bindgen-futures dep behind the `wasm-bindgen` feature flag ([#795](https://github.com/alloy-rs/alloy/issues/795))
- [rpc] Split off `eth` namespace in `alloy-rpc-types` to `alloy-rpc-types-eth` ([#847](https://github.com/alloy-rs/alloy/issues/847))
- [serde] Deprecate individual num::* for a generic `quantity` module ([#855](https://github.com/alloy-rs/alloy/issues/855))
- Add engine API v4 methods ([#853](https://github.com/alloy-rs/alloy/issues/853))
- Send_envelope ([#851](https://github.com/alloy-rs/alloy/issues/851))
- [rpc] Add remaining anvil rpc methods to provider ([#831](https://github.com/alloy-rs/alloy/issues/831))
- Add TransactionBuilder::apply ([#842](https://github.com/alloy-rs/alloy/issues/842))
- [rpc] Use `BlockTransactionsKind` enum instead of bool for full arguments ([#840](https://github.com/alloy-rs/alloy/issues/840))
- [network] Constrain `TransactionResponse` ([#835](https://github.com/alloy-rs/alloy/issues/835))
- Full block ambiguity ([#832](https://github.com/alloy-rs/alloy/issues/832))
- Feat(contract) : add reference to TransactionRequest type ([#828](https://github.com/alloy-rs/alloy/issues/828))
- [rpc] Add more helpers for `TraceResult` ([#815](https://github.com/alloy-rs/alloy/issues/815))
- [rpc] Implement `Default` for `TransactionTrace` ([#816](https://github.com/alloy-rs/alloy/issues/816))
- Method on `Provider` to make a new `N::TransactionRequest` ([#812](https://github.com/alloy-rs/alloy/issues/812))
- Feat(consensus) Add test for account  ([#801](https://github.com/alloy-rs/alloy/issues/801))
- Add overrides to eth_estimateGas ([#802](https://github.com/alloy-rs/alloy/issues/802))
- [rpc-types] Add topic0 (alias `event_signature`) getter to `Log` ([#799](https://github.com/alloy-rs/alloy/issues/799))
- Feat(consensus) implement RLP for Account information ([#789](https://github.com/alloy-rs/alloy/issues/789))
- Fromiterator for filterset ([#790](https://github.com/alloy-rs/alloy/issues/790))
- HttpConnect ([#786](https://github.com/alloy-rs/alloy/issues/786))
- [`provider`] `eth_getAccount` support ([#760](https://github.com/alloy-rs/alloy/issues/760))
- Set poll interval based on connected chain ([#767](https://github.com/alloy-rs/alloy/issues/767))
- Relay rpc types ([#758](https://github.com/alloy-rs/alloy/issues/758))
- Add methods to JwtSecret to read and write from filesystem ([#755](https://github.com/alloy-rs/alloy/issues/755))
- Block id convenience functions ([#757](https://github.com/alloy-rs/alloy/issues/757))
- Add Parlia genesis config for BSC ([#740](https://github.com/alloy-rs/alloy/issues/740))
- [eips] EIP-2935 history storage contract ([#747](https://github.com/alloy-rs/alloy/issues/747))
- Add depositContractAddress to genesis ([#744](https://github.com/alloy-rs/alloy/issues/744))
- Add op payload type ([#742](https://github.com/alloy-rs/alloy/issues/742))
- Add payload envelope v4 ([#741](https://github.com/alloy-rs/alloy/issues/741))
- [genesis] Add prague to chain config ([#733](https://github.com/alloy-rs/alloy/issues/733))
- Derive proptest arbitrary for `Request` ([#732](https://github.com/alloy-rs/alloy/issues/732))
- Serde for `Request` ([#731](https://github.com/alloy-rs/alloy/issues/731))
- Derive arbitrary for `Request` ([#729](https://github.com/alloy-rs/alloy/issues/729))
- Duplicate funtions of  in crates/contract/src/call.rs ([#534](https://github.com/alloy-rs/alloy/issues/534)) ([#726](https://github.com/alloy-rs/alloy/issues/726))
- Rlp enc/dec for requests ([#728](https://github.com/alloy-rs/alloy/issues/728))
- [consensus, eips] EIP-7002 system contract ([#727](https://github.com/alloy-rs/alloy/issues/727))
- Beacon sidecar iterator ([#718](https://github.com/alloy-rs/alloy/issues/718))
- Re-export and more http aliases ([#716](https://github.com/alloy-rs/alloy/issues/716))
- Re-export rpc-types-beacon in crates/alloy ([#713](https://github.com/alloy-rs/alloy/issues/713))
- Add eth mainnet EL requests envelope ([#707](https://github.com/alloy-rs/alloy/issues/707))
- Add eip-7685 enc/decode traits ([#704](https://github.com/alloy-rs/alloy/issues/704))
- Beacon sidecar types ([#709](https://github.com/alloy-rs/alloy/issues/709))
- Rlp for eip-7002 requests ([#705](https://github.com/alloy-rs/alloy/issues/705))
- Add `EngineApi` extension trait ([#676](https://github.com/alloy-rs/alloy/issues/676))
- Move beacon API types from paradigmxyz/reth ([#684](https://github.com/alloy-rs/alloy/issues/684))
- Manual blob deserialize ([#696](https://github.com/alloy-rs/alloy/issues/696))
- Impl `From` for exec payload v4 ([#695](https://github.com/alloy-rs/alloy/issues/695))
- Add MaybeCancunPayloadFields::as_ref ([#692](https://github.com/alloy-rs/alloy/issues/692))
- Tracing for http transports ([#681](https://github.com/alloy-rs/alloy/issues/681))
- Add eip-7685 requests root to header ([#668](https://github.com/alloy-rs/alloy/issues/668))
- Derive arbitrary for BlobTransactionSidecar ([#679](https://github.com/alloy-rs/alloy/issues/679))
- Use alloy types for BlobTransactionSidecar ([#673](https://github.com/alloy-rs/alloy/issues/673))
- Add PayloadError variants ([#649](https://github.com/alloy-rs/alloy/issues/649))
- Eth_call builder  ([#645](https://github.com/alloy-rs/alloy/issues/645))
- Support changing CallBuilder decoders ([#641](https://github.com/alloy-rs/alloy/issues/641))
- Add extra_fields to ChainConfig ([#631](https://github.com/alloy-rs/alloy/issues/631))
- AnvilProvider ([#611](https://github.com/alloy-rs/alloy/issues/611))
- [engine] Add JSON Web Token (JWT) token generation and validation support ([#612](https://github.com/alloy-rs/alloy/issues/612))
- [pubsub] Set channel size ([#602](https://github.com/alloy-rs/alloy/issues/602))
- Passthrough methods on txenvelope ([#598](https://github.com/alloy-rs/alloy/issues/598))
- Add builder methods ([#591](https://github.com/alloy-rs/alloy/issues/591))
- Allow to only fill a transaction request ([#590](https://github.com/alloy-rs/alloy/issues/590))
- Add set_sidecar to the callbuilder ([#594](https://github.com/alloy-rs/alloy/issues/594))
- Add Display for block hash or number ([#592](https://github.com/alloy-rs/alloy/issues/592))
- Add generics to filter, transaction, and pub_sub. ([#573](https://github.com/alloy-rs/alloy/issues/573))
- Bubble up set_subscription_status ([#581](https://github.com/alloy-rs/alloy/issues/581))
- WalletProvider ([#569](https://github.com/alloy-rs/alloy/issues/569))
- Add the txhash getter. ([#574](https://github.com/alloy-rs/alloy/issues/574))
- Add ClientVersionV1 ([#562](https://github.com/alloy-rs/alloy/issues/562))
- Add prague engine types ([#557](https://github.com/alloy-rs/alloy/issues/557))
- Refactor request builder workflow ([#431](https://github.com/alloy-rs/alloy/issues/431))
- Export inner encoding / decoding functions from `Tx*` types ([#529](https://github.com/alloy-rs/alloy/issues/529))
- [provider] `debug_*` methods ([#548](https://github.com/alloy-rs/alloy/issues/548))
- [provider] Geth `txpool_*` methods ([#546](https://github.com/alloy-rs/alloy/issues/546))
- Add rpc-types-anvil ([#526](https://github.com/alloy-rs/alloy/issues/526))
- Add BaseFeeParams::new ([#525](https://github.com/alloy-rs/alloy/issues/525))
- [provider] Get_uncle_count ([#524](https://github.com/alloy-rs/alloy/issues/524))
- Port helpers for accesslist ([#508](https://github.com/alloy-rs/alloy/issues/508))
- Add missing blob versioned hashes error variant ([#506](https://github.com/alloy-rs/alloy/issues/506))
- [rpc] Trace requests and responses ([#498](https://github.com/alloy-rs/alloy/issues/498))
- Joinable transaction fillers ([#426](https://github.com/alloy-rs/alloy/issues/426))
- Helpers for AnyNetwork ([#476](https://github.com/alloy-rs/alloy/issues/476))
- Add Http::new for reqwest::Client ([#434](https://github.com/alloy-rs/alloy/issues/434))
- `std` feature flag for `alloy-consensus` ([#461](https://github.com/alloy-rs/alloy/issues/461))
- Add map_inner ([#460](https://github.com/alloy-rs/alloy/issues/460))
- Receipt qol functions ([#459](https://github.com/alloy-rs/alloy/issues/459))
- Use AnyReceiptEnvelope for AnyNetwork ([#457](https://github.com/alloy-rs/alloy/issues/457))
- Add AnyReceiptEnvelope ([#446](https://github.com/alloy-rs/alloy/issues/446))
- Rename alloy-rpc-*-types to alloy-rpc-types-* ([#435](https://github.com/alloy-rs/alloy/issues/435))
- Improve and complete `alloy` prelude crate feature flag compatiblity ([#421](https://github.com/alloy-rs/alloy/issues/421))
- [rpc] Add `blockTimestamp` to Log ([#429](https://github.com/alloy-rs/alloy/issues/429))
- Default to Ethereum network in `alloy-provider` and `alloy-contract` ([#356](https://github.com/alloy-rs/alloy/issues/356))
- Embed primitives Log in rpc Log and consensus Receipt in rpc Receipt ([#396](https://github.com/alloy-rs/alloy/issues/396))
- Add initial EIP-7547 engine types ([#287](https://github.com/alloy-rs/alloy/issues/287))
- Make HTTP provider optional ([#379](https://github.com/alloy-rs/alloy/issues/379))
- Add `AnyNetwork` ([#383](https://github.com/alloy-rs/alloy/issues/383))
- Implement `admin_trait`  ([#405](https://github.com/alloy-rs/alloy/issues/405))
- Handle 4844 fee ([#412](https://github.com/alloy-rs/alloy/issues/412))
- Add some BlockId helpers ([#413](https://github.com/alloy-rs/alloy/issues/413))
- Extend TransactionBuilder with BlobTransactionSideCar setters ([#411](https://github.com/alloy-rs/alloy/issues/411))
- Serde for consensus tx types ([#361](https://github.com/alloy-rs/alloy/issues/361))
- [providers] Connect_boxed api ([#342](https://github.com/alloy-rs/alloy/issues/342))
- Convenience functions for nonce and gas on `ProviderBuilder` ([#378](https://github.com/alloy-rs/alloy/issues/378))
- Add eth_blobBaseFee and eth_maxPriorityFeePerGas ([#380](https://github.com/alloy-rs/alloy/issues/380))
- Re-export EnvKzgSettings ([#375](https://github.com/alloy-rs/alloy/issues/375))
- Versioned hashes without kzg ([#360](https://github.com/alloy-rs/alloy/issues/360))
- `Provider::subscribe_logs` ([#339](https://github.com/alloy-rs/alloy/issues/339))
- `impl TryFrom<Transaction> for TxEnvelope` ([#343](https://github.com/alloy-rs/alloy/issues/343))
- [layers] GasEstimationLayer ([#326](https://github.com/alloy-rs/alloy/issues/326))
- [node-bindings] Add methods for returning instance urls ([#359](https://github.com/alloy-rs/alloy/issues/359))
- Support no_std for alloy-genesis/alloy-serde ([#320](https://github.com/alloy-rs/alloy/issues/320))
- `impl From<Transaction> for TransactionRequest` + small type updates ([#338](https://github.com/alloy-rs/alloy/issues/338))
- [json-rpc] Use `Cow` instead of `&'static str` for method names ([#319](https://github.com/alloy-rs/alloy/issues/319))
- 4844 SidecarBuilder ([#250](https://github.com/alloy-rs/alloy/issues/250))
- Update priority fee estimator ([#316](https://github.com/alloy-rs/alloy/issues/316))
- Enable default features for `coins_bip39` to export default wordlist ([#309](https://github.com/alloy-rs/alloy/issues/309))
- Move local signers to a separate crate, fix wasm ([#306](https://github.com/alloy-rs/alloy/issues/306))
- Default to Ethereum network in `ProviderBuilder` ([#304](https://github.com/alloy-rs/alloy/issues/304))
- Support no_std for `alloy-eips` ([#181](https://github.com/alloy-rs/alloy/issues/181))
- Merge Provider traits into one ([#297](https://github.com/alloy-rs/alloy/issues/297))
- [providers] Event, polling and streaming methods ([#274](https://github.com/alloy-rs/alloy/issues/274))
- Derive `Hash` for `TypedTransaction` ([#284](https://github.com/alloy-rs/alloy/issues/284))
- Nonce filling layer ([#276](https://github.com/alloy-rs/alloy/issues/276))
- `trace_call` and `trace_callMany` ([#277](https://github.com/alloy-rs/alloy/issues/277))
- [`signer`] Sign dynamic typed data ([#235](https://github.com/alloy-rs/alloy/issues/235))
- Network abstraction and transaction builder ([#190](https://github.com/alloy-rs/alloy/issues/190))
- [rpc-trace-types] Add support for mux tracer ([#252](https://github.com/alloy-rs/alloy/issues/252))
- Add types for opcode tracing ([#249](https://github.com/alloy-rs/alloy/issues/249))
- Add Optimism execution payload envelope v3 ([#245](https://github.com/alloy-rs/alloy/issues/245))
- Add OptimismExecutionPayloadV3 ([#242](https://github.com/alloy-rs/alloy/issues/242))
- [`consensus`] Add extra EIP-4844 types needed ([#229](https://github.com/alloy-rs/alloy/issues/229))
- Add parent beacon block root into `ExecutionPayloadEnvelopeV3` ([#227](https://github.com/alloy-rs/alloy/issues/227))
- Add `alloy` prelude crate ([#203](https://github.com/alloy-rs/alloy/issues/203))
- Alloy-contract ([#182](https://github.com/alloy-rs/alloy/issues/182))
- Extend FeeHistory type with eip-4844 fields ([#188](https://github.com/alloy-rs/alloy/issues/188))
- [`alloy-consensus`] `EIP4844` tx support ([#185](https://github.com/alloy-rs/alloy/issues/185))
- [`alloy-providers`] Additional missing methods ([#184](https://github.com/alloy-rs/alloy/issues/184))
- Subscription type ([#175](https://github.com/alloy-rs/alloy/issues/175))
- [genesis] Support optional block number ([#174](https://github.com/alloy-rs/alloy/issues/174))
- [signer] Re-export k256, add `Wallet::from_bytes(B256)` ([#173](https://github.com/alloy-rs/alloy/issues/173))
- [`alloy-genesis`] Pk support ([#171](https://github.com/alloy-rs/alloy/issues/171))
- Alloy-dyn-contract ([#149](https://github.com/alloy-rs/alloy/issues/149))
- Add into_signer to Wallet ([#146](https://github.com/alloy-rs/alloy/issues/146))
- Add optimism module and refactor types ([#143](https://github.com/alloy-rs/alloy/issues/143))
- Helper function to check pending block filter ([#130](https://github.com/alloy-rs/alloy/issues/130))
- [signers] Adds alloy-signer-gcp ([#94](https://github.com/alloy-rs/alloy/issues/94))
- [rpc-types] Expose LogError ([#119](https://github.com/alloy-rs/alloy/issues/119))
- Move reth genesis to alloy-genesis ([#120](https://github.com/alloy-rs/alloy/issues/120))
- Add `alloy-node-bindings` ([#111](https://github.com/alloy-rs/alloy/issues/111))
- Split rpc types into trace types and rpc types ([#96](https://github.com/alloy-rs/alloy/issues/96))
- Use reth-rpc-types ([#89](https://github.com/alloy-rs/alloy/issues/89))
- Temporary provider trait ([#20](https://github.com/alloy-rs/alloy/issues/20))
- Improve CallInput ([#86](https://github.com/alloy-rs/alloy/issues/86))
- Improve block transactions iterator ([#85](https://github.com/alloy-rs/alloy/issues/85))
- Signers ([#44](https://github.com/alloy-rs/alloy/issues/44))
- Make mix hash optional ([#70](https://github.com/alloy-rs/alloy/issues/70))
- Interprocess-based IPC ([#59](https://github.com/alloy-rs/alloy/issues/59))
- New RPC types, and ergonomics ([#29](https://github.com/alloy-rs/alloy/issues/29))
- Ws
- New pubsub
- StateOverride rpc type ([#24](https://github.com/alloy-rs/alloy/issues/24))
- Add RPC types + Add temporary bare `Provider` ([#13](https://github.com/alloy-rs/alloy/issues/13))
- Connect_boxed
- Connect fn
- TransportConnect
- TransportConnect traits
- Misc QoL
- Spawn_ext
- SerializedRequest
- Docs note and try_as fns
- Eth-notification and expanded json-rpc
- Wasm-compatability
- Wasm-compatability
- Hyper_http in request builder
- Hyper support
- Seal transport
- BoxTransport
- Lifetime on rpccall
- Allow type-erased rpc client
- Generic request
- Client builder
- Manual future for json rpc to avoid higher-ranked lifetime
- RpcObject
- Separate rpc type crate
- Send batch request
- Blanket
- DummyNetwork compile check
- Some cool combinators on rpccall
- Unwrap variants
- Transports crate

### Miscellaneous Tasks

- Release 0.1.1
- Add rpc types beacon pkg description
- [clippy] Apply lint suggestions ([#903](https://github.com/alloy-rs/alloy/issues/903))
- [alloy] Add link to book and alloy ([#891](https://github.com/alloy-rs/alloy/issues/891))
- [general] Add release configuration ([#888](https://github.com/alloy-rs/alloy/issues/888))
- Update EIP7002 withdrawal requests based on spec ([#885](https://github.com/alloy-rs/alloy/issues/885))
- [general] Update issue templates ([#880](https://github.com/alloy-rs/alloy/issues/880))
- Rm unused txtype mod ([#879](https://github.com/alloy-rs/alloy/issues/879))
- [other] Use type aliases where possible to improve clarity  ([#859](https://github.com/alloy-rs/alloy/issues/859))
- [eips] Compile tests with default features ([#860](https://github.com/alloy-rs/alloy/issues/860))
- [provider] Reorder methods in `Provider` trait ([#863](https://github.com/alloy-rs/alloy/issues/863))
- [provider] Document privileged status of EIP-1559 ([#850](https://github.com/alloy-rs/alloy/issues/850))
- [docs] Crate completeness and fix typos ([#861](https://github.com/alloy-rs/alloy/issues/861))
- [docs] Add doc aliases ([#843](https://github.com/alloy-rs/alloy/issues/843))
- Add Into for WithOtherFields in rpc types ([#813](https://github.com/alloy-rs/alloy/issues/813))
- Add engine_getClientVersionV1 ([#823](https://github.com/alloy-rs/alloy/issues/823))
- Add engine api v4 capabilities ([#822](https://github.com/alloy-rs/alloy/issues/822))
- Move trace to extension trait ([#818](https://github.com/alloy-rs/alloy/issues/818))
- Fix remaining warnings, add TODO for proptest-derive ([#819](https://github.com/alloy-rs/alloy/issues/819))
- Expose Claims is_within_time_window as pub ([#794](https://github.com/alloy-rs/alloy/issues/794))
- Fix warnings, check-cfg ([#776](https://github.com/alloy-rs/alloy/issues/776))
- [consensus] Re-export EIP-4844 transactions ([#777](https://github.com/alloy-rs/alloy/issues/777))
- Remove rlp encoding for `Request` ([#751](https://github.com/alloy-rs/alloy/issues/751))
- Get_transaction_by_hash returns Option<Transaction> ([#714](https://github.com/alloy-rs/alloy/issues/714))
- Collapse Debug for OtherFields ([#702](https://github.com/alloy-rs/alloy/issues/702))
- Actually impl from for payload v4 ([#698](https://github.com/alloy-rs/alloy/issues/698))
- Rename deposit receipt to deposit request ([#693](https://github.com/alloy-rs/alloy/issues/693))
- Unused feature
- Add missing serde default attributes ([#685](https://github.com/alloy-rs/alloy/issues/685))
- Move blob validation to sidecar ([#677](https://github.com/alloy-rs/alloy/issues/677))
- Replace `ExitV1` with `WithdrawalRequest` ([#672](https://github.com/alloy-rs/alloy/issues/672))
- [general] Add CI workflow for Windows + fix IPC test ([#642](https://github.com/alloy-rs/alloy/issues/642))
- Fix typo ([#653](https://github.com/alloy-rs/alloy/issues/653))
- Remove outdated comment ([#640](https://github.com/alloy-rs/alloy/issues/640))
- Update to geth 1.14 ([#628](https://github.com/alloy-rs/alloy/issues/628))
- B'a' ([#609](https://github.com/alloy-rs/alloy/issues/609))
- Document how state overrides work in `call` and `call_raw` ([#570](https://github.com/alloy-rs/alloy/issues/570))
- Move BlockId type to alloy-eip ([#565](https://github.com/alloy-rs/alloy/issues/565))
- Remove Sealed in Transport definition ([#551](https://github.com/alloy-rs/alloy/issues/551))
- Rm PathBuf import ([#533](https://github.com/alloy-rs/alloy/issues/533))
- Reorder conversion error variants ([#507](https://github.com/alloy-rs/alloy/issues/507))
- Clippy, warnings ([#504](https://github.com/alloy-rs/alloy/issues/504))
- Add missing eq derives ([#496](https://github.com/alloy-rs/alloy/issues/496))
- Add helper for next block base fee ([#494](https://github.com/alloy-rs/alloy/issues/494))
- Some NodeInfo touchups ([#482](https://github.com/alloy-rs/alloy/issues/482))
- Update homepage and repository url ([#475](https://github.com/alloy-rs/alloy/issues/475))
- Simplify some RpcCall code ([#470](https://github.com/alloy-rs/alloy/issues/470))
- Improve hyper http error messages ([#469](https://github.com/alloy-rs/alloy/issues/469))
- Add OtsReceipt type ([#455](https://github.com/alloy-rs/alloy/issues/455))
- Export AnyReceiptEnvelope ([#453](https://github.com/alloy-rs/alloy/issues/453))
- Reexport receipt types ([#445](https://github.com/alloy-rs/alloy/issues/445))
- Remove redundant code from ethers ([#443](https://github.com/alloy-rs/alloy/issues/443))
- Re-add evalir to codeowners ([#427](https://github.com/alloy-rs/alloy/issues/427))
- Rearrange field order ([#417](https://github.com/alloy-rs/alloy/issues/417))
- Add Default to GasEstimatorLayer ([#410](https://github.com/alloy-rs/alloy/issues/410))
- Dedupe blob in consensus and rpc ([#401](https://github.com/alloy-rs/alloy/issues/401))
- Clean up kzg and features ([#386](https://github.com/alloy-rs/alloy/issues/386))
- Add helpers for next block ([#382](https://github.com/alloy-rs/alloy/issues/382))
- Error when missing to field in transaction conversion ([#365](https://github.com/alloy-rs/alloy/issues/365))
- Remove stale todos ([#354](https://github.com/alloy-rs/alloy/issues/354))
- Tweak tracing in ws transport ([#333](https://github.com/alloy-rs/alloy/issues/333))
- Rename `RpcClient::prepare` to `request` ([#299](https://github.com/alloy-rs/alloy/issues/299))
- [meta] Update CODEOWNERS ([#298](https://github.com/alloy-rs/alloy/issues/298))
- Debug/copy/clone derives ([#282](https://github.com/alloy-rs/alloy/issues/282))
- Const fns ([#280](https://github.com/alloy-rs/alloy/issues/280))
- Add contract to issue forms ([#265](https://github.com/alloy-rs/alloy/issues/265))
- Only accept required args ([#257](https://github.com/alloy-rs/alloy/issues/257))
- Clippy ([#251](https://github.com/alloy-rs/alloy/issues/251))
- Add missing doc link for parent_beacon_block_root ([#244](https://github.com/alloy-rs/alloy/issues/244))
- Rm unused file ([#234](https://github.com/alloy-rs/alloy/issues/234))
- [alloy] Re-export `alloy-core` items individually ([#230](https://github.com/alloy-rs/alloy/issues/230))
- Remove unused imports ([#224](https://github.com/alloy-rs/alloy/issues/224))
- Add from to test ([#223](https://github.com/alloy-rs/alloy/issues/223))
- Clean up Display impls ([#222](https://github.com/alloy-rs/alloy/issues/222))
- Use `impl Future` in `PubSubConnect` ([#218](https://github.com/alloy-rs/alloy/issues/218))
- [`rpc-types`] Add FromStr impl for BlockId ([#214](https://github.com/alloy-rs/alloy/issues/214))
- [`provider`] Make `BlockId` opt on get_storage_at ([#213](https://github.com/alloy-rs/alloy/issues/213))
- Clippy ([#208](https://github.com/alloy-rs/alloy/issues/208))
- Pin alloy-sol-macro ([#193](https://github.com/alloy-rs/alloy/issues/193))
- Simplify PubsubFrontend ([#168](https://github.com/alloy-rs/alloy/issues/168))
- More execution payload getters ([#166](https://github.com/alloy-rs/alloy/issues/166))
- Expose prev randao on `ExecutionPayload` ([#165](https://github.com/alloy-rs/alloy/issues/165))
- Add missing helpers to BlockTransactions ([#159](https://github.com/alloy-rs/alloy/issues/159))
- Clean up tracing macro uses ([#154](https://github.com/alloy-rs/alloy/issues/154))
- [`signers`] Fix errors from primitives upgrade, avoid passing `B256` by val ([#152](https://github.com/alloy-rs/alloy/issues/152))
- Add SECURITY.md ([#145](https://github.com/alloy-rs/alloy/issues/145))
- Reuse alloy genesis in bindings ([#139](https://github.com/alloy-rs/alloy/issues/139))
- Move blob tx sidecar ([#129](https://github.com/alloy-rs/alloy/issues/129))
- [github] Add consensus component to bug report form ([#127](https://github.com/alloy-rs/alloy/issues/127))
- Add back ssz feature ([#124](https://github.com/alloy-rs/alloy/issues/124))
- Remove allocator type ([#122](https://github.com/alloy-rs/alloy/issues/122))
- Correct doc typo ([#116](https://github.com/alloy-rs/alloy/issues/116))
- Add helper functions to ResponsePacket ([#115](https://github.com/alloy-rs/alloy/issues/115))
- Make CallRequest hash ([#114](https://github.com/alloy-rs/alloy/issues/114))
- Add support for other fields in call/txrequest ([#112](https://github.com/alloy-rs/alloy/issues/112))
- Cleanup rpc types ([#110](https://github.com/alloy-rs/alloy/issues/110))
- Make Log Default ([#101](https://github.com/alloy-rs/alloy/issues/101))
- Expose op receipt fields ([#95](https://github.com/alloy-rs/alloy/issues/95))
- [meta] Update ISSUE_TEMPLATE ([#72](https://github.com/alloy-rs/alloy/issues/72))
- Clippy ([#62](https://github.com/alloy-rs/alloy/issues/62))
- Misc improvements ([#26](https://github.com/alloy-rs/alloy/issues/26))
- More lints and warns and errors
- Add warns and denies to more lib files
- Add warns and denies to some lib files
- Fix wasm
- Remove dbg from test
- Remove dbg from test
- Add evalir to codeowners
- Add `rpc-types` to bug form
- Propagate generic error payload
- Improve id docs and ser
- Some batch request cleanup
- Fix cargo hack ci
- Update link in provider readme
- CI and more rustdoc
- Remove dead code
- Clippy
- Clippy cleanup
- Misc cleanup
- Cleanup in transports mod
- Clippy
- Delete unused src
- Workspace setup

### Other

- Add custom conversion error to handle additional situations (such as optimism deposit tx) ([#875](https://github.com/alloy-rs/alloy/issues/875))
- [Fix] use Eip2718Error, add docs on different encodings ([#869](https://github.com/alloy-rs/alloy/issues/869))
- Add receipt deserialize tests for `AnyTransactionReceipt` ([#868](https://github.com/alloy-rs/alloy/issues/868))
- Add `status` method to `ReceiptResponse` trait ([#846](https://github.com/alloy-rs/alloy/issues/846))
- Implement `Default` to `NodeForkConfig` ([#844](https://github.com/alloy-rs/alloy/issues/844))
- [feat] Synchronous filling ([#841](https://github.com/alloy-rs/alloy/issues/841))
- Pin to 0.24.6 ([#836](https://github.com/alloy-rs/alloy/issues/836))
- RecommendFiller -> RecommendedFiller, move to fillers ([#825](https://github.com/alloy-rs/alloy/issues/825))
- Implementation `Default` for `GethTrace` ([#817](https://github.com/alloy-rs/alloy/issues/817))
- Impl Eq, PartialEq for WithOtherFields<T: PartialEq | Eq> ([#806](https://github.com/alloy-rs/alloy/issues/806))
- Add Raw variant for Authorzation ([#804](https://github.com/alloy-rs/alloy/issues/804))
- Add iter on FilterSet ([#784](https://github.com/alloy-rs/alloy/issues/784))
- Add clippy at workspace level ([#766](https://github.com/alloy-rs/alloy/issues/766))
- Exporting waiter struct from batch ([#773](https://github.com/alloy-rs/alloy/issues/773))
- Specific Configs to GethDebugTracerConfig + generic config build method for GethDebugTracingOptions ([#686](https://github.com/alloy-rs/alloy/issues/686))
- Update clippy warnings ([#765](https://github.com/alloy-rs/alloy/issues/765))
- Arbitrary Sidecar implementation + build. Closes [#680](https://github.com/alloy-rs/alloy/issues/680). ([#708](https://github.com/alloy-rs/alloy/issues/708))
- Use Self instead of BlockNumberOrTag ([#754](https://github.com/alloy-rs/alloy/issues/754))
- Use into instead of from ([#749](https://github.com/alloy-rs/alloy/issues/749))
- Correctly sign non legacy transaction without EIP155 ([#647](https://github.com/alloy-rs/alloy/issues/647))
- RpcWithBlock ([#674](https://github.com/alloy-rs/alloy/issues/674))
- Some refactoring ([#739](https://github.com/alloy-rs/alloy/issues/739))
- Replace into_receipt by into ([#735](https://github.com/alloy-rs/alloy/issues/735))
- Replace into_tx by into ([#737](https://github.com/alloy-rs/alloy/issues/737))
- Small refactoring ([#724](https://github.com/alloy-rs/alloy/issues/724))
- Add `with_base_fee` for `TransactionInfo` ([#721](https://github.com/alloy-rs/alloy/issues/721))
- Implement From<Response> and From<EthNotification> for PubSubItem ([#710](https://github.com/alloy-rs/alloy/issues/710))
- Use Self when possible ([#711](https://github.com/alloy-rs/alloy/issues/711))
- Clarify installation instructions for alloy ([#697](https://github.com/alloy-rs/alloy/issues/697))
- Implement `TryFrom<Transaction>` for `TransactionInfo` ([#662](https://github.com/alloy-rs/alloy/issues/662))
- Implement `From<B256>` for `JsonStorageKey` ([#661](https://github.com/alloy-rs/alloy/issues/661))
- Implement From for FilterId ([#655](https://github.com/alloy-rs/alloy/issues/655))
- Small refactor ([#652](https://github.com/alloy-rs/alloy/issues/652))
- Use `From<Address>` for `TxKind` ([#651](https://github.com/alloy-rs/alloy/issues/651))
- Add AuthCall variant to CallType ([#650](https://github.com/alloy-rs/alloy/issues/650))
- Expose inner `B64` from `PayloadId` ([#646](https://github.com/alloy-rs/alloy/issues/646))
- [Refactor] Move Provider into its own module ([#644](https://github.com/alloy-rs/alloy/issues/644))
- Move block hash types to alloy-eips ([#639](https://github.com/alloy-rs/alloy/issues/639))
- [Refactor] Delete the internal-test-utils crate ([#632](https://github.com/alloy-rs/alloy/issues/632))
- [Call] Added more fields for call builder ([#625](https://github.com/alloy-rs/alloy/issues/625))
- Improve FilterChanges implementation ([#610](https://github.com/alloy-rs/alloy/issues/610))
- Derive Default for Parity ([#608](https://github.com/alloy-rs/alloy/issues/608))
- Configure polling interval ([#437](https://github.com/alloy-rs/alloy/issues/437))
- Expose SendableTx in providers ([#601](https://github.com/alloy-rs/alloy/issues/601))
- Add signature related ConversionError variants ([#586](https://github.com/alloy-rs/alloy/issues/586))
- Temp get_uncle fix ([#589](https://github.com/alloy-rs/alloy/issues/589))
- [Feature] Set subscription status on request and meta ([#576](https://github.com/alloy-rs/alloy/issues/576))
- Use the same way to both serialize and deserialize `OptimismPayloadAttributes::gas_limit`. ([#563](https://github.com/alloy-rs/alloy/issues/563))
- Add blob gas conversion error ([#545](https://github.com/alloy-rs/alloy/issues/545))
- Add new variants to `ConversionError` ([#541](https://github.com/alloy-rs/alloy/issues/541))
- Add link to docs to README ([#542](https://github.com/alloy-rs/alloy/issues/542))
- Update comments ([#521](https://github.com/alloy-rs/alloy/issues/521))
- Prestwich/signer multiplex ([#515](https://github.com/alloy-rs/alloy/issues/515))
- Revert "chore: remove outdated license ([#510](https://github.com/alloy-rs/alloy/issues/510))" ([#513](https://github.com/alloy-rs/alloy/issues/513))
- Add arbitrary derive for Withdrawal ([#501](https://github.com/alloy-rs/alloy/issues/501))
- Enable default-tls for alloy-provider/reqwest feature ([#483](https://github.com/alloy-rs/alloy/issues/483))
- Extension ([#474](https://github.com/alloy-rs/alloy/issues/474))
- TypeTransaction conversion trait impls ([#472](https://github.com/alloy-rs/alloy/issues/472))
- Update typo in README ([#480](https://github.com/alloy-rs/alloy/issues/480))
- Implement is_zero method for U64HexOrNumber ([#478](https://github.com/alloy-rs/alloy/issues/478))
- Derive default implementation for rpc Block ([#471](https://github.com/alloy-rs/alloy/issues/471))
- Mark envelopes non-exhaustive ([#456](https://github.com/alloy-rs/alloy/issues/456))
- TransactionList and BlockResponse ([#444](https://github.com/alloy-rs/alloy/issues/444))
- Removed reqwest prefix ([#462](https://github.com/alloy-rs/alloy/issues/462))
- Numeric type audit: network, consensus, provider, rpc-types ([#454](https://github.com/alloy-rs/alloy/issues/454))
- Derive arbitrary for rpc `Header` and `Transaction` ([#458](https://github.com/alloy-rs/alloy/issues/458))
- Enable ws and ipc flags to enable `on_ws` and `on_ipc` on ProviderBuilder ([#436](https://github.com/alloy-rs/alloy/issues/436))
- Adds `check -Zcheck-cfg ` job ([#419](https://github.com/alloy-rs/alloy/issues/419))
- Move Otterscan types to alloy ([#418](https://github.com/alloy-rs/alloy/issues/418))
- Added MAINNET_KZG_TRUSTED_SETUP ([#385](https://github.com/alloy-rs/alloy/issues/385))
- Check no_std in CI ([#367](https://github.com/alloy-rs/alloy/issues/367))
- TrezorHDPath -> HDPath ([#345](https://github.com/alloy-rs/alloy/issues/345))
- Bug form typo ([#351](https://github.com/alloy-rs/alloy/issues/351))
- Add `block_time_f64` to `Anvil` ([#336](https://github.com/alloy-rs/alloy/issues/336))
- Use latest stable
- `new` method to initialize IpcConnect ([#322](https://github.com/alloy-rs/alloy/issues/322))
- Rename `alloy-providers` to `alloy-provider` ([#278](https://github.com/alloy-rs/alloy/issues/278))
- Convert non-200 http responses into errors ([#254](https://github.com/alloy-rs/alloy/issues/254))
- Add `try_spawn` function for Anvil and Geth bindings ([#226](https://github.com/alloy-rs/alloy/issues/226))
- ClientRefs, Poller, and Streams ([#179](https://github.com/alloy-rs/alloy/issues/179))
- Add concurrency ([#238](https://github.com/alloy-rs/alloy/issues/238))
- Move total_difficulty to Header ([#220](https://github.com/alloy-rs/alloy/issues/220))
- Update state.rs ([#215](https://github.com/alloy-rs/alloy/issues/215))
- Various Subscription improvements ([#177](https://github.com/alloy-rs/alloy/issues/177))
- Use nextest as the test runner ([#134](https://github.com/alloy-rs/alloy/issues/134))
- Correct `is_create` condition ([#117](https://github.com/alloy-rs/alloy/issues/117))
- Impl TryFrom<alloy_rpc_types::Log> for alloy_primitives::Log ([#50](https://github.com/alloy-rs/alloy/issues/50))
- Removed missdocs in parity.rs ([#46](https://github.com/alloy-rs/alloy/issues/46))
- Revert "fix: correct signature type for transaction rpc object ([#51](https://github.com/alloy-rs/alloy/issues/51))" ([#88](https://github.com/alloy-rs/alloy/issues/88))
- Use to_raw_value from serde_json ([#64](https://github.com/alloy-rs/alloy/issues/64))
- Avoid unnecessary serialize for RequestPacket. ([#61](https://github.com/alloy-rs/alloy/issues/61))
- Remove Sync constraint for provider ([#52](https://github.com/alloy-rs/alloy/issues/52))
- Avoid allocation when convert Box<RawValue> into a hyper request ([#48](https://github.com/alloy-rs/alloy/issues/48))
- Merge pull request [#21](https://github.com/alloy-rs/alloy/issues/21) from alloy-rs/prestwich/new-pubsub
- Clippy
- Temporarily comment out tests
- Match tuple order
- Merge pull request [#23](https://github.com/alloy-rs/alloy/issues/23) from alloy-rs/evalir/add-to-codeowners
- Merge pull request [#16](https://github.com/alloy-rs/alloy/issues/16) from alloy-rs/onbjerg/rpc-types-bug
- Merge pull request [#11](https://github.com/alloy-rs/alloy/issues/11) from alloy-rs/prestwich/new-new-transport
- Reorder
- Transport
- Move attribute
- Naming
- Merge pull request [#9](https://github.com/alloy-rs/alloy/issues/9) from alloy-rs/prestwich/wasm-compat
- Merge pull request [#3](https://github.com/alloy-rs/alloy/issues/3) from alloy-rs/prestwich/readme-and-cleanup
- Merge pull request [#2](https://github.com/alloy-rs/alloy/issues/2) from alloy-rs/prestwich/transports
- Rename middleware to provider
- Some clippy and stuff
- Some middleware noodling
- Fuck jsonrpsee
- Mware and combinator stuff
- Address comments
- Initial commit

### Performance

- Remove getBlock request in feeHistory ([#414](https://github.com/alloy-rs/alloy/issues/414))
- Use raw response bytes ([#233](https://github.com/alloy-rs/alloy/issues/233))
- Don't collect or try_for_each in pubsub code ([#153](https://github.com/alloy-rs/alloy/issues/153))

### Refactor

- [rpc] Extract `admin` and `txpool` into their respective crate ([#898](https://github.com/alloy-rs/alloy/issues/898))
- [signers] Use `signer` for single credentials and `wallet` for credential stores  ([#883](https://github.com/alloy-rs/alloy/issues/883))
- Improve eth_call internals ([#763](https://github.com/alloy-rs/alloy/issues/763))
- Refactor around TxEip4844Variant ([#738](https://github.com/alloy-rs/alloy/issues/738))
- Change u64 to Duration ([#636](https://github.com/alloy-rs/alloy/issues/636))
- Clean up legacy serde helpers ([#624](https://github.com/alloy-rs/alloy/issues/624))
- Make optional BlockId params required in provider functions ([#516](https://github.com/alloy-rs/alloy/issues/516))
- Rename to reqd_confs ([#353](https://github.com/alloy-rs/alloy/issues/353))
- Remove `async_trait` in tx builder ([#279](https://github.com/alloy-rs/alloy/issues/279))
- Dedupe `CallRequest`/`TransactionRequest` ([#178](https://github.com/alloy-rs/alloy/issues/178))
- [`ipc`] Use single buffer and remove manual wakers ([#69](https://github.com/alloy-rs/alloy/issues/69))
- RpcError and RpcResult and TransportError and TransportResult ([#28](https://github.com/alloy-rs/alloy/issues/28))
- Break transports into several crates
- Rename env vars
- Disable batching for pubsub
- Delete pubsub trait
- Move box to its own module
- Better naming
- Update to use packets
- Deserialization of RpcResult
- Move transport to own modfile
- Packets
- Response module
- Relax a bound
- Rename to make obvious
- Seal transport
- Docs and cleanup
- Rename to boxed
- Cow for jsonrpc params
- More crate
- Move is_local to transport
- Transport requires type-erased futures. improved batch ergo
- Transport future aliases
- Minor legibility
- Remove Params type from RpcCall
- More stuff
- Small code quality
- RpcResult type
- RpcObject trait

### Styling

- Use poll loop for CallState ([#779](https://github.com/alloy-rs/alloy/issues/779))
- Format test files
- Make additional TxReceipt impls generic over T ([#617](https://github.com/alloy-rs/alloy/issues/617))
- [Blocked] Update TransactionRequest's `to` field to TxKind ([#553](https://github.com/alloy-rs/alloy/issues/553))
- [Feature] Receipt trait in alloy-consensus ([#477](https://github.com/alloy-rs/alloy/issues/477))
- Remove outdated license ([#510](https://github.com/alloy-rs/alloy/issues/510))
- Sort derives ([#499](https://github.com/alloy-rs/alloy/issues/499))
- Implement `arbitrary` for `TransactionReceipt` ([#449](https://github.com/alloy-rs/alloy/issues/449))
- Rename `ManagedNonceLayer` to `NonceManagerLayer` ([#415](https://github.com/alloy-rs/alloy/issues/415))
- [Feature] Move Mainnet KZG group and Lazy<KzgSettings> ([#368](https://github.com/alloy-rs/alloy/issues/368))
- Eip1559Estimation return type ([#352](https://github.com/alloy-rs/alloy/issues/352))
- Move `alloy-rpc-types` `serde_helpers` mod to standalone crate `alloy-serde` ([#259](https://github.com/alloy-rs/alloy/issues/259))
- Addition of engine rpc-types from reth ([#118](https://github.com/alloy-rs/alloy/issues/118))
- [`trace-rpc-types`] Rename crate to rpc-trace-types ([#97](https://github.com/alloy-rs/alloy/issues/97))
- Clean up fmt::Debug impls ([#75](https://github.com/alloy-rs/alloy/issues/75))
- [`rpc-types`] Sync `eth/trace` types with reth ([#47](https://github.com/alloy-rs/alloy/issues/47))
- Sync with core ([#27](https://github.com/alloy-rs/alloy/issues/27))

### Testing

- Add rand feature in providers ([#910](https://github.com/alloy-rs/alloy/issues/910))
- Add another fee history serde test ([#769](https://github.com/alloy-rs/alloy/issues/769))
- Add another serde test for fee history ([#746](https://github.com/alloy-rs/alloy/issues/746))
- Add bundle test ([#500](https://github.com/alloy-rs/alloy/issues/500))
- Add serde tests for eth_callMany ([#407](https://github.com/alloy-rs/alloy/issues/407))
- Add deserde test for errorpayload with missing data ([#237](https://github.com/alloy-rs/alloy/issues/237))
- Ignore instead of commenting a test ([#207](https://github.com/alloy-rs/alloy/issues/207))
- Http impls transport
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
