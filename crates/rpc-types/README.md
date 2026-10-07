# alloy-rpc-types

Meta-crate for all Ethereum JSON-RPC types.

This crate provides feature-gated access to the individual RPC type crates.
The default feature set enables `eth`; Ethereum types are available both under
`alloy_rpc_types::eth` and directly at the crate root. Other modules must be
enabled explicitly.

## Features

| Feature | Exposed module | Types |
| --- | --- | --- |
| `admin` | `alloy_rpc_types::admin` | Node administration |
| `anvil` | `alloy_rpc_types::anvil` | Anvil development node |
| `any` | `alloy_rpc_types::any` | Network-independent RPC types |
| `beacon` | `alloy_rpc_types::beacon` | Beacon Node API |
| `debug` | `alloy_rpc_types::debug` | Debug RPC |
| `engine` | `alloy_rpc_types::engine` | Execution Engine API |
| `eth` (default) | `alloy_rpc_types::eth` and the crate root | Ethereum RPC |
| `mev` | `alloy_rpc_types::mev` | MEV RPC |
| `trace` | `alloy_rpc_types::trace` | Trace RPC |
| `txpool` | `alloy_rpc_types::txpool` | Transaction pool RPC |

The `arbitrary`, `ssz`, `k256`, and `kzg` features forward support to the relevant
dependencies without enabling additional modules. The `default` feature also
forwards the default features of the `eth` and `engine` type crates when they
are enabled.

## RPC modules

`RpcModules` is always available, including when default features are disabled.
It represents the `rpc_modules` response as a map from module names to versions
and serializes as a JSON object, for example `{"eth":"1.0","net":"1.0"}`.

Use `modules()` to inspect the map by reference or `into_modules()` to take
ownership of it:

```rust
use alloy_primitives::map::HashMap;
use alloy_rpc_types::RpcModules;

let response = RpcModules::new(HashMap::from_iter([("eth".to_owned(), "1.0".to_owned())]));
assert_eq!(response.modules().get("eth").map(String::as_str), Some("1.0"));

let modules = response.into_modules();
assert_eq!(modules.get("eth").map(String::as_str), Some("1.0"));
```
