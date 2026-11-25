# alloy-rpc-client

Low-level Ethereum JSON-RPC client implementation.

## Usage

Usage of this crate typically means instantiating an `RpcClient` over some
`Transport`. The RPC client can then be used to make requests to the RPC
server. Requests are captured as `RpcCall` futures, which can then be polled to
completion.

For example, to make a simple request:

```rust,ignore
// Instantiate a new client over a transport.
let client: ReqwestClient = ClientBuilder::default().http(url);

// Prepare a request to the server.
let request = client.request_noparams("eth_blockNumber");

// Poll the request to completion.
let block_number: U64 = request.await.unwrap();
```

Batch requests are also supported. This example uses the default `reqwest` feature:

```rust,no_run
use alloy_primitives::{address, U256, U64};
use alloy_rpc_client::ClientBuilder;

# #[cfg(all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))))]
# async fn example() {
// Instantiate a new client over a transport.
let url = "http://localhost:8545".parse().unwrap();
let client = ClientBuilder::default().http(url);

// Prepare a batch request to the server.
let mut batch = client.new_batch();

// Batches serialize params immediately. So we need to handle the result when
// adding calls.
let block_number_fut = batch.add_call("eth_blockNumber", &()).unwrap();
let address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
// eth_getBalance expects positional parameters: the address and a block selector.
let balance_fut = batch.add_call("eth_getBalance", &(address, "latest")).unwrap();

// Make sure to send the batch!
batch.send().await.unwrap();

// After the batch is complete, we can get the results.
// Note that requests may error separately!
let block_number: U64 = block_number_fut.await.unwrap();
let balance: U256 = balance_fut.await.unwrap();
# }
```
