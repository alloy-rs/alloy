use std::borrow::Cow;

use alloy_primitives::U64;
use alloy_transports::{ClientBuilder, RpcCall, WsConnect};

#[tokio::test]
async fn it_makes_a_request() {
    let infura = std::env::var("INFURA_WS").unwrap();

    let connector = WsConnect {
        url: infura.parse().unwrap(),
        auth: None,
    };

    let client = ClientBuilder::default().connect(connector).await.unwrap();

    let params: Cow<'static, _> = Cow::Owned(());

    let req: RpcCall<_, Cow<'static, ()>, U64> = client.prepare("eth_blockNumber", params);
    let res = req.await;

    dbg!(&res);
    res.unwrap();
}
