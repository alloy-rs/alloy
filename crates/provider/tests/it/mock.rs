use alloy_network::{AnyNetwork, AnyRpcLog, AnyRpcTransaction};
use alloy_primitives::{bytes, Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::{Filter, FilterChanges, Log, TransactionRequest};
use alloy_serde::WithOtherFields;
use alloy_transport::mock::Asserter;
use futures::StreamExt;

#[tokio::test]
async fn mocked_default_provider() {
    let asserter = Asserter::new();
    let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

    asserter.push_success(&21965802);
    asserter.push_success(&21965803);
    asserter.push_failure_msg("mock test");

    let response = provider.get_block_number().await.unwrap();
    assert_eq!(response, 21965802);

    let response = provider.get_block_number().await.unwrap();
    assert_eq!(response, 21965803);

    let response = provider.get_block_number().await.unwrap_err();
    assert!(response.to_string().contains("mock test"), "{response}");

    let response = provider.get_block_number().await.unwrap_err();
    assert!(response.to_string().contains("empty asserter response queue"), "{response}");
    assert!(response.to_string().contains("eth_blockNumber"), "{response}");
    assert!(response.to_string().contains("3"), "{response}");

    let accounts = [Address::with_last_byte(1), Address::with_last_byte(2)];
    asserter.push_success(&accounts);
    let response = provider.get_accounts().await.unwrap();
    assert_eq!(response, accounts);

    let call_resp = bytes!("12345678");
    asserter.push_success(&call_resp);
    let tx = TransactionRequest::default();
    let response = provider.call(tx).await.unwrap();
    assert_eq!(response, call_resp);

    let assert_bal = U256::from(123456780);
    asserter.push_success(&assert_bal);
    let response = provider.get_balance(Address::default()).await.unwrap();
    assert_eq!(response, assert_bal);
}

#[tokio::test]
async fn mocked_any_network_preserves_log_fields() {
    let asserter = Asserter::new();
    let provider =
        ProviderBuilder::new().network::<AnyNetwork>().connect_mocked_client(asserter.clone());
    let mut log = WithOtherFields::new(Log::default());
    log.other.insert("blockTimestampMs".to_owned(), serde_json::json!("0xa4d8"));

    asserter.push_success(&vec![log.clone()]);
    let logs = provider.get_logs(&Filter::new()).await.unwrap();
    assert_eq!(logs[0].other.get("blockTimestampMs"), log.other.get("blockTimestampMs"));

    let changes = FilterChanges::<AnyRpcTransaction, AnyRpcLog>::Logs(vec![log.clone()]);
    asserter.push_success(&changes);
    let changes = provider.get_filter_changes_dyn(U256::from(1)).await.unwrap();
    assert_eq!(
        changes.as_logs().unwrap()[0].other.get("blockTimestampMs"),
        log.other.get("blockTimestampMs")
    );

    asserter.push_success(&U256::from(1));
    asserter.push_success(&vec![log.clone()]);
    let logs = provider
        .watch_logs(&Filter::new())
        .await
        .unwrap()
        .with_limit(Some(1))
        .into_stream()
        .next()
        .await
        .unwrap();
    assert_eq!(logs[0].other.get("blockTimestampMs"), log.other.get("blockTimestampMs"));
}
