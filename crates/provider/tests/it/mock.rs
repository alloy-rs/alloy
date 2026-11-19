use alloy_consensus::{transaction::SignerRecoverable, Transaction};
use alloy_primitives::{bytes, Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
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

/// A fully populated legacy transaction, so that filling and signing need no RPC calls.
fn populated_tx(from: Address) -> TransactionRequest {
    TransactionRequest {
        from: Some(from),
        nonce: Some(0),
        to: Some(Address::with_last_byte(9).into()),
        value: Some(U256::from(100)),
        gas: Some(21_000),
        gas_price: Some(20_000_000_000),
        chain_id: Some(1),
        ..Default::default()
    }
}

#[tokio::test]
async fn fill_and_sign_transaction_without_rpc() {
    let signer = PrivateKeySigner::random();
    let address = signer.address();
    let provider = ProviderBuilder::new()
        .disable_recommended_fillers()
        .wallet(signer)
        .connect_mocked_client(Asserter::new());

    let envelope = provider.fill_and_sign_transaction(populated_tx(address)).await.unwrap();

    assert_eq!(envelope.recover_signer().unwrap(), address);
    assert_eq!(envelope.nonce(), 0);
}

#[tokio::test]
async fn fill_and_sign_transaction_without_fillers() {
    let provider =
        ProviderBuilder::new().disable_recommended_fillers().connect_mocked_client(Asserter::new());

    let err = provider.fill_and_sign_transaction(populated_tx(Address::ZERO)).await.unwrap_err();

    assert!(err.is_local_usage_error(), "{err}");
    assert!(err.to_string().contains("no fillers configured"), "{err}");
}

#[tokio::test]
async fn fill_and_sign_transaction_without_wallet() {
    let provider = ProviderBuilder::new()
        .disable_recommended_fillers()
        .with_cached_nonce_management()
        .connect_mocked_client(Asserter::new());

    let err = provider.fill_and_sign_transaction(populated_tx(Address::ZERO)).await.unwrap_err();
    assert!(err.is_local_usage_error(), "{err}");
    assert!(err.to_string().contains("no wallet configured"), "{err}");

    // missing properties are reported before the missing wallet
    let err = provider.fill_and_sign_transaction(TransactionRequest::default()).await.unwrap_err();
    assert!(err.is_local_usage_error(), "{err}");
    assert!(err.to_string().contains("missing properties"), "{err}");
}
