# alloy-tx-macros

Derive macro for building EIP-2718 transaction envelope enums.

This crate provides the `TransactionEnvelope` derive macro, which generates an envelope over multiple transaction types following EIP-2718. Crate-level docs are sourced from this README.

## When to use

- **Model a family of transactions** under a single enum that implements the consensus traits used across Alloy.
- **Support multiple EIP-2718 types** via variant `ty = N` tags, and optionally delegate to other envelopes via `flatten`.

## Requirements

- The derive can be applied to **enums only**.
- Every variant must have **exactly one unnamed field** (tuple variant with a single element).
- Each variant must be annotated with `#[envelope(...)]` specifying either a concrete type tag or `flatten`.

## Container attributes

Apply on the enum with `#[envelope(...)]`:

- `tx_type_name = Ident`
  - Custom name for the generated transaction type enum.
  - Defaults to `{EnumName}Type`.

- `alloy_consensus = path::to::alloy_consensus`
  - Custom path to the `alloy-consensus` crate.
  - Defaults to `::alloy_consensus`.

- `typed = Ident`
  - If set, also generates a corresponding `TypedTransaction` enum with the given name.

- `serde_cfg = (..meta list..)`, `arbitrary_cfg = (..meta list..)`
  - Custom `cfg_attr` lists that gate serde/arbitrary implementations.
  - Must be specified as list-form metas, e.g. `serde_cfg(feature = "serde")`.
  - Defaults to `all()`.

## Variant attributes

Apply on each variant with `#[envelope(...)]`:

- `ty = N`
  - Declare the EIP-2718 transaction type ID (0–255) for this variant.

- `flatten`
  - Flatten this variant to delegate to the inner envelope type.

Optional per-variant:

- `typed = Ident`
  - Custom typed transaction mapping for this variant when a container-level `typed` enum is requested.

Forwarded attributes on variants:

- `#[serde(...)]` and `#[doc(...)]` are forwarded and respected by the generated code.

## Generated items

Given an enum `MyEnvelope`, the macro generates:

- `MyEnvelopeType` (or custom `tx_type_name`): enum of transaction type tags.
- Implementations of:
  - `Transaction`
  - `Typed2718`
  - `Encodable2718`
  - `Decodable2718`
- If the `serde` feature is enabled, serde `Serialize`/`Deserialize` support is generated and can be gated via `serde_cfg`.
- If the `arbitrary` feature is enabled, `arbitrary` support is generated and can be gated via `arbitrary_cfg`.
- If `typed = Ident` is provided at the container level, a `TypedTransaction` enum is generated mapping each variant to its unsigned type.
  - For inner types named `Signed<T>` or `Sealed<T>`, the unsigned `T` is automatically extracted for the typed mapping; otherwise the inner type is used as-is.

### Serde tagging

For typed variants (`ty = N`), serde uses a lowercase hexadecimal tag string of the form `"0x{hex}"` and provides aliases:

- Single-digit values accept both `"0xN"` and the zero-padded `"0x0N"`.
- Uppercase hex is accepted as an alias (e.g., `"0x7E"`).

## Examples

Minimal container and variants:

```rust,ignore
use alloy_tx_macros::TransactionEnvelope;

#[derive(TransactionEnvelope)]
#[envelope(tx_type_name = MyTxType)]
enum MyEnvelope {
    // A typed transaction with EIP-2718 type 0 (legacy)
    #[envelope(ty = 0)]
    Legacy(MyLegacyTx),

    // A typed transaction with EIP-2718 type 2
    #[envelope(ty = 2)]
    Eip1559(MyEip1559Tx),
}
```

Generating a typed transaction enum and customizing per-variant type mapping:

```rust,ignore
use alloy_tx_macros::TransactionEnvelope;

#[derive(TransactionEnvelope)]
#[envelope(typed = MyTypedTx)]
enum MyEnvelope {
    // Uses inner type extraction rules (e.g., Signed<T> -> T)
    #[envelope(ty = 0)]
    Legacy(Signed<LegacyUnsigned>),

    // Overrides the typed mapping for this variant
    #[envelope(ty = 2, typed = AltUnsigned)]
    Eip1559(Signed<Eip1559Unsigned>),
}
```

Flattening to another envelope type:

```rust,ignore
use alloy_tx_macros::TransactionEnvelope;

#[derive(TransactionEnvelope)]
enum OuterEnvelope {
    #[envelope(flatten)]
    Inner(InnerEnvelope),
}
```

Conditionally gating serde and arbitrary implementations:

```rust,ignore
#[derive(TransactionEnvelope)]
#[envelope(serde_cfg(feature = "serde"), arbitrary_cfg(feature = "arbitrary"))]
enum MyEnvelope { /* ... */ }
```

Overriding the `alloy_consensus` path (for non-standard setups):

```rust,ignore
#[derive(TransactionEnvelope)]
#[envelope(alloy_consensus = path::to::alloy_consensus)]
enum MyEnvelope { /* ... */ }
```

## Feature flags

- `serde`
  - Enables serde serialization/deserialization support in the generated code.
- `arbitrary`
  - Enables `arbitrary` support in the generated code.

Use `serde_cfg` / `arbitrary_cfg` on the container to control when these impls are compiled via `cfg_attr`.

## Notes

- The macro relies on `alloy-consensus` and its internal modules; use a compatible version if overriding `alloy_consensus` path.

## Troubleshooting

- "`TransactionEnvelope` can only be derived for enums": apply the derive to an enum type.
- "TransactionEnvelope variants must have a single unnamed field": ensure each variant is a tuple with exactly one element.
- `serde_cfg` / `arbitrary_cfg` must be specified as list metas, e.g. `serde_cfg(feature = "serde")`; other forms are rejected.
