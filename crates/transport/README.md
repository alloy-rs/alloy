# alloy-transport

### Example

Send a JSON-RPC request through a mock transport using the `tower::Service` interface.

```rust
use alloy_transport::mock::{Asserter, MockTransport};
use alloy_json_rpc as j;
use tower::Service;

// Prepare a mock response and a serialized request
let asserter = Asserter::new();
asserter.push_success(&12345u64);

let req: j::SerializedRequest = j::Request::new("test_method", 1u64.into(), ())
    .try_into()
    .unwrap();
let packet = j::RequestPacket::from(req);

// Drive the service and assert the response
let mut transport = MockTransport::new(asserter.clone());
let resp = tokio::runtime::Runtime::new()
    .unwrap()
    .block_on(async move { transport.call(packet).await })
    .unwrap();

if let j::ResponsePacket::Single(r) = resp {
    let n: u64 = match r.payload {
        j::ResponsePayload::Success(val) => serde_json::from_str(val.get()).unwrap(),
        j::ResponsePayload::Failure(err) => panic!("unexpected error: {err}"),
    };
    assert_eq!(n, 12345);
}
```

Low-level Ethereum JSON-RPC transport abstraction.

This crate handles RPC connection and request management. It builds an
`RpcClient` on top of the [tower `Service`] abstraction, and provides
futures for simple and batch RPC requests as well as a unified `TransportError`
type.

Typically, this crate should not be used directly. Most EVM users will want to
use the [alloy-provider] crate, which provides a high-level API for interacting
with JSON-RPC servers that provide the standard Ethereum RPC endpoints, or the
[alloy-rpc-client] crate, which provides a low-level JSON-RPC API without the
specific Ethereum endpoints.

[alloy-provider]: https://docs.rs/alloy_provider/
[alloy-rpc-client]: https://docs.rs/alloy_rpc_client/
[tower `Service`]: https://docs.rs/tower/latest/tower/trait.Service.html

### Transports

Alloy maintains the following transports:

- [alloy-transport-http]: JSON-RPC via HTTP.
- [alloy-transport-ws]: JSON-RPC via Websocket, supports pubsub via [alloy-pubsub].
- [alloy-transport-ipc]: JSON-RPC via IPC, supports pubsub via [alloy-pubsub].

[alloy-transport-http]: https://docs.rs/alloy_transport_http/
[alloy-transport-ws]: https://docs.rs/alloy_transport_ws/
[alloy-transport-ipc]: https://docs.rs/alloy_transport_ipc/
[alloy-pubsub]: https://docs.rs/alloy_pubsub/
