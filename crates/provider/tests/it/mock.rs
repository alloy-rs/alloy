use alloy_primitives::{bytes, Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_transport::mock::Asserter;

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
