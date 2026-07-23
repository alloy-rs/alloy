use alloy_json_rpc::{RequestPacket, Response, ResponsePacket};
use alloy_primitives::{bytes, Address, U256};
#[cfg(feature = "anvil-api")]
use alloy_provider::ext::AnvilApi;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_transport::{mock::Asserter, TransportError, TransportFut};
use serde_json::{json, Value};
use std::{
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

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
async fn mocked_send_raw_transaction_sync_timeout_params() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = ProviderBuilder::new()
        .connect_client(RpcClient::new(CaptureTransport::new(requests.clone()), true));

    let _ = provider.send_raw_transaction_sync(&[0x12, 0x34]).await;
    let _ = provider
        .send_raw_transaction_sync_with_timeout(&[0x12, 0x34], Duration::from_millis(1_500))
        .await;
    #[cfg(feature = "anvil-api")]
    {
        let _ = provider.eth_send_raw_transaction_sync(bytes!("1234")).await;
        let _ = provider
            .eth_send_raw_transaction_sync_with_timeout(
                bytes!("1234"),
                Duration::from_millis(1_500),
            )
            .await;
    }

    let expected = vec![
        json!({
            "method": "eth_sendRawTransactionSync",
            "params": ["0x1234"],
        }),
        json!({
            "method": "eth_sendRawTransactionSync",
            "params": ["0x1234", 1500],
        }),
    ];
    #[cfg(feature = "anvil-api")]
    let expected = {
        let mut expected = expected;
        expected.extend([
            json!({
                "method": "eth_sendRawTransactionSync",
                "params": ["0x1234"],
            }),
            json!({
                "method": "eth_sendRawTransactionSync",
                "params": ["0x1234", 1500],
            }),
        ]);
        expected
    };
    let requests = requests.lock().unwrap();
    assert_eq!(requests.as_slice(), expected);
}

#[derive(Clone, Debug)]
struct CaptureTransport {
    requests: Arc<Mutex<Vec<Value>>>,
}

impl CaptureTransport {
    const fn new(requests: Arc<Mutex<Vec<Value>>>) -> Self {
        Self { requests }
    }
}

impl tower::Service<RequestPacket> for CaptureTransport {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let requests = self.requests.clone();
        Box::pin(async move {
            let req = req.as_single().expect("expected single request");
            let params = req
                .params()
                .map(|params| serde_json::from_str(params.get()).unwrap())
                .unwrap_or(Value::Null);

            requests.lock().unwrap().push(json!({
                "method": req.method(),
                "params": params,
            }));

            Ok(ResponsePacket::Single(Response::internal_error(req.id().clone())))
        })
    }
}
