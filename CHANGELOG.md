# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0](https://github.com/alloy-rs/alloy/releases/tag/v0.8.0) - 2024-12-10

### Bug Fixes

- Use asref impl for receipt ([#1758](https://github.com/alloy-rs/alloy/issues/1758))
- Use `feeHistory` when estimating blob fee ([#1764](https://github.com/alloy-rs/alloy/issues/1764))

### Documentation

- Add `consensus-any` and `rpc-types-any` to the README ([#1759](https://github.com/alloy-rs/alloy/issues/1759))

### Features

- Add arbitrary for alloy types ([#1777](https://github.com/alloy-rs/alloy/issues/1777))
- [consensus] Require typed2718 for transaction ([#1746](https://github.com/alloy-rs/alloy/issues/1746))
- [engine] Forkchoice Version ([#1744](https://github.com/alloy-rs/alloy/issues/1744))
- Remove duplicated `to` method in `TransactionResponse` ([#1770](https://github.com/alloy-rs/alloy/issues/1770))
- Port reth pooled tx type ([#1767](https://github.com/alloy-rs/alloy/issues/1767))
- EIP-7691 ([#1762](https://github.com/alloy-rs/alloy/issues/1762))
- Relax RPC `Block` bounds ([#1757](https://github.com/alloy-rs/alloy/issues/1757))

### Miscellaneous Tasks

- Release 0.8.0
- Derive Copy for BlockWithParent ([#1776](https://github.com/alloy-rs/alloy/issues/1776))
- Introduce recovered and recoverable ([#1768](https://github.com/alloy-rs/alloy/issues/1768))
- Improve Display and Debug for BlockId ([#1765](https://github.com/alloy-rs/alloy/issues/1765))

### Other

- Reapply "feat(consensus): require typed2718 for transaction ([#1746](https://github.com/alloy-rs/alloy/issues/1746))" ([#1773](https://github.com/alloy-rs/alloy/issues/1773))
- Move deny into ci ([#1774](https://github.com/alloy-rs/alloy/issues/1774))
- Update deny.yml ([#1771](https://github.com/alloy-rs/alloy/issues/1771))
- Revert "feat(consensus): require typed2718 for transaction ([#1746](https://github.com/alloy-rs/alloy/issues/1746))" ([#1772](https://github.com/alloy-rs/alloy/issues/1772))

## [0.7.3](https://github.com/alloy-rs/alloy/releases/tag/v0.7.3) - 2024-12-05

### Bug Fixes

- Wrong func sig ([#1742](https://github.com/alloy-rs/alloy/issues/1742))
- Remove `Borrow` impl for RPC receipt ([#1721](https://github.com/alloy-rs/alloy/issues/1721))
- Adjust EIP-7742 to latest spec ([#1713](https://github.com/alloy-rs/alloy/issues/1713))
- Omit empty requests ([#1706](https://github.com/alloy-rs/alloy/issues/1706))
- Use B256::new instead of from ([#1701](https://github.com/alloy-rs/alloy/issues/1701))

### Dependencies

- [deps] Bump thiserror 2 ([#1700](https://github.com/alloy-rs/alloy/issues/1700))
- [general] Bump MSRV to 1.81, use `core::error::Error` on `no-std` compatible crates ([#1552](https://github.com/alloy-rs/alloy/issues/1552))

### Documentation

- Fix `SignableTransaction` docs to use `PrimitiveSignature` ([#1743](https://github.com/alloy-rs/alloy/issues/1743))
- Update docs for eip7685 `Requests` ([#1714](https://github.com/alloy-rs/alloy/issues/1714))

### Features

- Impl Encodable / Decodable for Receipts ([#1752](https://github.com/alloy-rs/alloy/issues/1752))
- Add TrieAccount conversion for genesis account ([#1755](https://github.com/alloy-rs/alloy/issues/1755))
- Add `BlockHeader::blob_fee` ([#1754](https://github.com/alloy-rs/alloy/issues/1754))
- Migrate to `TrieAccount` of alloy-trie ([#1750](https://github.com/alloy-rs/alloy/issues/1750))
- Move is_empty to trait function ([#1749](https://github.com/alloy-rs/alloy/issues/1749))
- Add missing new fn ([#1747](https://github.com/alloy-rs/alloy/issues/1747))
- Specialized geth tracer ([#1739](https://github.com/alloy-rs/alloy/issues/1739))
- Make Receipt rlp methods pub ([#1731](https://github.com/alloy-rs/alloy/issues/1731))
- Receipt root fn ([#1708](https://github.com/alloy-rs/alloy/issues/1708))
- Impl `Encodable2718` for `ReceiptWithBloom` ([#1719](https://github.com/alloy-rs/alloy/issues/1719))
- Feat(rpc-types-eth) add test for syncing ([#1724](https://github.com/alloy-rs/alloy/issues/1724))
- Add blob_gas_used ([#1704](https://github.com/alloy-rs/alloy/issues/1704))
- EIP-7685 requests helpers ([#1699](https://github.com/alloy-rs/alloy/issues/1699))

### Miscellaneous Tasks

- Release 0.7.3
- Export storage root fns ([#1756](https://github.com/alloy-rs/alloy/issues/1756))
- Re-export stateroot fns ([#1753](https://github.com/alloy-rs/alloy/issues/1753))
- Display instead of Debug the response JSON ([#1748](https://github.com/alloy-rs/alloy/issues/1748))
- Rm redundant generic ([#1737](https://github.com/alloy-rs/alloy/issues/1737))
- Relax ommers root fn ([#1736](https://github.com/alloy-rs/alloy/issues/1736))
- Add missing from impl ([#1732](https://github.com/alloy-rs/alloy/issues/1732))
- Update release.toml ([#1730](https://github.com/alloy-rs/alloy/issues/1730))
- Release 0.7.2 ([#1729](https://github.com/alloy-rs/alloy/issues/1729))
- Disable serde-with default features ([#1703](https://github.com/alloy-rs/alloy/issues/1703))
- Use encoded2718 ([#1702](https://github.com/alloy-rs/alloy/issues/1702))

### Other

- Specialized geth tracer for debug trace call ([#1741](https://github.com/alloy-rs/alloy/issues/1741))
- Add non strict JSON decoding for engine Payloadv2 type ([#1740](https://github.com/alloy-rs/alloy/issues/1740))
- Block_number_index added to callBundle reference type ([#1705](https://github.com/alloy-rs/alloy/issues/1705))
- Remove wrapper BlobsBundleV1Ssz ([#1726](https://github.com/alloy-rs/alloy/issues/1726))
- Change BlobsBundleV1Ssz unwrap implementation to safe code ([#1723](https://github.com/alloy-rs/alloy/issues/1723))

## [0.7.0](https://github.com/alloy-rs/alloy/releases/tag/v0.7.0) - 2024-11-28

### Bug Fixes

- EIP-7742 fixes ([#1697](https://github.com/alloy-rs/alloy/issues/1697))
- Pass slice to RlpReceipt::rlp_decode_fields ([#1696](https://github.com/alloy-rs/alloy/issues/1696))
- [provider] Use `BoxTransport` in `on_anvil_*` ([#1693](https://github.com/alloy-rs/alloy/issues/1693))
- [`signer`] Export PrimitiveSignature instead of deprecated sig ([#1671](https://github.com/alloy-rs/alloy/issues/1671))
- Wasm compatibility for RetryBackoff ([#1666](https://github.com/alloy-rs/alloy/issues/1666))
- [`consensus`] Serde aliases to avoid breaking changes ([#1654](https://github.com/alloy-rs/alloy/issues/1654))

### Dependencies

- Remove cron schedule for deps.yml ([#1674](https://github.com/alloy-rs/alloy/issues/1674))

### Features

- [eips] Make prague field an enum ([#1574](https://github.com/alloy-rs/alloy/issues/1574))
- EIP-7742 ([#1600](https://github.com/alloy-rs/alloy/issues/1600))
- Add contains for opcodegas container ([#1695](https://github.com/alloy-rs/alloy/issues/1695))
- Add helpers to initialize Tx request ([#1690](https://github.com/alloy-rs/alloy/issues/1690))
- Uninstall_filter in Provider trait ([#1685](https://github.com/alloy-rs/alloy/issues/1685))
- Get_block_transaction_count_by_number in Provider trait ([#1688](https://github.com/alloy-rs/alloy/issues/1688))
- Add parent_num_hash to BlockHeader ([#1687](https://github.com/alloy-rs/alloy/issues/1687))
- Get_block_transaction_count_by_hash in Provider trait ([#1686](https://github.com/alloy-rs/alloy/issues/1686))
- Get_filter_logs in Provider trait ([#1684](https://github.com/alloy-rs/alloy/issues/1684))
- Modify ReceiptWithBloom and associated impls to use with Reth ([#1672](https://github.com/alloy-rs/alloy/issues/1672))
- [consensus-tx] Enable fast `is_create` ([#1683](https://github.com/alloy-rs/alloy/issues/1683))
- Add `next_block_base_fee` to `BlockHeader` trait ([#1682](https://github.com/alloy-rs/alloy/issues/1682))
- Add missing size fn ([#1679](https://github.com/alloy-rs/alloy/issues/1679))
- Introduce Typed2718 trait ([#1675](https://github.com/alloy-rs/alloy/issues/1675))
- Feat(eip5792) add test for wallet_sendCalls request type ([#1670](https://github.com/alloy-rs/alloy/issues/1670))
- Feat(rpc-type-baecon) add default for header type ([#1669](https://github.com/alloy-rs/alloy/issues/1669))
- Add more missing eth_callBundle arguments ([#1667](https://github.com/alloy-rs/alloy/issues/1667))
- Move `AnyReceipt` and `AnyHeader` to `alloy-consensus-any` ([#1609](https://github.com/alloy-rs/alloy/issues/1609))
- Add missing txtype tryfroms ([#1651](https://github.com/alloy-rs/alloy/issues/1651))
- [debug] Add debug_executionWitness to debug api ([#1649](https://github.com/alloy-rs/alloy/issues/1649))
- Add rlp for txtype ([#1648](https://github.com/alloy-rs/alloy/issues/1648))

### Miscellaneous Tasks

- Release 0.7.0
- Add changelog
- Release 0.7.0
- Release 0.7.0
- Release 0.7.0
- Relax from impl ([#1698](https://github.com/alloy-rs/alloy/issues/1698))
- EIP-7685 changes ([#1599](https://github.com/alloy-rs/alloy/issues/1599))
- Move from impls to where they belong ([#1691](https://github.com/alloy-rs/alloy/issues/1691))
- Add new fn to eip1186 ([#1692](https://github.com/alloy-rs/alloy/issues/1692))
- Make clippy happy ([#1677](https://github.com/alloy-rs/alloy/issues/1677))
- Export typed2718 ([#1678](https://github.com/alloy-rs/alloy/issues/1678))
- [ci] Edit cron syntax ([#1673](https://github.com/alloy-rs/alloy/issues/1673))
- Add default for txtype ([#1668](https://github.com/alloy-rs/alloy/issues/1668))
- Add num hash with parent ([#1652](https://github.com/alloy-rs/alloy/issues/1652))
- Add some proof fns ([#1645](https://github.com/alloy-rs/alloy/issues/1645))
- Add transactions iter fn ([#1646](https://github.com/alloy-rs/alloy/issues/1646))
- Add partialEq to txtype ([#1647](https://github.com/alloy-rs/alloy/issues/1647))

### Other

- Add ignored advisory back ([#1676](https://github.com/alloy-rs/alloy/issues/1676))
- Add unit tests for notification ([#1664](https://github.com/alloy-rs/alloy/issues/1664))
- Add unit tests for pubsub ([#1663](https://github.com/alloy-rs/alloy/issues/1663))
- Add unit tests for serde ttd ([#1662](https://github.com/alloy-rs/alloy/issues/1662))
- Add blanket impl of Transaction, TxReceipt and BlockHeader references ([#1657](https://github.com/alloy-rs/alloy/issues/1657))
- Add unit tests for tx envelope ([#1656](https://github.com/alloy-rs/alloy/issues/1656))
- Add `BlockWithParent` ([#1650](https://github.com/alloy-rs/alloy/issues/1650))
- Inline getters in impl of `Transaction` ([#1642](https://github.com/alloy-rs/alloy/issues/1642))

### Refactor

- [json-rpc] Small refactor for packet ([#1665](https://github.com/alloy-rs/alloy/issues/1665))

### Testing

- [node-bindings] Add unit tests for node-bindings utils and refac ([#1637](https://github.com/alloy-rs/alloy/issues/1637))
- [serde] Add unit tests for serde optional ([#1658](https://github.com/alloy-rs/alloy/issues/1658))
- [serde] Add unit tests for serde storage ([#1659](https://github.com/alloy-rs/alloy/issues/1659))
- Add test for 7702 with v ([#1644](https://github.com/alloy-rs/alloy/issues/1644))

## [0.6.4](https://github.com/alloy-rs/alloy/releases/tag/v0.6.4) - 2024-11-12

### Bug Fixes

- Make EIP-155 signatures logic safer ([#1641](https://github.com/alloy-rs/alloy/issues/1641))

### Miscellaneous Tasks

- Release 0.6.4

### Other

- Add trait method `Transaction::effective_gas_price` ([#1640](https://github.com/alloy-rs/alloy/issues/1640))

## [0.6.3](https://github.com/alloy-rs/alloy/releases/tag/v0.6.3) - 2024-11-12

### Bug Fixes

- Serde for transactions ([#1630](https://github.com/alloy-rs/alloy/issues/1630))
- [`rpc-types`] `FeeHistory` deser ([#1629](https://github.com/alloy-rs/alloy/issues/1629))

### Features

- [consensus] `TxEnvelope::signature` ([#1634](https://github.com/alloy-rs/alloy/issues/1634))
- [`network`] `AnyNetworkWallet` ([#1631](https://github.com/alloy-rs/alloy/issues/1631))

### Miscellaneous Tasks

- Release 0.6.3
- Ignore derivative ([#1639](https://github.com/alloy-rs/alloy/issues/1639))
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

- Wrap dashmap in Arc ([#1624](https://github.com/alloy-rs/alloy/issues/1624))
- [`provider`] Make `Caller` `EthCall` specific ([#1620](https://github.com/alloy-rs/alloy/issues/1620))
- Serde for `AnyTxEnvelope` ([#1613](https://github.com/alloy-rs/alloy/issues/1613))
- Receipt status serde ([#1608](https://github.com/alloy-rs/alloy/issues/1608))
- [signer-ledger] Use `SIGN_ETH_EIP_712` instruction ([#1479](https://github.com/alloy-rs/alloy/issues/1479))
- Hash handling ([#1604](https://github.com/alloy-rs/alloy/issues/1604))
- Fix typo in RecommendedFillers associated type ([#1536](https://github.com/alloy-rs/alloy/issues/1536))
- RLP for `TxEip4844` ([#1596](https://github.com/alloy-rs/alloy/issues/1596))
- Add more rlp correctness checks ([#1595](https://github.com/alloy-rs/alloy/issues/1595))
- Update AnyNetwork type aliases ([#1591](https://github.com/alloy-rs/alloy/issues/1591))
- Clearer replay protection checks ([#1581](https://github.com/alloy-rs/alloy/issues/1581))
- [`provider`] Return `Subscription<N::HeaderResponse>` ([#1586](https://github.com/alloy-rs/alloy/issues/1586))
- [alloy-provider] `get_block_by_number` arg ([#1582](https://github.com/alloy-rs/alloy/issues/1582))
- Relay types ([#1577](https://github.com/alloy-rs/alloy/issues/1577))
- Make a sensible encoding api ([#1496](https://github.com/alloy-rs/alloy/issues/1496))
- Enable std with jwt ([#1569](https://github.com/alloy-rs/alloy/issues/1569))

### Dependencies

- [deps] Bump wasmtimer ([#1588](https://github.com/alloy-rs/alloy/issues/1588))
- [deps] Bump alloy-rlp requirement ([#1587](https://github.com/alloy-rs/alloy/issues/1587))

### Documentation

- Expand on what `Requests` contains ([#1564](https://github.com/alloy-rs/alloy/issues/1564))

### Features

- [`serde`] StorageKeyKind ([#1597](https://github.com/alloy-rs/alloy/issues/1597))
- [rpc-types-trace/parity] Add creationMethod for create action ([#1621](https://github.com/alloy-rs/alloy/issues/1621))
- Integrate signature with boolean parity ([#1540](https://github.com/alloy-rs/alloy/issues/1540))
- Serde helpers for hashmaps and btreemaps with quantity key types ([#1579](https://github.com/alloy-rs/alloy/issues/1579))
- Use `OtherFields` on `UnknownTypedTransaction` ([#1605](https://github.com/alloy-rs/alloy/issues/1605))
- Implement Arbitrary for transaction types ([#1603](https://github.com/alloy-rs/alloy/issues/1603))
- Make more Otterscan types generic over header ([#1594](https://github.com/alloy-rs/alloy/issues/1594))
- Make Otterscan types generic over header ([#1593](https://github.com/alloy-rs/alloy/issues/1593))
- Add impl From<Header> for AnyHeader ([#1592](https://github.com/alloy-rs/alloy/issues/1592))
- [consensus] Protected Legacy Signature ([#1578](https://github.com/alloy-rs/alloy/issues/1578))
- Embed consensus header into RPC ([#1573](https://github.com/alloy-rs/alloy/issues/1573))
- Introduce `anvil_reorg` and related types. ([#1576](https://github.com/alloy-rs/alloy/issues/1576))
- Make eth_call and eth_estimateGas default to using Pending block ([#1568](https://github.com/alloy-rs/alloy/issues/1568))
- [eips] Indexed Blob Hash ([#1526](https://github.com/alloy-rs/alloy/issues/1526))

### Miscellaneous Tasks

- Release 0.6.0
- Add default to payloadattributes ([#1625](https://github.com/alloy-rs/alloy/issues/1625))
- Make withdrawals pub ([#1623](https://github.com/alloy-rs/alloy/issues/1623))
- Misc clippy ([#1607](https://github.com/alloy-rs/alloy/issues/1607))
- Fix some compile issues for no-std test ([#1606](https://github.com/alloy-rs/alloy/issues/1606))
- [meta] Update SECURITY.md ([#1584](https://github.com/alloy-rs/alloy/issues/1584))
- Add blockbody default ([#1559](https://github.com/alloy-rs/alloy/issues/1559))

### Other

- Use `ChainId` in `WalletCapabilities` ([#1622](https://github.com/alloy-rs/alloy/issues/1622))
- Rm useless `len` var in `rlp_encoded_fields_length` ([#1612](https://github.com/alloy-rs/alloy/issues/1612))
- Add unit tests for `OtherFields` ([#1614](https://github.com/alloy-rs/alloy/issues/1614))
- Small refactor for `JwtSecret` ([#1611](https://github.com/alloy-rs/alloy/issues/1611))
- Rm `Receipts` `root_slow` unused method ([#1567](https://github.com/alloy-rs/alloy/issues/1567))
- Embed TxEnvelope into `rpc-types-eth::Transaction` ([#1460](https://github.com/alloy-rs/alloy/issues/1460))
- Add success job ([#1589](https://github.com/alloy-rs/alloy/issues/1589))
- Add `BadBlock` type to `debug_getbadblocks` return type ([#1566](https://github.com/alloy-rs/alloy/issues/1566))
- Implement `root_slow` for `Receipts` ([#1563](https://github.com/alloy-rs/alloy/issues/1563))
- Add `uncle_block_from_header` impl and test ([#1554](https://github.com/alloy-rs/alloy/issues/1554))
- Add missing unit test for `MIN_PROTOCOL_BASE_FEE` ([#1558](https://github.com/alloy-rs/alloy/issues/1558))
- Rm `BEACON_CONSENSUS_REORG_UNWIND_DEPTH` ([#1556](https://github.com/alloy-rs/alloy/issues/1556))
- Add unit tests to secure all conversions and impl ([#1544](https://github.com/alloy-rs/alloy/issues/1544))
- Fix `HOLESKY_GENESIS_HASH` ([#1555](https://github.com/alloy-rs/alloy/issues/1555))
- Impl `From<Sealed<alloy_consensus::Header>>` for `Header` ([#1532](https://github.com/alloy-rs/alloy/issues/1532))

### Refactor

- [signer] Small refactor in signer utils ([#1615](https://github.com/alloy-rs/alloy/issues/1615))
- [genesis] Small refactor ([#1618](https://github.com/alloy-rs/alloy/issues/1618))

### Styling

- Move txtype-specific builders to network-primitives ([#1602](https://github.com/alloy-rs/alloy/issues/1602))

### Testing

- [network-primitives] Add unit tests for `BlockTransactions` ([#1619](https://github.com/alloy-rs/alloy/issues/1619))
- [transport] Add unit tests for `Authorization` methods ([#1616](https://github.com/alloy-rs/alloy/issues/1616))
- [json-rpc] Add unit tests for `Id` ([#1617](https://github.com/alloy-rs/alloy/issues/1617))
- Fix tests ([#1583](https://github.com/alloy-rs/alloy/issues/1583))

## [0.5.4](https://github.com/alloy-rs/alloy/releases/tag/v0.5.4) - 2024-10-23

### Bug Fixes

- Sidecar rlp decoding ([#1549](https://github.com/alloy-rs/alloy/issues/1549))

### Dependencies

- Bump alloy-eip7702 ([#1550](https://github.com/alloy-rs/alloy/issues/1550))

### Features

- Add osaka time to genesis ([#1548](https://github.com/alloy-rs/alloy/issues/1548))

### Miscellaneous Tasks

- Release 0.5.4

### Other

- Add unit test for `amount_wei` `Withdrawal` ([#1551](https://github.com/alloy-rs/alloy/issues/1551))

## [0.5.3](https://github.com/alloy-rs/alloy/releases/tag/v0.5.3) - 2024-10-22

### Bug Fixes

- Correct implementations of Encodable and Decodable for sidecars ([#1528](https://github.com/alloy-rs/alloy/issues/1528))
- [filter] Treat null fields as null ([#1529](https://github.com/alloy-rs/alloy/issues/1529))
- Maybetagged serde for typed transaction ([#1495](https://github.com/alloy-rs/alloy/issues/1495))

### Dependencies

- Bump alloy-eip7702 ([#1547](https://github.com/alloy-rs/alloy/issues/1547))

### Documentation

- [prestate] Comment prestate more clear ([#1527](https://github.com/alloy-rs/alloy/issues/1527))

### Features

- [rpc-types-trace/prestate] Support disable_{code,storage} ([#1538](https://github.com/alloy-rs/alloy/issues/1538))
- Derive serde for `ExecutionPayloadSidecar` ([#1535](https://github.com/alloy-rs/alloy/issues/1535))

### Miscellaneous Tasks

- Release 0.5.3
- Remove self from codeowners ([#1498](https://github.com/alloy-rs/alloy/issues/1498))

### Other

- Add `Debug` trait bound for `Transaction` trait ([#1543](https://github.com/alloy-rs/alloy/issues/1543))
- Impl `From<RpcBlockHash>` for `BlockId` ([#1539](https://github.com/alloy-rs/alloy/issues/1539))
- Small refactor with `then_some` ([#1533](https://github.com/alloy-rs/alloy/issues/1533))
- Add unit tests and reduce paths ([#1531](https://github.com/alloy-rs/alloy/issues/1531))
- Use `Withdrawals` wrapper in `BlockBody` ([#1525](https://github.com/alloy-rs/alloy/issues/1525))

### Testing

- Fix more ci only ([#1402](https://github.com/alloy-rs/alloy/issues/1402))

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

### Features

- Add ExecutionPayloadSidecar type ([#1517](https://github.com/alloy-rs/alloy/issues/1517))

### Miscellaneous Tasks

- Release 0.5.1
- Extract error types to new modules ([#1518](https://github.com/alloy-rs/alloy/issues/1518))
- Add empty requests constant ([#1519](https://github.com/alloy-rs/alloy/issues/1519))
- Remove 7685 request variants ([#1515](https://github.com/alloy-rs/alloy/issues/1515))
- Remove redundant cfgs ([#1516](https://github.com/alloy-rs/alloy/issues/1516))

## [0.5.0](https://github.com/alloy-rs/alloy/releases/tag/v0.5.0) - 2024-10-18

### Bug Fixes

- [`rpc-types-eth`] Receipt deser ([#1506](https://github.com/alloy-rs/alloy/issues/1506))
- Use `requests_hash` ([#1508](https://github.com/alloy-rs/alloy/issues/1508))
- Allow missing-tag deser of tx envelope ([#1489](https://github.com/alloy-rs/alloy/issues/1489))
- Correct default impls to not bound T ([#1490](https://github.com/alloy-rs/alloy/issues/1490))
- Rename gas_limit to gas in serde def for txns ([#1486](https://github.com/alloy-rs/alloy/issues/1486))
- Types inside mev_calls.rs ([#1435](https://github.com/alloy-rs/alloy/issues/1435))
- [wasm] Support ws ([#1481](https://github.com/alloy-rs/alloy/issues/1481))
- [types/filter] Treat empty filter address as non-matching  ([#1473](https://github.com/alloy-rs/alloy/issues/1473))
- Remove signature assoc type from tx response trait ([#1451](https://github.com/alloy-rs/alloy/issues/1451))
- Change bound in RecommendedFillers to TxFiller<Self> ([#1466](https://github.com/alloy-rs/alloy/issues/1466))
- Make RecommendedFillers generic over Network ([#1458](https://github.com/alloy-rs/alloy/issues/1458))
- Enable serde on alloy-consensus ([#1449](https://github.com/alloy-rs/alloy/issues/1449))
- Proposer_index rustdoc ([#1443](https://github.com/alloy-rs/alloy/issues/1443))
- [eips] Blob Sidecar Item Serde ([#1441](https://github.com/alloy-rs/alloy/issues/1441))
- [rpc-client] Use wasm-compatible sleep ([#1437](https://github.com/alloy-rs/alloy/issues/1437))
- Enforce correct parity for legacy transactions ([#1428](https://github.com/alloy-rs/alloy/issues/1428))
- [provider] Use wasmtimer for wasm32 target ([#1426](https://github.com/alloy-rs/alloy/issues/1426))
- Set chain id for eth signer ([#1425](https://github.com/alloy-rs/alloy/issues/1425))

### Dependencies

- Enable serde types dependencies in rpc-types ([#1456](https://github.com/alloy-rs/alloy/issues/1456))

### Features

- Wallet namespace types ([#1448](https://github.com/alloy-rs/alloy/issues/1448))
- Make it possible to configure Ws config ([#1505](https://github.com/alloy-rs/alloy/issues/1505))
- [eip4895] Implement `Withdrawals` ([#1462](https://github.com/alloy-rs/alloy/issues/1462))
- Port generate_blob_sidecar ([#1511](https://github.com/alloy-rs/alloy/issues/1511))
- Make Pending transaction own the provider ([#1500](https://github.com/alloy-rs/alloy/issues/1500))
- Add missing eth_getTransaction methods ([#1457](https://github.com/alloy-rs/alloy/issues/1457))
- From impl for variant ([#1488](https://github.com/alloy-rs/alloy/issues/1488))
- BuildTransactionErr abstract over builder type ([#1452](https://github.com/alloy-rs/alloy/issues/1452))
- [provider] LRUCache Layer ([#954](https://github.com/alloy-rs/alloy/issues/954))
- Add helpers to configure GethDebugTracingOptions properly ([#1436](https://github.com/alloy-rs/alloy/issues/1436))
- [eips] Arbitrary BaseFeeParams ([#1432](https://github.com/alloy-rs/alloy/issues/1432))
- `Encodable2718::network_len` ([#1431](https://github.com/alloy-rs/alloy/issues/1431))
- Re-export more features from alloy-core ([#1423](https://github.com/alloy-rs/alloy/issues/1423))
- [rpc-types-mev] Add mev-share sse types ([#1419](https://github.com/alloy-rs/alloy/issues/1419))
- [rpc-types-mev] Add support for `Bundle` inside `BundleItem` ([#1418](https://github.com/alloy-rs/alloy/issues/1418))
- Add helper from impl ([#1407](https://github.com/alloy-rs/alloy/issues/1407))

### Miscellaneous Tasks

- Release 0.5.0
- Update pectra system contracts bytecodes & addresses ([#1512](https://github.com/alloy-rs/alloy/issues/1512))
- Flatten eip-7685 requests into a single opaque list ([#1383](https://github.com/alloy-rs/alloy/issues/1383))
- Rename requests root to requests hash ([#1379](https://github.com/alloy-rs/alloy/issues/1379))
- Refactor some match with same arms ([#1463](https://github.com/alloy-rs/alloy/issues/1463))
- [consensus] Test use Vec::with_capacity ([#1476](https://github.com/alloy-rs/alloy/issues/1476))
- Unify use Option ref ([#1477](https://github.com/alloy-rs/alloy/issues/1477))
- Update eip-7251 bytecode and address ([#1380](https://github.com/alloy-rs/alloy/issues/1380))
- More simplifications ([#1469](https://github.com/alloy-rs/alloy/issues/1469))
- Some lifetime simplifications ([#1467](https://github.com/alloy-rs/alloy/issues/1467))
- Remove redundant else ([#1468](https://github.com/alloy-rs/alloy/issues/1468))
- Rm needless pass by ref mut ([#1465](https://github.com/alloy-rs/alloy/issues/1465))
- Some small improvements ([#1461](https://github.com/alloy-rs/alloy/issues/1461))
- Use pending for next initial nonce ([#1455](https://github.com/alloy-rs/alloy/issues/1455))
- [rpc] Make keys required for execution witness ([#1446](https://github.com/alloy-rs/alloy/issues/1446))
- [deny] Allow Zlib ([#1438](https://github.com/alloy-rs/alloy/issues/1438))
- [rpc] Make TransactionRequest conversions exhaustive ([#1427](https://github.com/alloy-rs/alloy/issues/1427))
- Apply same member order ([#1408](https://github.com/alloy-rs/alloy/issues/1408))

### Other

- Update fn encoded_2718 ([#1475](https://github.com/alloy-rs/alloy/issues/1475))
- Add unit tests for `ConsolidationRequest` ([#1497](https://github.com/alloy-rs/alloy/issues/1497))
- Rm redundant root hash definitions ([#1501](https://github.com/alloy-rs/alloy/issues/1501))
- Add unit tests for `WithdrawalRequest` ([#1472](https://github.com/alloy-rs/alloy/issues/1472))
- Add more constraints to `TxReceipt` trait ([#1478](https://github.com/alloy-rs/alloy/issues/1478))
- Replace `to` by `kind` in Transaction trait ([#1484](https://github.com/alloy-rs/alloy/issues/1484))
- Add more unit tests ([#1464](https://github.com/alloy-rs/alloy/issues/1464))
- GenesisAccount : implement `deserialize_private_key` ([#1447](https://github.com/alloy-rs/alloy/issues/1447))
- Revert test: update test cases with addresses ([#1358](https://github.com/alloy-rs/alloy/issues/1358)) ([#1444](https://github.com/alloy-rs/alloy/issues/1444))
- Add default to payload id ([#1442](https://github.com/alloy-rs/alloy/issues/1442))
- Replace assert_eq! with similar_asserts::assert_eq! ([#1429](https://github.com/alloy-rs/alloy/issues/1429))

### Performance

- Manual serde for quantity vec ([#1509](https://github.com/alloy-rs/alloy/issues/1509))

### Refactor

- Change input output to Bytes ([#1487](https://github.com/alloy-rs/alloy/issues/1487))

### Styling

- Fmt ([#1439](https://github.com/alloy-rs/alloy/issues/1439))

### Testing

- [node-bindings] Consolidate integration tests ([#1422](https://github.com/alloy-rs/alloy/issues/1422))

## [0.4.2](https://github.com/alloy-rs/alloy/releases/tag/v0.4.2) - 2024-10-01

### Miscellaneous Tasks

- Release 0.4.2

### Styling

- Use alloc ([#1405](https://github.com/alloy-rs/alloy/issues/1405))

## [0.4.1](https://github.com/alloy-rs/alloy/releases/tag/v0.4.1) - 2024-10-01

### Bug Fixes

- Safe match for next base fee ([#1399](https://github.com/alloy-rs/alloy/issues/1399))

### Dependencies

- Bump alloy-eip7702

### Features

- [consensus] Bincode compatibility for EIP-7702 ([#1404](https://github.com/alloy-rs/alloy/issues/1404))

### Miscellaneous Tasks

- Release 0.4.1
- [consensus] Less derives for bincode compatible types ([#1401](https://github.com/alloy-rs/alloy/issues/1401))

## [0.4.0](https://github.com/alloy-rs/alloy/releases/tag/v0.4.0) - 2024-09-30

### Bug Fixes

- Support u64 hex from str for BlockId ([#1396](https://github.com/alloy-rs/alloy/issues/1396))
- Ensure `max_fee_per_blob_gas` field handles `Some(0)` gracefully ([#1389](https://github.com/alloy-rs/alloy/issues/1389))
- Advance buffer during 2718 decoding ([#1367](https://github.com/alloy-rs/alloy/issues/1367))
- Use std::error ([#1363](https://github.com/alloy-rs/alloy/issues/1363))
- Correct `encode_2718_len` for legacy transactions ([#1360](https://github.com/alloy-rs/alloy/issues/1360))
- `Error::source` for `Eip2718Error` ([#1361](https://github.com/alloy-rs/alloy/issues/1361))
- [serde] Encode optional types as Some ([#1348](https://github.com/alloy-rs/alloy/issues/1348))
- `eth_simulateV1` serde ([#1345](https://github.com/alloy-rs/alloy/issues/1345))
- Use vec for flat call frame ([#1343](https://github.com/alloy-rs/alloy/issues/1343))
- [`rpc-client`] Add test for BuiltInConnString.connect_boxed ([#1331](https://github.com/alloy-rs/alloy/issues/1331))
- RecommendedFillers typo ([#1311](https://github.com/alloy-rs/alloy/issues/1311))
- Enforce correct parity encoding for typed transactions ([#1305](https://github.com/alloy-rs/alloy/issues/1305))

### Dependencies

- Bump alloy 0.8.5 ([#1374](https://github.com/alloy-rs/alloy/issues/1374))
- [deps] Bump alloy-core 0.8.4 in Cargo.toml ([#1364](https://github.com/alloy-rs/alloy/issues/1364))
- [deps] Bump breaking deps ([#1356](https://github.com/alloy-rs/alloy/issues/1356))

### Features

- [consensus] Bincode compatibility for header and transaction types ([#1397](https://github.com/alloy-rs/alloy/issues/1397))
- [rpc-types-engine] Use strum for ClientCode ([#1386](https://github.com/alloy-rs/alloy/issues/1386))
- Replace std/hashbrown with alloy_primitives::map ([#1384](https://github.com/alloy-rs/alloy/issues/1384))
- [engine] Add Trin Execution client code ([#1372](https://github.com/alloy-rs/alloy/issues/1372))
- [signer-local] Add `keystore-geth-compat` feature ([#1381](https://github.com/alloy-rs/alloy/issues/1381))
- Errors for responses ([#1369](https://github.com/alloy-rs/alloy/issues/1369))
- [transport-http] JWT auth layer ([#1314](https://github.com/alloy-rs/alloy/issues/1314))
- Impl From<Eip2718Error> for alloy_rlp::Error ([#1359](https://github.com/alloy-rs/alloy/issues/1359))
- Add Header::num_hash_slow ([#1357](https://github.com/alloy-rs/alloy/issues/1357))
- Blob Tx Sidecar Iterator ([#1334](https://github.com/alloy-rs/alloy/issues/1334))
- Deserialize requests ([#1351](https://github.com/alloy-rs/alloy/issues/1351))
- [serde] Remove deprecated `num` module ([#1350](https://github.com/alloy-rs/alloy/issues/1350))
- [consensus] Generic Block Type ([#1319](https://github.com/alloy-rs/alloy/issues/1319))
- [provider] Subscribe to new blocks if possible in heartbeat ([#1321](https://github.com/alloy-rs/alloy/issues/1321))
- Add getters into TransactionResponse and update implementations  ([#1328](https://github.com/alloy-rs/alloy/issues/1328))
- [consensus] Move requests struct definition from reth ([#1326](https://github.com/alloy-rs/alloy/issues/1326))
- Add builder style function to simulate payload args ([#1324](https://github.com/alloy-rs/alloy/issues/1324))
- Add builder style functions to ethcallbundle ([#1325](https://github.com/alloy-rs/alloy/issues/1325))
- Add eth_simulateV1 ([#1323](https://github.com/alloy-rs/alloy/issues/1323))
- [rpc-types-beacon] `BuilderBlockValidationRequestV4` ([#1322](https://github.com/alloy-rs/alloy/issues/1322))
- [rpc-types-beacon] `BuilderBlockValidationRequestV3` ([#1310](https://github.com/alloy-rs/alloy/issues/1310))
- Bundle hash on ethsendbundle ([#1308](https://github.com/alloy-rs/alloy/issues/1308))

### Miscellaneous Tasks

- Release 0.4.0
- Rm outdated comments ([#1392](https://github.com/alloy-rs/alloy/issues/1392))
- Move type def to where it belongs ([#1391](https://github.com/alloy-rs/alloy/issues/1391))
- Update comment to be more accurate ([#1390](https://github.com/alloy-rs/alloy/issues/1390))
- Use std::error
- Fix warnings on no_std ([#1355](https://github.com/alloy-rs/alloy/issues/1355))
- Add codes into execution witness ([#1352](https://github.com/alloy-rs/alloy/issues/1352))
- Remove an unused lifetime ([#1336](https://github.com/alloy-rs/alloy/issues/1336))
- Fix some warnings ([#1320](https://github.com/alloy-rs/alloy/issues/1320))
- Reexport BlobAndProofV1

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
- Make factory and paymaster fields optional in `PackedUserOperation` ([#1330](https://github.com/alloy-rs/alloy/issues/1330))
- Add `BlockHeader` getter trait ([#1302](https://github.com/alloy-rs/alloy/issues/1302))
- Remove repetitive as_ref ([#1329](https://github.com/alloy-rs/alloy/issues/1329))
- Add `OperationType::OpEofCreate` ([#1327](https://github.com/alloy-rs/alloy/issues/1327))
- Implement custom default for `Account` representing a valid empty account ([#1313](https://github.com/alloy-rs/alloy/issues/1313))

### Styling

- Make tests that require binaries in path CI only ([#1393](https://github.com/alloy-rs/alloy/issues/1393))

### Testing

- Add retry test ([#1373](https://github.com/alloy-rs/alloy/issues/1373))
- Update test cases with addresses ([#1358](https://github.com/alloy-rs/alloy/issues/1358))

## [0.3.6](https://github.com/alloy-rs/alloy/releases/tag/v0.3.6) - 2024-09-18

### Bug Fixes

- [types-eth] Optional Alloy Serde ([#1284](https://github.com/alloy-rs/alloy/issues/1284))
- `eth_simulateV1` ([#1289](https://github.com/alloy-rs/alloy/issues/1289))

### Features

- Add block num hash helper ([#1304](https://github.com/alloy-rs/alloy/issues/1304))
- ProviderCall ([#788](https://github.com/alloy-rs/alloy/issues/788))
- [rpc-types-beacon] `SignedBidSubmissionV4` ([#1303](https://github.com/alloy-rs/alloy/issues/1303))
- [transport-http] Layer client ([#1227](https://github.com/alloy-rs/alloy/issues/1227))
- Add blob and proof v1 ([#1300](https://github.com/alloy-rs/alloy/issues/1300))
- Add types for flat call tracer ([#1292](https://github.com/alloy-rs/alloy/issues/1292))
- [`node-bindings`] Support appending extra args ([#1299](https://github.com/alloy-rs/alloy/issues/1299))

### Miscellaneous Tasks

- Release 0.3.6
- [rpc] Rename witness fields ([#1293](https://github.com/alloy-rs/alloy/issues/1293))
- [engine] `no_std` Checks ([#1298](https://github.com/alloy-rs/alloy/issues/1298))

### Refactor

- Separate transaction builders for tx types ([#1259](https://github.com/alloy-rs/alloy/issues/1259))

## [0.3.5](https://github.com/alloy-rs/alloy/releases/tag/v0.3.5) - 2024-09-13

### Bug Fixes

- Add missing conversion ([#1287](https://github.com/alloy-rs/alloy/issues/1287))

### Miscellaneous Tasks

- Release 0.3.5
- Release 0.3.5

## [0.3.4](https://github.com/alloy-rs/alloy/releases/tag/v0.3.4) - 2024-09-13

### Bug Fixes

- `debug_traceCallMany` and `trace_callMany` ([#1278](https://github.com/alloy-rs/alloy/issues/1278))
- Serde for `eth_simulateV1` ([#1273](https://github.com/alloy-rs/alloy/issues/1273))

### Features

- [engine] Optional Serde ([#1283](https://github.com/alloy-rs/alloy/issues/1283))
- [alloy-rpc-types-eth] Optional serde ([#1276](https://github.com/alloy-rs/alloy/issues/1276))
- Improve node bindings ([#1279](https://github.com/alloy-rs/alloy/issues/1279))
- Add serde for NumHash ([#1277](https://github.com/alloy-rs/alloy/issues/1277))
- [engine] No_std engine types ([#1268](https://github.com/alloy-rs/alloy/issues/1268))
- No_std eth rpc types ([#1252](https://github.com/alloy-rs/alloy/issues/1252))

### Miscellaneous Tasks

- Release 0.3.4
- Remove eth rpc types dep from engine types ([#1280](https://github.com/alloy-rs/alloy/issues/1280))
- Swap `BlockHashOrNumber` alias and struct name ([#1270](https://github.com/alloy-rs/alloy/issues/1270))
- [consensus] Remove Header Method ([#1271](https://github.com/alloy-rs/alloy/issues/1271))
- [consensus] Alloc by Default ([#1272](https://github.com/alloy-rs/alloy/issues/1272))
- [network-primitives] Remove alloc Vec Dep ([#1267](https://github.com/alloy-rs/alloy/issues/1267))

### Other

- Add trait methods `cumulative_gas_used` and `state_root` to `ReceiptResponse` ([#1275](https://github.com/alloy-rs/alloy/issues/1275))
- Implement `seal` helper for `Header` ([#1269](https://github.com/alloy-rs/alloy/issues/1269))

## [0.3.3](https://github.com/alloy-rs/alloy/releases/tag/v0.3.3) - 2024-09-10

### Bug Fixes

- [rpc-types-trace] Use rpc-types Log in OtsReceipt ([#1261](https://github.com/alloy-rs/alloy/issues/1261))

### Features

- [rpc-types-trace] Always serialize result if no error ([#1258](https://github.com/alloy-rs/alloy/issues/1258))

### Miscellaneous Tasks

- Release 0.3.3
- Require destination for 7702 ([#1262](https://github.com/alloy-rs/alloy/issues/1262))
- Swap BlockNumHash alias and struct name ([#1265](https://github.com/alloy-rs/alloy/issues/1265))

### Other

- Implement `AsRef` for `Header` ([#1260](https://github.com/alloy-rs/alloy/issues/1260))

### Testing

- Dont use fork test ([#1263](https://github.com/alloy-rs/alloy/issues/1263))

## [0.3.2](https://github.com/alloy-rs/alloy/releases/tag/v0.3.2) - 2024-09-09

### Bug Fixes

- [consensus] Remove Unused Alloc Vecs ([#1250](https://github.com/alloy-rs/alloy/issues/1250))

### Dependencies

- Bump tower to 0.5 ([#1249](https://github.com/alloy-rs/alloy/issues/1249))

### Features

- No_std network primitives ([#1248](https://github.com/alloy-rs/alloy/issues/1248))
- [rpc-types-eth] AnyBlock ([#1243](https://github.com/alloy-rs/alloy/issues/1243))
- Add Reth node bindings ([#1092](https://github.com/alloy-rs/alloy/issues/1092))
- [rpc-types-engine] Add forkchoice state zero helpers ([#1231](https://github.com/alloy-rs/alloy/issues/1231))
- [network-primitives] Expose more fields via block response traits ([#1229](https://github.com/alloy-rs/alloy/issues/1229))

### Miscellaneous Tasks

- Release 0.3.2
- Add aliases for Num Hash ([#1253](https://github.com/alloy-rs/alloy/issues/1253))
- Add helpers for beacon blob bundle ([#1254](https://github.com/alloy-rs/alloy/issues/1254))
- [eip1898] Display `RpcBlockHash` ([#1242](https://github.com/alloy-rs/alloy/issues/1242))
- Optional derive more ([#1239](https://github.com/alloy-rs/alloy/issues/1239))
- Derive more default features false ([#1230](https://github.com/alloy-rs/alloy/issues/1230))

### Other

- Add getter trait methods to `ReceiptResponse` ([#1251](https://github.com/alloy-rs/alloy/issues/1251))
- Impl `exceeds_allowed_future_timestamp` for `Header` ([#1237](https://github.com/alloy-rs/alloy/issues/1237))
- Impl `is_zero_difficulty` for `Header` ([#1236](https://github.com/alloy-rs/alloy/issues/1236))
- Impl parent_num_hash for Header ([#1238](https://github.com/alloy-rs/alloy/issues/1238))
- Implement `Arbitrary` for `Header` ([#1235](https://github.com/alloy-rs/alloy/issues/1235))

## [0.3.1](https://github.com/alloy-rs/alloy/releases/tag/v0.3.1) - 2024-09-02

### Bug Fixes

- Anvil builder default port ([#1213](https://github.com/alloy-rs/alloy/issues/1213))
- [eips] No-std compat ([#1222](https://github.com/alloy-rs/alloy/issues/1222))
- Value of TxEip1559.ty ([#1210](https://github.com/alloy-rs/alloy/issues/1210))

### Dependencies

- Bump rust msrv to 1.78 ([#1219](https://github.com/alloy-rs/alloy/issues/1219))

### Documentation

- Update version ([#1211](https://github.com/alloy-rs/alloy/issues/1211))

### Features

- [`json-rpc`] Implement From U256 and String for SubId ([#1226](https://github.com/alloy-rs/alloy/issues/1226))
- Workflow to validate no_std compatibility ([#1223](https://github.com/alloy-rs/alloy/issues/1223))
- Derive `arbitrary::Arbitrary` for `TxEip7702` ([#1216](https://github.com/alloy-rs/alloy/issues/1216))
- Implement `tx_type` for `TxEip7702` ([#1214](https://github.com/alloy-rs/alloy/issues/1214))
- [alloy-provider] Add abstraction for `NonceFiller` behavior ([#1108](https://github.com/alloy-rs/alloy/issues/1108))

### Miscellaneous Tasks

- Release 0.3.1
- [README] Add a link to `rpc-types-debug` ([#1212](https://github.com/alloy-rs/alloy/issues/1212))
- [features] Enable `consensus` and `network` along with `providers` ([#1207](https://github.com/alloy-rs/alloy/issues/1207))

### Other

- Rm useless methods for `TxEip7702` ([#1221](https://github.com/alloy-rs/alloy/issues/1221))

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

- Release 0.3.0
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
- Add required software to run tests in Contributing.md ([#627](https://github.com/alloy-rs/alloy/issues/627))
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
- Duplicate functions of  in crates/contract/src/call.rs ([#534](https://github.com/alloy-rs/alloy/issues/534)) ([#726](https://github.com/alloy-rs/alloy/issues/726))
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
- Improve and complete `alloy` prelude crate feature flag compatibility ([#421](https://github.com/alloy-rs/alloy/issues/421))
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
- Wasm-compatibility
- Wasm-compatibility
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
- Improve id docs and set
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
