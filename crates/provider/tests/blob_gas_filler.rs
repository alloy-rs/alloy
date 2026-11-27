//! Regression tests for the blob gas filler's protocol fee floor.

use alloy_consensus::{SidecarBuilder, SimpleCoder, Transaction, TxEnvelope};
use alloy_eips::{eip2718::Decodable2718, eip4844::BLOB_TX_MIN_BLOB_GASPRICE};
use alloy_network::{Ethereum, TransactionBuilder4844};
use alloy_primitives::{address, Address, Bytes, B256};
use alloy_provider::{
    fillers::{BlobGasEstimator, BlobGasFiller, FillerControlFlow, TxFiller},
    Provider, ProviderBuilder,
};
use alloy_rpc_types_eth::{FeeHistory, TransactionRequest};
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::mock::Asserter;

// All other fields are populated so that only blob gas estimation needs an RPC call.
fn blob_request() -> TransactionRequest {
    let sidecar = SidecarBuilder::<SimpleCoder>::from_slice(b"Hello World").build_4844().unwrap();
    TransactionRequest {
        from: Some(Address::repeat_byte(0x11)),
        to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
        nonce: Some(0),
        chain_id: Some(1),
        gas: Some(21_000),
        max_fee_per_gas: Some(2_000_000_000),
        max_priority_fee_per_gas: Some(1_000_000_000),
        sidecar: Some(sidecar.into()),
        ..Default::default()
    }
}

fn fee_history(blob_fee: u128) -> FeeHistory {
    FeeHistory {
        oldest_block: 1,
        base_fee_per_gas: vec![1_000_000_000; 3],
        gas_used_ratio: vec![0.5; 2],
        // Only the last entry predicts the next block's fee.
        base_fee_per_blob_gas: vec![100, 200, blob_fee],
        blob_gas_used_ratio: vec![0.0; 2],
        reward: None,
    }
}

fn methods(asserter: &Asserter) -> Vec<String> {
    asserter.take_requests().iter().map(|r| r.method().to_string()).collect()
}

#[tokio::test]
async fn blob_fee_estimate_satisfies_status_after_one_request() {
    let request = blob_request();
    for blob_fee in [0, BLOB_TX_MIN_BLOB_GASPRICE, 42, u128::MAX] {
        for max_fee_per_blob_gas in [None, Some(0)] {
            let asserter = Asserter::new();
            let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
            asserter.push_success(&fee_history(blob_fee));
            let tx = TransactionRequest { max_fee_per_blob_gas, ..request.clone() };

            let filled = provider.fill(tx).await.unwrap();
            let builder = filled.as_builder().unwrap();
            assert_eq!(
                builder.max_fee_per_blob_gas(),
                Some(blob_fee.max(BLOB_TX_MIN_BLOB_GASPRICE))
            );
            assert_eq!(
                <BlobGasFiller as TxFiller<Ethereum>>::status(&BlobGasFiller::default(), builder),
                FillerControlFlow::Finished
            );
            assert_eq!(methods(&asserter), ["eth_feeHistory"]);
        }
    }
}

#[tokio::test]
async fn custom_blob_fee_estimate_obeys_floor() {
    let request = blob_request();
    for estimate in [0, BLOB_TX_MIN_BLOB_GASPRICE, 42, u128::MAX] {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .with_blob_gas_estimator(BlobGasEstimator::new(move |base_fee, ratios| {
                assert_eq!(base_fee, 7);
                assert_eq!(ratios, [0.0, 0.0]);
                estimate
            }))
            .connect_mocked_client(asserter.clone());
        asserter.push_success(&fee_history(7));

        let filled = provider.fill(request.clone()).await.unwrap();
        assert_eq!(
            filled.as_builder().unwrap().max_fee_per_blob_gas(),
            Some(estimate.max(BLOB_TX_MIN_BLOB_GASPRICE))
        );
        assert_eq!(methods(&asserter), ["eth_feeHistory"]);
    }
}

#[tokio::test]
async fn valid_blob_fee_skips_estimation() {
    let request = blob_request();
    let asserter = Asserter::new();
    let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
    for fee in [BLOB_TX_MIN_BLOB_GASPRICE, 42, u128::MAX] {
        let tx = TransactionRequest { max_fee_per_blob_gas: Some(fee), ..request.clone() };
        let filled = provider.fill(tx).await.unwrap();
        assert_eq!(filled.as_builder().unwrap().max_fee_per_blob_gas(), Some(fee));
    }
    assert!(methods(&asserter).is_empty());
}

#[tokio::test]
async fn zero_rpc_blob_fee_is_floored_before_signing_and_broadcast() {
    let request = blob_request();
    for max_fee_per_blob_gas in [None, Some(0)] {
        let signer = PrivateKeySigner::random();
        let asserter = Asserter::new();
        let provider =
            ProviderBuilder::new().wallet(signer).connect_mocked_client(asserter.clone());
        asserter.push_success(&fee_history(0));
        asserter.push_success(&B256::ZERO);
        let tx = TransactionRequest { from: None, max_fee_per_blob_gas, ..request.clone() };

        let _pending = provider.send_transaction(tx).await.unwrap();

        let requests = asserter.take_requests();
        let sent: Vec<_> = requests.iter().map(|r| r.method()).collect();
        assert_eq!(sent, ["eth_feeHistory", "eth_sendRawTransaction"]);
        let (raw,): (Bytes,) = serde_json::from_str(requests[1].params().unwrap().get()).unwrap();
        let broadcast = TxEnvelope::decode_2718(&mut raw.as_ref()).unwrap();
        assert_eq!(broadcast.max_fee_per_blob_gas(), Some(BLOB_TX_MIN_BLOB_GASPRICE));
    }
}

#[tokio::test]
async fn missing_blob_fee_history_returns_error() {
    let asserter = Asserter::new();
    let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
    asserter.push_success(&FeeHistory { base_fee_per_blob_gas: vec![], ..fee_history(1) });

    let err = provider.fill(blob_request()).await.unwrap_err();
    assert!(matches!(err, alloy_json_rpc::RpcError::NullResp), "{err}");
    assert_eq!(methods(&asserter), ["eth_feeHistory"]);
}

#[tokio::test]
async fn blob_fee_rpc_error_is_propagated_without_broadcast() {
    let asserter = Asserter::new();
    let provider = ProviderBuilder::new()
        .wallet(PrivateKeySigner::random())
        .connect_mocked_client(asserter.clone());
    asserter.push_failure_msg("fee history unavailable");
    let tx = TransactionRequest { from: None, ..blob_request() };

    let err = provider.send_transaction(tx).await.err().unwrap();
    assert!(err.to_string().contains("fee history unavailable"), "{err}");
    assert_eq!(methods(&asserter), ["eth_feeHistory"]);
}
