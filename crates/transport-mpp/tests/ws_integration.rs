//! Integration tests for `alloy_transport_mpp` against a tiny in-process MPP
//! WebSocket server.
//!
//! Each test spawns a dedicated server on `127.0.0.1:0` that scripts a
//! sequence of MPP frames and asserts the client's responses.

use alloy_json_rpc::{Id, Request};
use alloy_pubsub::PubSubConnect;
use alloy_transport_mpp::{MppEvent, MppWsConnect, VoucherProvider, VoucherRequest};
use futures::{future::BoxFuture, SinkExt, StreamExt};
use mpp::{
    client::PaymentProvider, protocol::core::Base64UrlJson, MppError, PaymentChallenge,
    PaymentCredential, PaymentPayload, Receipt,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Notify,
    time::{sleep, timeout},
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct StubProvider;

impl PaymentProvider for StubProvider {
    fn supports(&self, _: &str, _: &str) -> bool {
        true
    }
    async fn pay(&self, ch: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
        Ok(PaymentCredential::new(ch.to_echo(), PaymentPayload::hash("0xdeadbeef")))
    }
}

#[derive(Clone)]
struct StubVoucherProvider;

impl VoucherProvider for StubVoucherProvider {
    async fn next_voucher(&self, _: &VoucherRequest) -> Result<PaymentCredential, MppError> {
        let ch = test_challenge();
        Ok(PaymentCredential::new(ch.to_echo(), PaymentPayload::hash("0xvoucher")))
    }
}

fn test_challenge() -> PaymentChallenge {
    PaymentChallenge::new(
        "test-id",
        "alloy-test",
        "tempo",
        "charge",
        Base64UrlJson::from_value(&serde_json::json!({"amount":"1000","currency":"USD"})).unwrap(),
    )
}

type ServerStream = WebSocketStream<TcpStream>;

async fn spawn_server<F>(script: F) -> String
where
    F: FnOnce(ServerStream) -> BoxFuture<'static, ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        script(ws).await;
    });
    format!("ws://127.0.0.1:{port}")
}

async fn send_text(ws: &mut ServerStream, value: serde_json::Value) {
    ws.send(Message::Text(value.to_string().into())).await.unwrap();
}

async fn recv_value(ws: &mut ServerStream) -> serde_json::Value {
    let msg = ws.next().await.unwrap().unwrap();
    let text = msg.into_text().unwrap();
    serde_json::from_str(&text).unwrap()
}

fn challenge_frame() -> serde_json::Value {
    serde_json::json!({
        "type": "challenge",
        "challenge": serde_json::to_value(test_challenge()).unwrap(),
        "error": null,
    })
}

fn receipt_frame() -> serde_json::Value {
    let receipt = Receipt::success("tempo", "0xreference");
    serde_json::json!({ "type": "receipt", "receipt": receipt })
}

#[tokio::test]
async fn handshake_emits_challenge_credential_and_receipt_events() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let cred = recv_value(&mut ws).await;
            assert_eq!(cred["type"], "credential");
            assert!(cred["credential"].as_str().unwrap().starts_with("Payment "));
            send_text(&mut ws, receipt_frame()).await;
            // Hold the connection open briefly so the client can observe.
            sleep(Duration::from_millis(100)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    // Receipt watch updates.
    timeout(TIMEOUT, handle.receipt.changed()).await.unwrap().unwrap();
    let r = handle.receipt.borrow().clone().expect("receipt set");
    assert_eq!(r.reference, "0xreference");
    assert_eq!(r.method.as_str(), "tempo");

    // Events stream sees the full handshake in order.
    let e1 = timeout(TIMEOUT, handle.events.recv()).await.unwrap().unwrap();
    assert!(matches!(e1, MppEvent::Challenge(_)));
    let e2 = timeout(TIMEOUT, handle.events.recv()).await.unwrap().unwrap();
    assert!(matches!(e2, MppEvent::CredentialSent));
    let e3 = timeout(TIMEOUT, handle.events.recv()).await.unwrap().unwrap();
    assert!(matches!(e3, MppEvent::Receipt(_)));
}

#[tokio::test]
async fn json_rpc_round_trips_through_mpp_message_envelope() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Handshake.
            send_text(&mut ws, challenge_frame()).await;
            let _cred = recv_value(&mut ws).await;

            // Receive the wrapped JSON-RPC request.
            let outbound = recv_value(&mut ws).await;
            assert_eq!(outbound["type"], "message");
            let inner = &outbound["data"];
            assert_eq!(inner["method"], "eth_blockNumber");
            let id = inner["id"].clone();

            // Echo back a wrapped JSON-RPC response.
            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": "0x1234",
            });
            // mpp-rs's WsServerMessage::Data has data: String (JSON-encoded).
            send_text(
                &mut ws,
                serde_json::json!({ "type": "message", "data": response.to_string() }),
            )
            .await;
            sleep(Duration::from_millis(100)).await;
        })
    })
    .await;

    let frontend = MppWsConnect::new(url, StubProvider).into_service().await.unwrap();
    let req = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
    let resp = timeout(TIMEOUT, frontend.send(req)).await.unwrap().unwrap();
    let payload = resp.payload.as_success().unwrap();
    assert_eq!(payload.get(), "\"0x1234\"");
}

