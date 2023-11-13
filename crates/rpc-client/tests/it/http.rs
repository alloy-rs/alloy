use alloy_primitives::U64;
use alloy_rpc_client::{ClientBuilder, RpcCall};
use std::borrow::Cow;

// #[tokio::test]
async fn it_makes_a_request() {
    let infura = std::env::var("HTTP_PROVIDER_URL").unwrap();

    let client = ClientBuilder::default().reqwest_http(infura.parse().unwrap());

    let params: Cow<'static, _> = Cow::Owned(());

    let req: RpcCall<_, Cow<'static, ()>, U64> = client.prepare("eth_blockNumber", params);
    req.await.unwrap();
}
