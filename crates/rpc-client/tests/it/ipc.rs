use alloy_node_bindings::Geth;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ipc::IpcConnect;

#[tokio::test]
async fn it_makes_a_request() {
    let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
    let geth = Geth::new()
        .disable_discovery()
        .ipc_path(temp_dir.path().join("alloy.ipc"))
        .enable_ipc()
        .block_time(1u64)
        .data_dir(temp_dir.path())
        .spawn();

    let connect = IpcConnect::new(geth.ipc_endpoint());
    let client = ClientBuilder::default().pubsub(connect).await.unwrap();

    let req: RpcCall<_, _, U64> = client.request_noparams("eth_blockNumber");
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
    let res = timeout.await.unwrap().unwrap();
    assert!(res.to::<u64>() <= 3);
}