#[tokio::test]
async fn need_voucher_with_provider_emits_voucher_credential() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            send_text(
                &mut ws,
                serde_json::json!({
                    "type": "needVoucher",
                    "channelId": "0xchannel",
                    "requiredCumulative": "2000",
                    "acceptedCumulative": "1000",
                    "deposit": "5000",
                }),
            )
            .await;
            // Expect a second credential frame in response.
            let cred = recv_value(&mut ws).await;
            assert_eq!(cred["type"], "credential");
            sleep(Duration::from_millis(50)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider).with_voucher_provider(StubVoucherProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    // Drain events until we see VoucherSent.
    let mut saw_voucher = false;
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        if let MppEvent::VoucherSent = ev {
            saw_voucher = true;
            break;
        }
    }
    assert!(saw_voucher, "expected VoucherSent event");
}

#[tokio::test]
async fn need_voucher_without_provider_surfaces_error_event() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            send_text(
                &mut ws,
                serde_json::json!({
                    "type": "needVoucher",
                    "channelId": "0xchannel",
                    "requiredCumulative": "2000",
                    "acceptedCumulative": "1000",
                    "deposit": "5000",
                }),
            )
            .await;
            // Translator should error and close; absorb the close.
            let _ = ws.next().await;
        })
    })
    .await;

    // Default voucher provider = NoVoucher.
    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let mut saw_error = false;
    let mut saw_need_voucher = false;
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        match ev {
            MppEvent::NeedVoucher(_) => saw_need_voucher = true,
            MppEvent::Error(_) => {
                saw_error = true;
                break;
            }
            _ => {}
        }
    }
    assert!(saw_need_voucher, "expected NeedVoucher event");
    assert!(saw_error, "expected Error event when NoVoucher provider rejects");
}

#[tokio::test]
async fn duplicate_challenge_or_voucher_closes_connection() {
    // Test (a): two Challenge frames back-to-back before the first pay completes.
    // The provider blocks until released so the second Challenge arrives while
    // the first JoinHandle is still pending.
    #[derive(Clone)]
    struct BlockingProvider(Arc<Notify>);
    impl PaymentProvider for BlockingProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            true
        }
        async fn pay(&self, ch: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            self.0.notified().await;
            Ok(PaymentCredential::new(ch.to_echo(), PaymentPayload::hash("0xfeedface")))
        }
    }

    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            send_text(&mut ws, challenge_frame()).await;
            // Drain whatever the client sends until the socket dies.
            while let Some(msg) = ws.next().await {
                if msg.is_err() {
                    break;
                }
            }
        })
    })
    .await;

    let release = Arc::new(Notify::new());
    let connect = MppWsConnect::new(url, BlockingProvider(release.clone()));
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();
    // Drop the connector so its `events_tx` sender clone goes away — the
    // broadcast channel only closes once *all* senders are gone.
    drop(connect);

    // The translator should close because the second Challenge is duplicate.
    // The pending pay task is aborted on shutdown — release the latch so it
    // doesn't hold the test open if abort racy.
    release.notify_waiters();

    // Receiver should eventually close (RecvError::Closed) once the translator exits.
    let mut closed = false;
    for _ in 0..50 {
        match timeout(Duration::from_millis(100), handle.events.recv()).await {
            Ok(Err(_)) => {
                closed = true;
                break;
            }
            Ok(Ok(_)) => continue,
            Err(_) => continue,
        }
    }
    assert!(closed, "translator should close after duplicate Challenge");
}

#[tokio::test]
async fn duplicate_need_voucher_closes_connection() {
    #[derive(Clone)]
    struct BlockingVoucher(Arc<Notify>);
    impl VoucherProvider for BlockingVoucher {
        async fn next_voucher(&self, _: &VoucherRequest) -> Result<PaymentCredential, MppError> {
            self.0.notified().await;
            let ch = test_challenge();
            Ok(PaymentCredential::new(ch.to_echo(), PaymentPayload::hash("0xvoucher")))
        }
    }

    let voucher_frame = serde_json::json!({
        "type": "needVoucher",
        "channelId": "0xchannel",
        "requiredCumulative": "2000",
        "acceptedCumulative": "1000",
        "deposit": "5000",
    });

    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Complete the initial challenge handshake.
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            // Two needVoucher frames back-to-back.
            send_text(&mut ws, voucher_frame.clone()).await;
            send_text(&mut ws, voucher_frame).await;
            while let Some(msg) = ws.next().await {
                if msg.is_err() {
                    break;
                }
            }
        })
    })
    .await;

    let release = Arc::new(Notify::new());
    let connect = MppWsConnect::new(url, StubProvider)
        .with_voucher_provider(BlockingVoucher(release.clone()));
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();
    drop(connect);

    release.notify_waiters();

    let mut closed = false;
    for _ in 0..50 {
        match timeout(Duration::from_millis(100), handle.events.recv()).await {
            Ok(Err(_)) => {
                closed = true;
                break;
            }
            Ok(Ok(_)) => continue,
            Err(_) => continue,
        }
    }
    assert!(closed, "translator should close after duplicate NeedVoucher");
}

#[tokio::test]
async fn server_error_frame_surfaces_event_and_closes() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, serde_json::json!({ "type": "error", "error": "bad request" }))
                .await;
            sleep(Duration::from_millis(50)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let ev = timeout(TIMEOUT, handle.events.recv()).await.unwrap().unwrap();
    match ev {
        MppEvent::Error(s) => assert_eq!(s, "bad request"),
        other => panic!("unexpected event: {other:?}"),
    }
}
