#![cfg(feature = "ws")]

use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::WsConnect;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_sub_new_heads_fast() {
    let anvil = Anvil::default().spawn();

    let provider = ProviderBuilder::new().on_ws(WsConnect::new(anvil.ws_endpoint())).await.unwrap();

    let mut blocks = provider.subscribe_blocks().await.unwrap();

    let num = 5;

    provider
        .client()
        .request::<_, ()>("anvil_mine", vec![alloy_primitives::U256::from(num)])
        .await
        .unwrap();

    let mut count = 0;
    while let Ok(_block) = blocks.recv_any().await {
        count += 1;
    }
    assert_eq!(count, num);
}
