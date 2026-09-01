# Consensus RLP fuzz findings

The `consensus_rlp` target applies byte-level decode/encode injectivity, exact EIP-2718 decoding,
error-cursor preservation, and sealed-container boundary properties to top-level consensus types.
Generated corpus entries are canonical values; no known crashing input is embedded in the target.

| Finding | Detecting property | Isolated resolution | Tempo/reth status |
| --- | --- | --- | --- |
| [alloy#4058](https://github.com/alloy-rs/alloy/pull/4058): a string can be interpreted as omitted withdrawals | `BlockBody` and `Block` injectivity | `fix/block-withdrawals-rlp-canonicality` (`bca8a68e`); upstream fix `8bd4bd09` | Fixed in Alloy 2.3.0 used by both products |
| [alloy#4059](https://github.com/alloy-rs/alloy/pull/4059): a failed fallback variant advances the shared cursor | top-level envelope injectivity and error-cursor preservation | `fix/fallback-decode-cursor-4059` (`65883fe5`) | Fixed in Alloy 2.3.0 used by both products |
| `Block::decode_sealed` ignores its outer RLP boundary | sealed block injectivity | `fix/decode-sealed-outer-boundary` (`10d152c0`) | Unfixed in Alloy 2.3.0; Tempo's caller pre-slices under-declared inputs, although trailing-item canonicality remains |
| `network_decode` accepts an unwrapped typed transaction | top-level `BasicTxEnvelope` injectivity | `fix/reject-unwrapped-typed-network` (`71d51708`) | Unfixed in Alloy 2.3.0; reachable in network/container decoding, with semantic transaction validation unchanged |
| `Eip658Value` accepts noncanonical one-byte status values | receipt-envelope injectivity | `fix/reject-invalid-eip658-status` (`aa619212`) | Unfixed in Alloy 2.3.0; malformed receipts normalize before downstream use |
| `Log::decode` accepts an RLP string in place of a list | receipt-envelope injectivity | [alloy-core#1166](https://github.com/alloy-rs/core/pull/1166), released in alloy-primitives 1.7.1 | Fixed in the fuzz lockfile; malformed log containers normalized to canonical lists on 1.6.1 |
| [alloy#4167](https://github.com/alloy-rs/alloy/pull/4167): an explicit type-`0x00` prefix is accepted as legacy and discarded on re-encoding | transaction-envelope injectivity | `fix(consensus): reject 0x00-tagged legacy transactions in typed_decode` (`b84d74db`) | Parser canonicality defect; reth rejects affected blocks through transaction-root validation |
| `AnyReceiptEnvelope` accepts an explicit type-`0x00` prefix and discards it on re-encoding | any-receipt-envelope injectivity | local, unopened fix; split typed decoding from the untagged fallback | Low-severity parser canonicality; no Reth ingress, and only test/RPC use found in Tempo |
| `ReceiptWithBloom<EthereumReceipt>` accepts an explicit type-`0x00` prefix and discards it on re-encoding | generic Ethereum-receipt EIP-2718 injectivity | local, unopened fix; reject zero in `typed_decode_with_bloom` | Reachable in Reth/Tempo receipt wire decoding; downstream receipt-root checks prevent consensus impact |

There are no intentional injectivity exclusions. Explicitly type-`0x00`-prefixed legacy inputs are
invalid EIP-2718 encodings and remain in the transaction-envelope and pooled-envelope oracles.
