# alloy-transports

<!-- TODO: More links and real doctests -->

Transport implementations for Alloy providers.

This crate handles RPC connection and request management. It builds an
`RpcClient` on top of the [tower `Service`] abstraction, and provides
futures for simple and batch RPC requests as well as a unified `TransportError`
type.

[alloy-providers]: ../providers/
[tower `Service`]: https://docs.rs/tower/latest/tower/trait.Service.html

## Usage

Usage of this crate typically means instantiating an `RpcClient<T>` over some
`Transport`. The RPC client can then be used to make requests to the RPC
server. Requests are captured as `RpcCall` futures, which can be polled to
completion.

For example, to make a simple request:

```rust,ignore
// Instantiate a new client over a transport.
let client: RpcClient<reqwest::Http> = "https://mainnet.infura.io/v3/...".parse().unwrap();

// Prepare a request to the server.
let request = client.request("eth_blockNumber", ());

// Poll the request to completion.
let block_number = request.await.unwrap();
```

Batch requests are also supported:

```rust,ignore
// Instantiate a new client over a transport.
let client: RpcClient<reqwest::Http> = "https://mainnet.infura.io/v3/...".parse().unwrap();

// Prepare a batch request to the server.
let batch = client.new_batch();

// Batches serialize params immediately. So we need to handle the result when
// adding calls.
let block_number_fut = batch.add_call("eth_blockNumber", ()).unwrap();
let balance_fut = batch.add_call("eth_getBalance", address).unwrap();

// Make sure to send the batch!
batch.send().await.unwrap();

// After the batch is complete, we can get the results.
// Note that requests may error separately!
let block_number = block_number_fut.await.unwrap();
let balance = balance_fut.await.unwrap();
```

### Features

- `reqwest`: Enables the `reqwest` transport implementation.
- `hyper`: Enables the `hyper` transport implementation (not available in WASM).
