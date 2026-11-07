# alloy fuzz harnesses

Coverage-guided fuzz targets for the transaction wire-format decoders in
`alloy-consensus` and `alloy-eips`. These exercise the same parsing entry
points that every Ethereum node hits on JSON-RPC, devp2p, and txpool gossip
input — anything that crashes here is reachable from untrusted bytes.

## Targets

| Name | Decoder | Entry point |
| --- | --- | --- |
| `tx_envelope_2718` | `TxEnvelope::decode_2718` | EIP-2718 typed transaction envelope (legacy / 2930 / 1559 / 4844 / 7702) |
| `pooled_transaction_2718` | `PooledTransaction::decode_2718` | EIP-2718 envelope with embedded EIP-4844 blob sidecar (devp2p `PooledTransactions`) |
| `blob_sidecar_eip4844` | `BlobTransactionSidecar::decode` | RLP-encoded blob sidecar (blobs + commitments + KZG proofs) |

Each target asserts a decode → encode → decode roundtrip equality, so silent
corruption of internal state (not just panics) is caught.

## Running

Requires `cargo-fuzz` and a nightly toolchain:

```sh
rustup toolchain install nightly
cargo install cargo-fuzz

cd fuzz
cargo +nightly fuzz run tx_envelope_2718 -- -max_total_time=600
cargo +nightly fuzz run pooled_transaction_2718 -- -max_total_time=600
cargo +nightly fuzz run blob_sidecar_eip4844 -- -max_total_time=600
```

The `tx_envelope_2718` and `pooled_transaction_2718` corpora are seeded from
the existing `crates/consensus/testdata/{4844rlp,7594rlp}/*.rlp` fixtures
(hex-decoded). The blob sidecar corpus starts empty and is grown by
libFuzzer.

## Reporting crashes

Reproducer files land in `fuzz/artifacts/<target>/`. Please open an issue
with the minimized artifact attached so the decoder can be tightened
without exposing the input to anyone reading the bug tracker before a fix
ships.
