# alloy-serde

Serde helpers for Ethereum JSON-RPC formats that differ from Serde's defaults.

- [`quantity`] encodes primitive integers as canonical RPC quantities such as
  `"0x2a"`. Its `opt`, `vec`, `hashmap`, and `btreemap` modules cover common containers.
- [`ttd`] handles geth's mixed number/string representation of terminal total
  difficulty.
- [`storage`] handles storage keys and zero-padding cropped storage values.
- [`WithOtherFields`] preserves unknown object fields for forwarding or
  round-tripping RPC payloads.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Request {
    #[serde(with = "alloy_serde::quantity")]
    block: u64,
    // `default` is required if an omitted field should become `None`.
    #[serde(default, with = "alloy_serde::quantity::opt")]
    limit: Option<u64>,
}

let request: Request = serde_json::from_str(r#"{"block":"0x2a"}"#)?;
assert_eq!(request, Request { block: 42, limit: None });
assert_eq!(
    serde_json::to_string(&Request { block: 42, limit: Some(16) })?,
    r#"{"block":"0x2a","limit":"0x10"}"#,
);
# Ok::<(), serde_json::Error>(())
```

For deserialization-only helpers such as [`null_as_default`], use
`deserialize_with` and pair it with `#[serde(default)]` when missing fields should also use the
default. The no-prefix hex helpers are serialization-only and therefore use `serialize_with`.

[`null_as_default`]: https://docs.rs/alloy-serde/latest/alloy_serde/fn.null_as_default.html
[`quantity`]: https://docs.rs/alloy-serde/latest/alloy_serde/quantity/index.html
[`storage`]: https://docs.rs/alloy-serde/latest/alloy_serde/storage/index.html
[`ttd`]: https://docs.rs/alloy-serde/latest/alloy_serde/ttd/index.html
[`WithOtherFields`]: https://docs.rs/alloy-serde/latest/alloy_serde/struct.WithOtherFields.html
