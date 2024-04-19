#![cfg(feature = "ws")]

use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::WsConnect;
use futures::StreamExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_sub_new_heads_fast() {
    let anvil = Anvil::default().spawn();

    let provider = ProviderBuilder::new().on_ws(WsConnect::new(anvil.ws_endpoint())).await.unwrap();

    let blocks = provider.subscribe_blocks().await.unwrap();

    let p = provider.clone();
    let num = 1_000u64; // WON'T WORK
    provider.client().request::<_, ()>("anvil_mine", vec![num]).await.unwrap();

    let mut blocks = blocks.into_stream();
    while let Some(block) = blocks.next().await {
        dbg!(block.header.number.unwrap());
    }

    // // collect all the blocks
    // let blocks = blocks.into_stream().take(num as usize).collect::<Vec<_>>().await;
    // let block_numbers = blocks.into_iter().map(|b| b.header.number.unwrap()).collect::<Vec<_>>();

    // let numbers = (1..=num).collect::<Vec<_>>();
    // assert_eq!(block_numbers, numbers);
}
