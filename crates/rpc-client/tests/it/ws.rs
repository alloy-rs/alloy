use alloy_node_bindings::Anvil;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ws::WsConnect;

#[tokio::test]
async fn it_makes_a_request() {
    let anvil = Anvil::new().spawn();
    let url = anvil.ws_endpoint();
    let connector = WsConnect { url: url.parse().unwrap(), auth: None };
    let client = ClientBuilder::default().pubsub(connector).await.unwrap();
    let req: RpcCall<_, _, U64> = client.request_noparams("eth_blockNumber");
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
    let res = timeout.await.unwrap().unwrap();
    assert_eq!(res.to::<u64>(), 0);
}
