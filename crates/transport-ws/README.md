# alloy-transport-ws

WebSocket transport implementation for Ethereum JSON-RPC communication.

This crate provides WebSocket-based transport for the Alloy Ethereum toolkit, enabling real-time communication with Ethereum nodes via JSON-RPC over WebSocket protocol. It supports both native (non-WASM) and WebAssembly environments.

## Features

- **Cross-platform support**: Works on native platforms and in WebAssembly
- **PubSub support**: Enables subscription to real-time blockchain events via [alloy-pubsub]
- **TLS support**: Secure WebSocket connections with rustls on native platforms
- **Async/await**: Built on top of tokio for efficient async operations

## Usage

This crate is typically used as part of the [alloy-provider] ecosystem. For direct usage:

```rust
use alloy_transport_ws::WsConnect;

// Connect to a WebSocket endpoint
let ws_connect = WsConnect::new("wss://mainnet.infura.io/ws/v3/YOUR_PROJECT_ID");
```

## Related Crates

- [alloy-transport]: Core transport abstraction
- [alloy-transport-http]: HTTP-based transport
- [alloy-transport-ipc]: IPC-based transport
- [alloy-pubsub]: PubSub functionality for real-time events
- [alloy-provider]: High-level Ethereum provider API

[alloy-transport]: https://docs.rs/alloy_transport/
[alloy-transport-http]: https://docs.rs/alloy_transport_http/
[alloy-transport-ipc]: https://docs.rs/alloy_transport_ipc/
[alloy-pubsub]: https://docs.rs/alloy_pubsub/
[alloy-provider]: https://docs.rs/alloy_provider/
