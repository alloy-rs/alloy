#![cfg(feature = "ws")]

use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::WsConnect;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_sub_new_heads_fast() {
    // init tracing
    let _ = tracing_subscriber::fmt::try_init();

    let anvil = Anvil::default().spawn();

    let provider = ProviderBuilder::new().on_ws(WsConnect::new(anvil.ws_endpoint())).await.unwrap();

    let mut blocks = provider.subscribe_blocks().await.unwrap();
    tracing::info!(id = %blocks.local_id(), "id");

    let num = 200;

    provider
        .client()
        .request::<_, ()>("anvil_mine", vec![alloy_primitives::U256::from(num)])
        .await
        .unwrap();

    tracing::info!("This is never reached");

    let mut count = 0;
    loop {
        let block = blocks.recv_any().await;
        if block.is_err() {
            tracing::error!("lagged");
        }
        count += 1;
        tracing::info!(count, "got");
        if count >= num {
            break;
        }
    }
    assert_eq!(count, num);
}
