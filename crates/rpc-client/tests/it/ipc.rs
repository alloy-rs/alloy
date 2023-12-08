use std::borrow::Cow;

use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ipc::{IpcConnect, MockIpcServer};

#[test_log::test(tokio::test)]
async fn it_makes_a_request() {
    let mut server = MockIpcServer::new();

    server.add_reply("{\"jsonrpc\": \"2.0\",\"id\": 1,\"result\": \"0x0\"}");

    let path = server.path();

    let _ = server.spawn().await;

    let connector: IpcConnect<_> = path.into();

    let client = ClientBuilder::default().pubsub(connector).await.unwrap();

    let params: Cow<'static, _> = Cow::Owned(vec![]);

    let req: RpcCall<_, Cow<'static, Vec<String>>, U64> = client.prepare("eth_blockNumber", params);

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);

    timeout.await.unwrap().unwrap();
}
