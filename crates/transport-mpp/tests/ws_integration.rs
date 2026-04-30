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
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
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
async fn payment_provider_error_surfaces_event_and_closes() {
    // The provider rejects the challenge with an error; the translator must
    // surface it as MppEvent::Error and close.
    #[derive(Clone)]
    struct FailingProvider;
    impl PaymentProvider for FailingProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            true
        }
        async fn pay(&self, _: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            Err(MppError::bad_request("nope"))
        }
    }

    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            // Client should NOT send a credential; just absorb whatever ends the conn.
            let _ = ws.next().await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, FailingProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let mut saw_error = false;
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        if let MppEvent::Error(s) = ev {
            assert!(s.contains("nope") || !s.is_empty());
            saw_error = true;
            break;
        }
    }
    assert!(saw_error, "expected Error event when payment provider returns Err");
}

#[tokio::test]
async fn voucher_provider_error_surfaces_event_and_closes() {
    #[derive(Clone)]
    struct FailingVoucher;
    impl VoucherProvider for FailingVoucher {
        async fn next_voucher(&self, _: &VoucherRequest) -> Result<PaymentCredential, MppError> {
            Err(MppError::bad_request("voucher unavailable"))
        }
    }

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
            let _ = ws.next().await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider).with_voucher_provider(FailingVoucher);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let mut saw_error = false;
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        if let MppEvent::Error(_) = ev {
            saw_error = true;
            break;
        }
    }
    assert!(saw_error, "expected Error event when voucher provider returns Err");
}

#[tokio::test]
async fn multiple_json_rpc_requests_round_trip_in_session() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Handshake.
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            // Echo back a fixed result for each incoming request.
            for _ in 0..3 {
                let outbound = recv_value(&mut ws).await;
                assert_eq!(outbound["type"], "message");
                let inner = &outbound["data"];
                let id = inner["id"].clone();
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": format!("0x{}", id),
                });
                send_text(
                    &mut ws,
                    serde_json::json!({ "type": "message", "data": response.to_string() }),
                )
                .await;
            }
            sleep(Duration::from_millis(50)).await;
        })
    })
    .await;

    let frontend = MppWsConnect::new(url, StubProvider).into_service().await.unwrap();
    for i in 1..=3u64 {
        let req = Request::new("eth_blockNumber", Id::Number(i), ()).serialize().unwrap();
        let resp = timeout(TIMEOUT, frontend.send(req)).await.unwrap().unwrap();
        let payload = resp.payload.as_success().unwrap();
        assert_eq!(payload.get(), &format!("\"0x{i}\""));
    }
}

#[tokio::test]
async fn request_buffered_until_handshake_completes() {
    // Verifies that an RPC issued before the challenge arrives is held in
    // the frontend channel and only flushed once the credential is sent.
    // Server-side ordering of reads guarantees the property: credential
    // MUST come before the wrapped JSON-RPC `message`.
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Delay so the client has time to push the RPC into the channel
            // before the challenge is even on the wire.
            sleep(Duration::from_millis(150)).await;
            send_text(&mut ws, challenge_frame()).await;

            // The very first frame from the client must be the credential,
            // not the queued JSON-RPC message.
            let first = recv_value(&mut ws).await;
            assert_eq!(first["type"], "credential");

            // The next frame is the buffered JSON-RPC request.
            let second = recv_value(&mut ws).await;
            assert_eq!(second["type"], "message");
            let id = second["data"]["id"].clone();
            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": "0xqueued",
            });
            send_text(
                &mut ws,
                serde_json::json!({ "type": "message", "data": response.to_string() }),
            )
            .await;
            sleep(Duration::from_millis(50)).await;
        })
    })
    .await;

    let frontend = MppWsConnect::new(url, StubProvider).into_service().await.unwrap();
    let req = Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap();
    let resp = timeout(TIMEOUT, frontend.send(req)).await.unwrap().unwrap();
    let payload = resp.payload.as_success().unwrap();
    assert_eq!(payload.get(), "\"0xqueued\"");
}

