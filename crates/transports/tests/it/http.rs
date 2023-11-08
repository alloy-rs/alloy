use std::borrow::Cow;

use alloy_primitives::U64;
use alloy_transports::{ClientBuilder, RpcCall};

#[tokio::test]
async fn it_makes_a_request() {
    let infura = std::env::var("INFURA").unwrap();

    let client = ClientBuilder::default().reqwest_http(infura.parse().unwrap());

    let params: Cow<'static, _> = Cow::Owned(());

    let req: RpcCall<_, Cow<'static, ()>, U64> = client.prepare("eth_blockNumber", params);
    let res = req.await;
    res.unwrap();
}
