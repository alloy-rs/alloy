use alloy_network::{ReceiptResponse, TransactionBuilder};
use alloy_node_bindings::Anvil;
use alloy_primitives::U256;
use alloy_provider::{ext::AnvilApi, Provider, ProviderBuilder, WalletProvider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_eth::TransactionRequest;
use std::time::Duration;

#[tokio::test]
async fn get_receipt_with_required_confirmations_returns_confirmed_receipt() {
    let anvil = Anvil::new().block_time(1).spawn();
    let wallet = anvil.wallet().expect("dev wallet");

    let client = RpcClient::builder()
        .connect(&anvil.endpoint())
        .await
        .expect("connect anvil")
        .with_poll_interval(Duration::from_millis(100));

    let provider = ProviderBuilder::new().wallet(wallet).connect_client(client);

    provider.anvil_set_auto_mine(false).await.expect("disable automine");

    for attempt in 1..=50 {
        let tx = TransactionRequest::default()
            .with_to(provider.default_signer_address())
            .with_value(U256::from(1_u64));

        let pending = provider.send_transaction(tx).await.expect("send transaction");
        let tx_hash = *pending.tx_hash();

        let watcher = tokio::spawn(async move {
            pending
                .with_required_confirmations(3)
                .with_timeout(Some(Duration::from_secs(10)))
                .get_receipt()
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        provider.anvil_mine(Some(3), None).await.expect("mine confirmations");

        let receipt = watcher.await.expect("watch task panicked").unwrap_or_else(|err| {
            panic!(
                "get_receipt timed out before returning existing confirmed receipt on attempt \
                 {attempt}: {err}"
            )
        });
        let direct_receipt =
            provider.get_transaction_receipt(tx_hash).await.expect("fetch direct receipt");
        let latest_block = provider.get_block_number().await.expect("fetch latest block");

        let receipt_block = receipt.block_number().expect("receipt has block number");
        assert_eq!(receipt.transaction_hash(), tx_hash);
        assert!(receipt.status());
        assert!(latest_block >= receipt_block + 2);
        assert!(direct_receipt.is_some());
    }
}
