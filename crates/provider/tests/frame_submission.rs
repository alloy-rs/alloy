//! End-to-end filler and raw submission regressions without a running node.

use alloy_consensus::{
    transaction::{PooledTransaction, TxHashRef},
    Transaction, TxEip8141,
};
use alloy_eips::{
    eip7594::BlobTransactionSidecarEip7594,
    eip8141::{Frame, TransactionFees},
    Decodable2718,
};
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
use alloy_network::{AnyNetwork, AnyTypedTransaction, Ethereum, EthereumWallet, NetworkWallet};
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::TransportError;

#[tokio::test]
async fn presigned_wallet_paths_move_frames_and_check_sender() {
    let tx = TxEip8141 {
        sender: Address::repeat_byte(1),
        frames: vec![Frame::default()],
        ..Default::default()
    };
    let sender = tx.sender;
    let wallet = EthereumWallet::default();
    let wrong_sender = Address::repeat_byte(2);
    assert!(<EthereumWallet as NetworkWallet<Ethereum>>::sign_transaction_from(
        &wallet,
        wrong_sender,
        tx.clone().into()
    )
    .await
    .is_err());
    let frames = tx.frames.as_ptr();
    let envelope = <EthereumWallet as NetworkWallet<Ethereum>>::sign_transaction_from(
        &wallet,
        sender,
        tx.into(),
    )
    .await
    .unwrap();
    assert_eq!(envelope.frame_transaction().unwrap().frames.as_ptr(), frames);

    let tx = envelope.into_typed_transaction();
    let envelope = <EthereumWallet as NetworkWallet<AnyNetwork>>::sign_transaction_from(
        &wallet,
        sender,
        AnyTypedTransaction::Ethereum(tx),
    )
    .await
    .unwrap();
    assert_eq!(envelope.frame_transaction().unwrap().frames.as_ptr(), frames);
}

#[tokio::test]
async fn frame_submission_keeps_sidecar_and_full_width_fees() {
    let sidecar = BlobTransactionSidecarEip7594 {
        commitments: vec![Default::default()],
        ..Default::default()
    };
    let tx = TxEip8141 {
        chain_id: 1,
        sender: Address::repeat_byte(1),
        frames: vec![Frame::default()],
        fees: TransactionFees {
            max_fee_per_gas: U256::from(1) << 128,
            max_priority_fee_per_gas: U256::from(1),
            max_fee_per_blob_gas: U256::from(1) << 128,
        },
        blob_versioned_hashes: sidecar.versioned_hashes().collect(),
        ..Default::default()
    };
    let expected = tx.clone();
    let expected_sidecar = sidecar.clone();
    let transport = tower::service_fn(
        move |request: RequestPacket| -> alloy_transport::TransportFut<'static> {
            let expected = expected.clone();
            let sidecar = expected_sidecar.clone();
            Box::pin(async move {
                let RequestPacket::Single(request) = request else { panic!("unexpected batch") };
                // No gas/fee estimation RPC is allowed to rewrite the self-authorized transaction.
                assert_eq!(request.method(), "eth_sendRawTransaction");
                let json: serde_json::Value =
                    serde_json::from_str(request.serialized().get()).unwrap();
                let bytes: Bytes = serde_json::from_value(json["params"][0].clone()).unwrap();
                let pooled = PooledTransaction::decode_2718_exact(&bytes).unwrap();
                assert_eq!(pooled.frame_transaction(), Some(&expected));
                assert_eq!(pooled.as_eip8141().unwrap().sidecar(), Some(&sidecar));
                Ok::<_, TransportError>(ResponsePacket::Single(Response {
                    id: request.id().clone(),
                    payload: ResponsePayload::Success(
                        serde_json::value::to_raw_value(pooled.tx_hash()).unwrap(),
                    ),
                }))
            })
        },
    );
    let client = RpcClient::builder().transport(transport, true);
    let wallet = PrivateKeySigner::random();
    let provider = ProviderBuilder::new().wallet(wallet).connect_client(client);
    let mut request: TransactionRequest = tx.clone().into();
    request.sidecar = Some(sidecar.into());
    let pending = provider.send_transaction(request).await.unwrap();
    assert_eq!(pending.tx_hash(), &tx.tx_hash());
}
