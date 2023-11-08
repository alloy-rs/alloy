use alloy_transports::{ClientBuilder, RpcCall, WsConnect};

use alloy_primitives::U64;
use std::borrow::Cow;

#[test_log::test(tokio::test)]
async fn it_makes_a_request() {
    let infura = std::env::var("WS_PROVIDER_URL").unwrap();

    let connector = WsConnect {
        url: infura.parse().unwrap(),
        auth: None,
    };

    let client = ClientBuilder::default().connect(connector).await.unwrap();

    let params: Cow<'static, _> = Cow::Owned(());

    let req: RpcCall<_, Cow<'static, ()>, U64> = client.prepare("eth_blockNumber", params);

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), req);

    timeout.await.unwrap().unwrap();
}
