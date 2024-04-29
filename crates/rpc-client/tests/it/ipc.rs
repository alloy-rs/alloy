use alloy_node_bindings::Geth;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ipc::IpcConnect;

#[tokio::test]
async fn it_makes_a_request() {
    let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
    let _geth = Geth::new()
        .disable_discovery()
        .enable_ipc()
        .block_time(1u64)
        .data_dir(temp_dir.path())
        .spawn();
    let ipc_path = temp_dir.into_path().join("geth.ipc");

    let connector: IpcConnect<_> = ipc_path.into();
    let client = ClientBuilder::default().pubsub(connector).await.unwrap();

    let req: RpcCall<_, (), U64> = client.request("eth_blockNumber", ());
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
    let res = timeout.await.unwrap().unwrap();
    assert!(res.to::<u64>() <= 3);
}
