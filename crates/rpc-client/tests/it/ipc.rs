use alloy_node_bindings::{Geth, GethInstance};
use alloy_primitives::U64;
use alloy_pubsub::PubSubFrontend;
use alloy_rpc_client::{ClientBuilder, RpcCall, RpcClient};
use alloy_transport_ipc::IpcConnect;
use std::borrow::Cow;
use tempfile::NamedTempFile;

async fn connect() -> (RpcClient<PubSubFrontend>, GethInstance) {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.into_temp_path().to_path_buf();
    let geth = Geth::new().block_time(1u64).ipc_path(&path).spawn();

    // [Windows named pipes](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes)
    // are located at `\\<machine_address>\pipe\<pipe_name>`.
    #[cfg(windows)]
    let path = format!(r"\\.\pipe\{}", path.display());

    let connector: IpcConnect<_> = path.into();

    let client = ClientBuilder::default().pubsub(connector).await.unwrap();

    (client, geth)
}

#[test_log::test(tokio::test)]
async fn it_makes_a_request() {
    let (client, _geth) = connect().await;

    let params: Cow<'static, _> = Cow::Owned(vec![]);

    let req: RpcCall<_, Cow<'static, Vec<String>>, U64> = client.prepare("eth_blockNumber", params);

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);

    timeout.await.unwrap().unwrap();
}
