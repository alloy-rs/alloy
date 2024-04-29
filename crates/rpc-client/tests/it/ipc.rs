use alloy_node_bindings::{Geth, GethInstance};
use alloy_primitives::U64;
use alloy_pubsub::PubSubFrontend;
use alloy_rpc_client::{ClientBuilder, RpcCall, RpcClient};
use alloy_transport_ipc::IpcConnect;
use tempfile::TempDir;

async fn connect() -> (RpcClient<PubSubFrontend>, GethInstance) {
    let temp_dir = TempDir::with_prefix("geth-test-").unwrap();
    let geth = Geth::new()
        .disable_discovery()
        .data_dir(temp_dir.path())
        .enable_ipc()
        .block_time(1u64)
        .spawn();
    let path = temp_dir.path().join("geth.ipc");
    let connector: IpcConnect<_> = path.into();

    let client = ClientBuilder::default().pubsub(connector).await.unwrap();

    (client, geth)
}

#[tokio::test]
async fn it_makes_a_request() {
    let (client, _geth) = connect().await;
    let req: RpcCall<_, (), U64> = client.request("eth_blockNumber", ());
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
    let res = timeout.await.unwrap().unwrap();
    assert!(res.to::<u64>() <= 3);
}
