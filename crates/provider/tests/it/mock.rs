use alloy_primitives::{bytes, Address, U256, TxKind};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_transport::mock::Asserter;

#[tokio::test]
async fn mocked_default_provider() {
    let asserter = Asserter::new();
    let provider = ProviderBuilder::new().on_mocked_client(asserter.clone());

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
    
    let tx_hash = bytes!("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234");
    asserter.push_success(&tx_hash);
    let tx = TransactionRequest {
        from: Some(Address::with_last_byte(1)),
        to: Some(TxKind::Call(Address::with_last_byte(2))),
        gas: Some(21000),
        value: Some(U256::from(1_000_000_000_000u64)),
        ..Default::default()
    };
    let response = provider.send_transaction(tx).await.unwrap();
    let pending_hash = response.tx_hash(); 

    assert_eq!(format!("{:x}", pending_hash), format!("{:x}", tx_hash));

    let mock_gas_price = U256::from(20_000_000_000u64); 
    asserter.push_success(&mock_gas_price);
    let response = provider.get_gas_price().await.unwrap();
    assert_eq!(U256::from(response), mock_gas_price);

    let mock_chain_id = U256::from(1); 
    asserter.push_success(&mock_chain_id);
    let response = provider.get_chain_id().await.unwrap();
    assert_eq!(U256::from(response), mock_chain_id);

    let mock_nonce = U256::from(15);
    asserter.push_success(&mock_nonce);
    let response = provider.get_transaction_count(Address::with_last_byte(1)).await.unwrap();
    assert_eq!(U256::from(response), mock_nonce);

    asserter.push_failure_msg("gas estimation failed");
    let tx = TransactionRequest {
        from: Some(Address::with_last_byte(1)),
        to: Some(alloy_primitives::TxKind::Call(Address::with_last_byte(3))),
        value: Some(U256::from(500)),
        ..Default::default()
    };
    let err = provider.estimate_gas(tx).await.unwrap_err();
    assert!(err.to_string().contains("gas estimation failed"));
}
