use alloy_node_bindings::Anvil;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ws::WsConnect;
use similar_asserts::assert_eq;

#[tokio::test]
async fn it_makes_a_request() {
    let anvil = Anvil::new().spawn();
    let url = anvil.ws_endpoint();
    let connector = WsConnect::new(url);
    let client = ClientBuilder::default().pubsub(connector).await.unwrap();
    let req: RpcCall<_, _, U64> = client.request_noparams("eth_blockNumber");
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
    let res = timeout.await.unwrap().unwrap();
    assert_eq!(res.to::<u64>(), 0);
}

// <https://github.com/alloy-rs/alloy/issues/4085>
#[tokio::test]
async fn it_makes_a_batch_request() {
    let anvil = Anvil::new().spawn();
    let url = anvil.ws_endpoint();
    let connector = WsConnect::new(url);
    let client = ClientBuilder::default().pubsub(connector).await.unwrap();

    let mut batch = client.new_batch();
    let block_number = batch.add_call::<_, U64>("eth_blockNumber", &()).unwrap();
    let chain_id = batch.add_call::<_, U64>("eth_chainId", &()).unwrap();
    batch.send().await.unwrap();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        (block_number.await, chain_id.await)
    });
    let (block_number, chain_id) = timeout.await.unwrap();
    assert_eq!(block_number.unwrap().to::<u64>(), 0);
    assert_eq!(chain_id.unwrap().to::<u64>(), 31337);
}
