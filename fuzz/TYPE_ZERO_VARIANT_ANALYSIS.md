# EIP-2718 explicit type-zero variant analysis

Date: 2026-08-31

## Summary

The original transaction bug exists because an explicit EIP-2718 type byte of `0x00` reaches a
typed decoder even though Alloy reserves type zero for the untagged legacy fallback. The decoded
value reports type zero, and `Encodable2718::type_flag` consequently omits the byte on re-encoding.

The sweep confirmed two production receipt variants with the same root cause:

1. `AnyReceiptEnvelope::typed_decode` accepted type zero directly.
2. `EthereumReceipt<T>::typed_decode_with_bloom`, reached through
   `ReceiptWithBloom<EthereumReceipt<T>>`, accepted type zero directly.

Both are parser-canonicality findings with low security severity and high confidence. No existing
Alloy issue or pull request was found for either receipt variant.

## Original issue

`Signed<T>::typed_decode(0, legacy_rlp)` accepted the explicit typed form because the decoded
legacy transaction also returned `ty() == 0`. Re-encoding omitted the byte. Alloy PR
[#4167](https://github.com/alloy-rs/alloy/pull/4167) adds the missing rejection.

## Search methodology

| Version | Pattern | Tool | Matches | Result |
| --- | --- | --- | ---: | --- |
| v0 | `decoded.ty() != ty` in `Signed<T>::typed_decode` | ripgrep/history | 1 | Calibrated the known transaction bug |
| v1 | `Self::typed_decode(0, buf)` | ripgrep | 1 | Confirmed `AnyReceiptEnvelope` variant |
| v2 | every `fn typed_decode(` implementation | ripgrep | 10 textual matches | 1 trait declaration, 2 vulnerable implementations before fixes, 1 explicit safe implementation, 6 delegating/generated implementations |
| v3 | every `fn typed_decode_with_bloom(` implementation | ripgrep | 3 textual matches | 1 trait declaration, 1 production variant, 1 test-only model |
| v4 | `T::try_from(0)` and all zero-valued envelope variants | ripgrep/call tracing | 3 relevant paths | Untagged fallbacks are valid; typed entry points require a separate zero rejection |

Searches covered the entire `crates/` tree. Generalization stopped at all `Decodable2718`
implementations because wrappers beyond that point delegate to an already-triaged inner decoder.

## Confirmed variants

### `AnyReceiptEnvelope` — low severity, high confidence

The type stored `r#type: 0` when called through `typed_decode(0, ...)`. Its encoder treats that value
as legacy and emits no prefix. The fix rejects zero in `typed_decode` and decodes the untagged
legacy form directly in `fallback_decode`.

### `ReceiptWithBloom<EthereumReceipt<T>>` — low severity, high confidence

`EthereumReceipt<T>::typed_decode_with_bloom` converted zero into the legacy transaction type and
decoded the receipt. `eip2718_encode_with_bloom` omits the prefix for that value. The fix rejects
zero in the typed entry point while retaining `T::try_from(0)` in the untagged fallback.

## Ruled-out and lower-priority candidates

- `ReceiptEnvelope` already returns `UnexpectedType(0)` from typed decoding.
- `Sealed<T>`, `Either<L, R>`, `Extended<B, T>`, `ReceiptWithBloom<R>`, and `AnyTxEnvelope` delegate
  to their inner decoder; the concrete production inners are protected by the fixes above.
- The `TransactionEnvelope` derive still dispatches zero to its declared legacy variant, but every
  current production transaction envelope reaches the protected `Signed<T>` decoder. Adding a
  macro-level rejection would be reasonable defense in depth for third-party inner types, but no
  additional in-tree exploitable instance was found.
- `proofs.rs` contains a test-only receipt model that accepts typed zero. It is not production code;
  production regression tests and fuzz seeds now state the stricter invariant.
- Unknown JSON transaction types cannot be binary re-encoded and therefore do not create this
  normalization path.

## Regression guard

The consensus RLP target now has no type-zero exemption. Generated corpus entries cover direct and
RLP-network-wrapped tagged-zero transactions, pooled transactions, `AnyReceiptEnvelope`, and
`ReceiptWithBloom<EthereumReceipt>`. CI can replay them with:

```sh
cargo +nightly fuzz run --sanitizer none consensus_rlp fuzz/corpus/consensus_rlp -- -runs=0
```

