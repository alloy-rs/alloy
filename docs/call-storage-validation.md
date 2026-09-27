# Pending EthCall storage

This optimization is based on upstream `c6a2f8c472e28beff4d3e65c234c829b05843c6a`
(Alloy 2.5.0). A pending `EthCallFut` reserves inline space for its largest state
and `ProviderCall` variant even when a caller returns a small boxed reply future.
Boxing preparation parameters and storing only the active reply variant removes
that unused space from the pending future. An existing boxed reply is reused.
The preparation allocation releases on first poll.

This trades additional allocations for smaller retained pending futures. It is
not a correctness fix or evidence of a memory leak. Method names, transaction
parameters, block pins, overrides, response mapping, lazy polling, and
cancellation behavior remain unchanged. Whole-process memory and throughput
benefits have not been established.

## Reproduction

`crates/provider/tests/call_storage.rs` uses a thread-local System-allocator
counter around 64 manually polled pending calls. It counts requested bytes,
including nested allocations and reallocations, and checks complete release
after both completion and cancellation. The test caller returns gated boxed
replies through the public `Caller` API.

On macOS arm64, the four allocation regressions fail on the original source at
`2e211ac8dc1e8dd5b32fb377d727e08fb6fd206b`, retaining 64,000 requested bytes per
cohort. Four behavioral controls pass. These measurements exclude allocator
fragmentation and are not RSS measurements. The 256-byte per-call bound is a
test budget, not a protocol requirement.

The suite covers `eth_call` and `eth_estimateGas`, exact transaction/block/state
and block overrides, borrowed non-Send mappers, original preparation/reply
errors, dropping unpolled futures, Ready/Waiter/RpcCall replies, and a successful
request after cancellation. It does not contact an external RPC service.

## Validation

Use the committed lockfile and `nightly-2026-08-25`:

```sh
cargo +nightly-2026-08-25 fmt --all --check
cargo +nightly-2026-08-25 nextest run --locked -p alloy-provider --test call_storage --no-fail-fast --nocapture --retries 0
cargo +nightly-2026-08-25 nextest run --locked -p alloy-provider -p alloy-contract --all-features --no-fail-fast --retries 0 --test-threads 2
cargo +nightly-2026-08-25 clippy --locked -p alloy-provider -p alloy-contract --all-features --all-targets -- -D warnings
```

All eight focused tests pass with the optimization, retaining 4,096 requested
bytes for each 64-call cohort. The broader provider/contract run passes 309 tests
with 21 skipped. Strict Clippy passes with no command-line lint exceptions, and
formatting passes. No dependency versions or lockfile entries changed.

These checks used one build job and
`RUSTFLAGS='--cfg tokio_unstable -Ctarget-cpu=native'` on macOS arm64. The source
and lock fingerprint was
`48b9679d451818cd72aa34df987cf795dadba14f66f4cd9fc64f7a6e44c4830a` throughout.
The requested-byte reduction applies to the pending boxed-reply workload above;
no whole-application throughput or RSS claim follows from it.