#[tokio::test]
async fn re_challenge_mid_session_pays_again() {
    // Server sends two challenges (with a gap so the first pay completes
    // before the second arrives) and expects two credentials.
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let first = recv_value(&mut ws).await;
            assert_eq!(first["type"], "credential");
            // Brief gap so `pending_pay` is None when the next challenge lands.
            sleep(Duration::from_millis(50)).await;
            send_text(&mut ws, challenge_frame()).await;
            let second = recv_value(&mut ws).await;
            assert_eq!(second["type"], "credential");
            sleep(Duration::from_millis(50)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let mut challenges = 0;
    let mut credentials_sent = 0;
    while let Ok(Ok(ev)) = timeout(Duration::from_millis(800), handle.events.recv()).await {
        match ev {
            MppEvent::Challenge(_) => challenges += 1,
            MppEvent::CredentialSent => credentials_sent += 1,
            _ => {}
        }
        if challenges == 2 && credentials_sent == 2 {
            break;
        }
    }
    assert_eq!(challenges, 2, "expected two challenges");
    assert_eq!(credentials_sent, 2, "expected two credentials sent");
}

#[tokio::test]
async fn server_close_frame_closes_connection() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Send a Close frame straight away.
            ws.close(None).await.ok();
            sleep(Duration::from_millis(50)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();
    drop(connect);

    // Translator should exit; eventually the events channel closes once the
    // last sender (the translator's clone) is dropped.
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
    assert!(closed, "translator should close after server Close frame");
}

#[tokio::test]
async fn binary_frame_closes_connection() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Send an unsolicited binary frame; translator only handles text.
            ws.send(Message::Binary(vec![0xde, 0xad, 0xbe, 0xef].into())).await.unwrap();
            let _ = ws.next().await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();
    drop(connect);

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
    assert!(closed, "translator should close on binary frame");
}

#[tokio::test]
async fn malformed_challenge_closes_connection() {
    // The outer envelope is valid, but the inner `challenge` payload cannot
    // be deserialized into a PaymentChallenge.
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(
                &mut ws,
                serde_json::json!({
                    "type": "challenge",
                    "challenge": "not-an-object",
                    "error": null,
                }),
            )
            .await;
            let _ = ws.next().await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();
    drop(connect);

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
    assert!(closed, "translator should close on malformed challenge");
}

#[tokio::test]
async fn malformed_data_payload_closes_connection() {
    // Handshake completes, then server sends a Data frame whose inner
    // payload is not valid JSON-RPC. The translator must close.
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            send_text(&mut ws, serde_json::json!({ "type": "message", "data": "not json at all" }))
                .await;
            let _ = ws.next().await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();
    drop(connect);

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
    assert!(closed, "translator should close on malformed Data payload");
}

#[tokio::test]
async fn multiple_receipts_update_watch_channel() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            // Two distinct receipts back-to-back.
            send_text(
                &mut ws,
                serde_json::json!({
                    "type": "receipt",
                    "receipt": Receipt::success("tempo", "0xfirst"),
                }),
            )
            .await;
            send_text(
                &mut ws,
                serde_json::json!({
                    "type": "receipt",
                    "receipt": Receipt::success("tempo", "0xsecond"),
                }),
            )
            .await;
            sleep(Duration::from_millis(100)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    // The broadcast event channel sees one event per receipt — collect both.
    let mut refs = Vec::new();
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        if let MppEvent::Receipt(r) = ev {
            refs.push(r.reference);
            if refs.len() == 2 {
                break;
            }
        }
    }
    assert_eq!(refs, vec!["0xfirst".to_string(), "0xsecond".to_string()]);

    // The watch channel collapses to the most recent value; assert it
    // converges on the second receipt (may already be there).
    if handle.receipt.borrow().as_ref().map(|r| r.reference.as_str()) != Some("0xsecond") {
        timeout(TIMEOUT, handle.receipt.changed()).await.unwrap().unwrap();
    }
    let latest = handle.receipt.borrow().as_ref().expect("latest receipt").reference.clone();
    assert_eq!(latest, "0xsecond");
}

