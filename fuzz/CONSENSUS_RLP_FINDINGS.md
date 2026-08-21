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
| `Log::decode` accepts an RLP string in place of a list | receipt-envelope injectivity | alloy-core `fix/log-rlp-require-list` (`dfd5e33`) | Unfixed in alloy-primitives 1.6.1; malformed log containers normalize to canonical lists |

`EthereumTxEnvelope` deliberately does not retain whether a legacy transaction arrived with an
optional `0x00` type prefix. The injectivity property skips only that explicitly tagged form; it
still checks ordinary untagged legacy transactions and every nonzero typed encoding.
