# EIP-8141 integration and migration

Frame transactions have no outer ECDSA signature, top-level calldata, or
conventional receipt payload. These differences are explicit in the API:

- `EthereumTxEnvelope::signature()` and `input_mut()` return `Option`.
- Envelope `into_signed()`, including the RPC transaction helpers, returns
  `Result`. Use the corresponding `TryFrom` conversion instead of `From`.
  Keep frame transactions as sealed envelopes; do not invent an outer signature.
- `ReceiptEnvelope::from_typed()` and `into_receipt()` return `Result`.
  Construct frame receipts from `FrameReceiptPayload` and access them with
  `as_eip8141()`.
- `ReceiptEnvelope::logs_bloom()` returns a bloom by value, computing it for
  frame receipts. Conventional receipts retain their cached bloom.
- `AnyReceiptEnvelope` distinguishes `Ethereum(ReceiptEnvelope)` from
  `Other { inner, type }`. Use `tx_type()`, `logs()`, and `bloom()` instead
  of reading the former common fields. `bloom_ref()` returns `None` when the
  bloom must be computed. Unknown conventional receipt formats remain supported.

The aggregate frame receipt status means “all frames succeeded.” This is a
convenience policy, not a consensus field in the frame receipt encoding.

## Building and sending

The provider's wallet filler preserves an EIP-7594 sidecar in
`SendableTx::EnvelopeWithSidecar`. The canonical envelope stays sidecar-free;
the separately encoded bytes are sent through the raw transaction endpoint.
Gas fillers do not estimate or overwrite frame gas fields: provide all frame
budgets and fees before submission. Frame authorization remains the caller's
responsibility, with validity determined during execution.

For manual raw submission, consume the request with
`build_8141_with_sidecar()` and submit its `encoded_2718()` bytes.
`build_8141()` / `build_consensus_tx()` produce canonical data for signing
hashes and simulation, not a sidecar-bearing network transaction.
`build_typed_tx()` and the wallet's direct unsigned builder reject frame blob
requests instead of silently discarding their sidecars.

Bare frame transactions with blob hashes cannot be converted to or decoded as
pooled transactions. The relevant `From` conversions are now `TryFrom`.
An eth/72 sidecar with blob payloads stripped is still a sidecar and remains
supported.

The pooled frame variant contains a sealed `CachedFrameTransaction`.
Its gas limit is calculated once; canonical fields are immutable.
Use `tx()` and `sidecar()` to inspect it, and `sidecar_mut()` to change only
transport data. Unwrap and reconstruct it to change canonical transaction
fields, so derived metadata cannot become stale.

## Requests and fees

`Transaction::frame_transaction()` exposes borrowed canonical frame data.
Generic request conversions preserve frames and signatures through this hook.
As before, `TransactionRequest::from_transaction()` leaves `from` unset;
use the sender-aware helper when rebuilding.

Frame request `fees` is a serializable, full-width fallback. Explicit top-level
fee fields take precedence, so ordinary fee edits work. Conversion populates
top-level fields only when they fit in `u128`; it never clamps a wider value.
Both JSON and the bincode compatibility adapter preserve the fallback.
Changing a fee invalidates any existing authorization covering that fee.

Legacy type inference and generic `EthereumWallet` support are retained.
Custom networks can override `Network::try_into_presigned` to support their
own self-authorized formats.

## Release prerequisite

`alloy-eip8141` now has a version requirement and an immutable git revision.
The matching basic-types version must still be published before Alloy can
resolve its normalized crates.io package dependencies. Pinning a git revision
does not replace that publication step.