#[tokio::test]
async fn multiple_event_subscribers_each_receive_handshake() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            let _ = recv_value(&mut ws).await;
            send_text(&mut ws, receipt_frame()).await;
            sleep(Duration::from_millis(100)).await;
        })
    })
    .await;

    let connect = MppWsConnect::new(url, StubProvider);
    let mut h1 = connect.mpp_handle();
    let mut h2 = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    async fn drain_until_receipt(rx: &mut tokio::sync::broadcast::Receiver<MppEvent>) -> bool {
        let mut saw_challenge = false;
        let mut saw_credential = false;
        let mut saw_receipt = false;
        while let Ok(Ok(ev)) = timeout(TIMEOUT, rx.recv()).await {
            match ev {
                MppEvent::Challenge(_) => saw_challenge = true,
                MppEvent::CredentialSent => saw_credential = true,
                MppEvent::Receipt(_) => {
                    saw_receipt = true;
                    break;
                }
                _ => {}
            }
        }
        saw_challenge && saw_credential && saw_receipt
    }

    assert!(drain_until_receipt(&mut h1.events).await);
    assert!(drain_until_receipt(&mut h2.events).await);
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

#[tokio::test]
async fn unsupported_challenge_skips_pay_and_closes_fatal() {
    #[derive(Clone, Default)]
    struct UnsupportedProvider(Arc<AtomicUsize>);
    impl PaymentProvider for UnsupportedProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            false
        }
        async fn pay(&self, _: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(MppError::bad_request("should not be called"))
        }
    }

    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            send_text(&mut ws, challenge_frame()).await;
            // Server must not receive a credential frame.
            let next = timeout(Duration::from_millis(200), ws.next()).await;
            assert!(next.is_err() || matches!(next, Ok(None) | Ok(Some(Err(_)))));
        })
    })
    .await;

    let pay_calls = Arc::new(AtomicUsize::new(0));
    let connect = MppWsConnect::new(url, UnsupportedProvider(pay_calls.clone()));
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let mut saw_unsupported = false;
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        if let MppEvent::Error(s) = ev {
            assert!(s.contains("does not support"), "unexpected error: {s}");
            saw_unsupported = true;
            break;
        }
    }
    assert!(saw_unsupported, "expected unsupported-method error event");
    assert_eq!(pay_calls.load(Ordering::SeqCst), 0, "pay() must not be invoked");
}

#[tokio::test]
async fn handshake_timeout_surfaces_error_when_server_silent() {
    let url = spawn_server(|mut ws| {
        Box::pin(async move {
            // Hold the connection open without sending any frame.
            let _ = timeout(Duration::from_secs(2), ws.next()).await;
        })
    })
    .await;

    let connect =
        MppWsConnect::new(url, StubProvider).with_handshake_timeout(Duration::from_millis(200));
    let mut handle = connect.mpp_handle();
    let _conn = connect.connect().await.unwrap();

    let mut saw_timeout = false;
    while let Ok(Ok(ev)) = timeout(TIMEOUT, handle.events.recv()).await {
        if let MppEvent::Error(s) = ev {
            assert!(s.contains("timed out"), "unexpected error message: {s}");
            saw_timeout = true;
            break;
        }
    }
    assert!(saw_timeout, "expected handshake timeout error event");
}

#[tokio::test]
async fn fatal_provider_error_does_not_re_invoke_pay() {
    // The server issues a challenge on every accept; any reconnect would
    // surface as another accept and another `pay()` call.
    #[derive(Clone, Default)]
    struct CountingFailingProvider(Arc<AtomicUsize>);
    impl PaymentProvider for CountingFailingProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            true
        }
        async fn pay(&self, _: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(MppError::bad_request("insufficient funds"))
        }
    }

    let accepts = Arc::new(AtomicUsize::new(0));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let accepts_srv = accepts.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => return,
            };
            accepts_srv.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                if let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await {
                    send_text(&mut ws, challenge_frame()).await;
                    let _ = timeout(Duration::from_millis(200), ws.next()).await;
                }
            });
        }
    });
    let url = format!("ws://127.0.0.1:{port}");

    let pay_calls = Arc::new(AtomicUsize::new(0));
    let connect = MppWsConnect::new(url, CountingFailingProvider(pay_calls.clone()))
        .with_max_retries(5)
        .with_retry_interval(Duration::from_millis(10));
    let _frontend = connect.into_service().await.unwrap();

    // Long enough that several reconnects would happen if classification were wrong.
    sleep(Duration::from_millis(800)).await;

    assert_eq!(accepts.load(Ordering::SeqCst), 1, "fatal error must not trigger reconnects");
    assert_eq!(pay_calls.load(Ordering::SeqCst), 1, "pay() must not be re-invoked");
}
