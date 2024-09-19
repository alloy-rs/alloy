use alloy_node_bindings::{utils::run_with_tempdir, Geth};
use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ipc::IpcConnect;

#[tokio::test]
async fn can_make_a_request() {
    run_with_tempdir("geth-test-", |temp_dir| async move {
        let geth = Geth::new()
            .disable_discovery()
            .ipc_path(temp_dir.join("alloy.ipc"))
            .enable_ipc()
            .block_time(1u64)
            .data_dir(temp_dir)
            .spawn();

        let connect = IpcConnect::new(geth.ipc_endpoint());
        let client = ClientBuilder::default().pubsub(connect).await.unwrap();

        let req: RpcCall<_, _, U64> = client.request_noparams("eth_blockNumber");
        let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);
        let res = timeout.await.unwrap().unwrap();
        assert!(res.to::<u64>() <= 3);
    })
    .await;
}
