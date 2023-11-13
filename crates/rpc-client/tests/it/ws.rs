use alloy_rpc_client::{ClientBuilder, RpcCall};
use alloy_transport_ws::WsConnect;

use alloy_primitives::U64;
use std::borrow::Cow;

// #[test_log::test(tokio::test)]
async fn it_makes_a_request() {
    let infura = std::env::var("WS_PROVIDER_URL").unwrap();

    let connector = WsConnect { url: infura.parse().unwrap(), auth: None };

    let client = ClientBuilder::default().pubsub(connector).await.unwrap();

    let params: Cow<'static, _> = Cow::Owned(());

    let req: RpcCall<_, Cow<'static, ()>, U64> = client.prepare("eth_blockNumber", params);

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);

    timeout.await.unwrap().unwrap();
}
