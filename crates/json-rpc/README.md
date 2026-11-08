# alloy-json-rpc

Core types for JSON-RPC 2.0 clients.

This crate includes data types and traits for JSON-RPC 2.0 requests and
responses, targeted at RPC client usage.

## Core model

A JSON-RPC call contains a method, an ID, and optional parameters; protocol
notifications omit the ID. Alloy's client-oriented `Request` always serializes
an `id`, with `Id::None` encoded as JSON `null`. Any type that implements
[`RpcSend`] may be used as parameters. Parameters are omitted when their Rust
type is zero-sized, such as `()` or `[(); 0]`; an empty `Vec` is still serialized
as `[]`.

Responses contain either a success value or an [`ErrorPayload`]. Client code
usually handles them through [`RpcResult<T, E>`], an alias for
`Result<T, RpcError<E>>`:

- `Ok(T)` means the server returned a successful result.
- `Err(RpcError::ErrorResp(_))` means the server returned a JSON-RPC error.
- `Err(RpcError::NullResp)` means a method that requires a value received JSON
  `null`.
- Other variants include serialization, deserialization, local usage,
  unsupported-feature, and transport failures.

The borrowed response aliases, such as [`BorrowedResponse`] and
[`BorrowedResponsePacket`], support inspecting payloads without first copying
them. The client-oriented [`RpcRecv`] trait requires owned response types.

Most applications should use [`alloy-rpc-client`] or [`alloy-provider`]
instead of constructing these protocol types directly.

[`alloy-provider`]: https://docs.rs/alloy-provider/
[`alloy-rpc-client`]: https://docs.rs/alloy-rpc-client/
[`BorrowedResponse`]: https://docs.rs/alloy-json-rpc/latest/alloy_json_rpc/type.BorrowedResponse.html
[`BorrowedResponsePacket`]: https://docs.rs/alloy-json-rpc/latest/alloy_json_rpc/type.BorrowedResponsePacket.html
[`ErrorPayload`]: https://docs.rs/alloy-json-rpc/latest/alloy_json_rpc/struct.ErrorPayload.html
[`RpcRecv`]: https://docs.rs/alloy-json-rpc/latest/alloy_json_rpc/trait.RpcRecv.html
[`RpcResult<T, E>`]: https://docs.rs/alloy-json-rpc/latest/alloy_json_rpc/type.RpcResult.html
[`RpcSend`]: https://docs.rs/alloy-json-rpc/latest/alloy_json_rpc/trait.RpcSend.html
