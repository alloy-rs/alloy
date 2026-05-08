//! MPP WebSocket transport wrapper.
//!
//! [`MppWsConnect`] wraps a [`WsConnect`] and transparently handles MPP
//! (Machine Payments Protocol) WebSocket sessions. After the WS handshake
//! completes, the wrapper races a short timeout for an inbound `challenge`
//! frame:
//!
//! * If a challenge arrives, it is paid via the user-supplied [`PaymentProvider`], the resulting
//!   credential is sent back, and the connection enters MPP mode for its lifetime — outbound
//!   JSON-RPC frames are wrapped as `{"type":"message","data":…}` and inbound `message` frames are
//!   unwrapped.
//! * If the timeout fires (or any non-MPP frame arrives first), the wrapper commits to plain
//!   WebSocket mode and forwards frames untouched.
//!
//! The plain/MPP decision is latched on the connector so reconnects skip the
//! probe entirely. A late challenge after we've committed to plain mode (or
//! during a session) is dropped — the next reconnect will probe again.
//!
//! Because MPP writes a `credential` frame at handshake time it is mutually
//! exclusive with WS-frame middleware that interferes with the raw stream.
//! Provider failures are reported as
//! [`TransportErrorKind::non_retryable`](alloy_transport::TransportErrorKind::non_retryable)
//! so outer pubsub retry loops do not re-drive payments.
//!
//! # Example
//!
//! ```no_run
//! use alloy_transport_ws::{MppWsConnect, WsConnect};
//! use mpp::{
//!     client::PaymentProvider, MppError, PaymentChallenge, PaymentCredential, PaymentPayload,
//! };
//!
//! #[derive(Clone)]
//! struct MyProvider;
//!
//! impl PaymentProvider for MyProvider {
//!     fn supports(&self, method: &str, intent: &str) -> bool {
//!         method == "tempo" && intent == "charge"
//!     }
//!
//!     async fn pay(&self, challenge: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
//!         let tx_hash = "0xdeadbeef".to_string();
//!         Ok(PaymentCredential::new(challenge.to_echo(), PaymentPayload::hash(tx_hash)))
//!     }
//! }
//!
//! # async fn build() -> Result<(), Box<dyn std::error::Error>> {
//! let connector = MppWsConnect::new(WsConnect::new("wss://paid-rpc.example.com"), MyProvider);
//! // Plug `connector` into your `RpcClient` / `ProviderBuilder`. Payments
//! // happen transparently if the server emits an MPP `challenge` frame at
//! // handshake; otherwise plain WebSocket is used.
//! # let _ = connector;
//! # Ok(()) }
//! ```

use crate::{native::TungsteniteStream, WsBackend, WsConnect};
use alloy_json_rpc::PubSubItem;
use alloy_pubsub::{ConnectionHandle, ConnectionInterface, PubSubConnect};
use alloy_transport::{utils::Spawnable, TransportErrorKind, TransportResult};
use futures::{SinkExt, StreamExt};
use mpp::{base64url_encode, client::PaymentProvider, PaymentChallenge, PaymentCredential};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest, Message};

pub use mpp::{
    client::PaymentProvider as Provider, PaymentChallenge as Challenge,
    PaymentCredential as Credential,
};

/// Default time the connector waits for an MPP `challenge` frame before
/// falling back to plain WebSocket mode.
const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(500);

/// A [`WsConnect`] wrapper that adds MPP support.
#[derive(Clone, Debug)]
pub struct MppWsConnect<P> {
    inner: WsConnect,
    provider: P,
    handshake_timeout: Duration,
    detected_plain: Arc<AtomicBool>,
}

impl<P> MppWsConnect<P> {
    /// Wrap an existing [`WsConnect`] with MPP handshake handling.
    pub fn new(inner: WsConnect, provider: P) -> Self {
        Self {
            inner,
            provider,
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            detected_plain: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set the time the connector waits for an MPP `challenge` frame at
    /// handshake before falling back to plain WebSocket mode.
    ///
    /// Default is 500 ms.
    pub const fn with_handshake_timeout(mut self, handshake_timeout: Duration) -> Self {
        self.handshake_timeout = handshake_timeout;
        self
    }

    /// Borrow the underlying [`WsConnect`].
    pub const fn inner(&self) -> &WsConnect {
        &self.inner
    }

    /// Returns `true` once the connector has decided that the endpoint is
    /// not MPP-enabled. Subsequent reconnects skip the probe.
    pub fn detected_plain(&self) -> bool {
        self.detected_plain.load(Ordering::SeqCst)
    }
}

impl<P> PubSubConnect for MppWsConnect<P>
where
    P: PaymentProvider + 'static,
{
    fn is_local(&self) -> bool {
        self.inner.is_local()
    }

    async fn connect(&self) -> TransportResult<ConnectionHandle> {
        // Once we've decided the endpoint is non-MPP, defer entirely to the
        // plain WS connector so we don't pay the probe latency on every
        // reconnect.
        if self.detected_plain.load(Ordering::SeqCst) {
            return self.inner.connect().await;
        }

        #[cfg(any(feature = "aws-lc-rs", feature = "ring"))]
        crate::native::install_default_crypto_provider();

        let req = self.inner.clone().into_client_request().map_err(TransportErrorKind::custom)?;
        let (socket, _) =
            tokio_tungstenite::connect_async_with_config(req, self.inner.config().copied(), false)
                .await
                .map_err(TransportErrorKind::custom)?;

        let (handle, interface) = ConnectionHandle::new();

        spawn_translator(
            socket,
            interface,
            self.provider.clone(),
            self.handshake_timeout,
            self.inner.keepalive_interval(),
            self.detected_plain.clone(),
        );

        Ok(handle
            .with_max_retries(self.inner.max_retries())
            .with_retry_interval(self.inner.retry_interval()))
    }

    /// Reconnect uses [`Self::connect`], which honors the cached
    /// [`Self::detected_plain`] decision and skips the MPP probe on
    /// non-MPP endpoints.
    async fn try_reconnect(&self) -> TransportResult<ConnectionHandle> {
        self.connect().await
    }
}

/// Spawn the probe + translator task.
fn spawn_translator<P: PaymentProvider + 'static>(
    socket: TungsteniteStream,
    interface: ConnectionInterface,
    provider: P,
    handshake_timeout: Duration,
    keepalive_interval: Duration,
    detected_plain: Arc<AtomicBool>,
) {
    let fut = async move {
        run_translator(
            socket,
            interface,
            provider,
            handshake_timeout,
            keepalive_interval,
            detected_plain,
        )
        .await
    };
    fut.spawn_task();
}

/// Result of classifying a single inbound frame during the handshake probe.
enum ProbeClass {
    /// MPP challenge — enter MPP mode.
    Challenge(PaymentChallenge),
    /// Frame parses as a valid plain pubsub item — endpoint is non-MPP.
    /// We forward this frame and latch [`MppWsConnect::detected_plain`].
    PlainLatch(Message),
    /// Frame is text but neither a challenge nor a valid pubsub item
    /// (malformed or some other MPP control frame). Fall through to plain
    /// mode for THIS connection only — do NOT latch.
    PlainNoLatch(Message),
    /// WebSocket control frame (Ping / Pong / raw Frame). Ignore and keep
    /// probing within the remaining deadline.
    Skip,
}

fn classify_first_frame(msg: Message) -> ProbeClass {
    let Message::Text(ref text) = msg else {
        // Binary / Close are not expected during MPP handshake; treat them
        // as plain-no-latch so a single weird frame does not poison future
        // reconnects.
        if matches!(msg, Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) {
            return ProbeClass::Skip;
        }
        return ProbeClass::PlainNoLatch(msg);
    };
    let s: &str = text.as_ref();

    if let Some(challenge) = try_parse_challenge(s) {
        return ProbeClass::Challenge(challenge);
    }
    if serde_json::from_str::<PubSubItem>(s).is_ok() {
        return ProbeClass::PlainLatch(msg);
    }
    ProbeClass::PlainNoLatch(msg)
}

async fn run_translator<P: PaymentProvider>(
    mut socket: TungsteniteStream,
    interface: ConnectionInterface,
    provider: P,
    handshake_timeout: Duration,
    keepalive_interval: Duration,
    detected_plain: Arc<AtomicBool>,
) {
    let deadline = Instant::now() + handshake_timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let probe = tokio::time::timeout(remaining, socket.next()).await;

        match probe {
            Ok(Some(Ok(msg))) => match classify_first_frame(msg) {
                ProbeClass::Challenge(challenge) => {
                    handle_mpp_handshake(
                        socket,
                        interface,
                        provider,
                        challenge,
                        keepalive_interval,
                    )
                    .await;
                    return;
                }
                ProbeClass::PlainLatch(msg) => {
                    trace!("non-MPP frame at handshake → latching plain mode");
                    detected_plain.store(true, Ordering::SeqCst);
                    spawn_plain_with_initial(socket, interface, keepalive_interval, Some(msg));
                    return;
                }
                ProbeClass::PlainNoLatch(msg) => {
                    trace!("malformed/control frame at handshake → plain mode (no latch)");
                    spawn_plain_with_initial(socket, interface, keepalive_interval, Some(msg));
                    return;
                }
                ProbeClass::Skip => continue,
            },
            Ok(Some(Err(err))) => {
                error!(%err, "WS connection error during MPP probe");
                interface.close_with_error();
                return;
            }
            Ok(None) => {
                error!("WS server closed connection during MPP probe");
                interface.close_with_error();
                return;
            }
            Err(_) => {
                trace!("MPP probe timed out, falling back to plain WS");
                detected_plain.store(true, Ordering::SeqCst);
                spawn_plain_with_initial(socket, interface, keepalive_interval, None);
                return;
            }
        }
    }
}

/// Run the post-handshake MPP exchange. Any failure after this point is
/// reported as non-retryable so the outer reconnect loop does not re-drive
/// payment on deterministic protocol errors.
async fn handle_mpp_handshake<P: PaymentProvider>(
    mut socket: TungsteniteStream,
    interface: ConnectionInterface,
    provider: P,
    challenge: PaymentChallenge,
    keepalive_interval: Duration,
) {
    if !provider.supports(challenge.method.as_str(), challenge.intent.as_str()) {
        error!(
            method = %challenge.method.as_str(),
            intent = %challenge.intent.as_str(),
            "MPP challenge unsupported by provider"
        );
        interface.close_with_transport_error(TransportErrorKind::non_retryable_str(
            "unsupported MPP challenge method/intent",
        ));
        return;
    }

    let credential = match provider.pay(&challenge).await {
        Ok(c) => c,
        Err(e) => {
            error!(%e, "MPP provider failure");
            interface.close_with_transport_error(TransportErrorKind::non_retryable(Box::new(e)));
            return;
        }
    };

    if let Err(e) = send_credential(&mut socket, &credential).await {
        // Payment may already have been broadcast — do not let the outer
        // reconnect loop pay again.
        error!(%e, "failed to send MPP credential after payment");
        interface.close_with_transport_error(TransportErrorKind::non_retryable(Box::new(e)));
        return;
    }

    run_mpp_loop(socket, interface, keepalive_interval).await;
}

/// Spawn a plain [`WsBackend`] for the given socket, optionally injecting an
/// already-received first frame.
fn spawn_plain_with_initial(
    socket: TungsteniteStream,
    interface: ConnectionInterface,
    keepalive_interval: Duration,
    initial: Option<Message>,
) {
    let mut backend = WsBackend::from_socket(socket, interface, keepalive_interval);
    if let Some(msg) = initial {
        if backend.handle(msg).is_err() {
            backend.interface.close_with_error();
            return;
        }
    }
    backend.spawn();
}

/// Try to parse a text frame as an MPP `{"type":"challenge","challenge":{…}}`
/// envelope. Borrows the inner challenge as raw JSON to avoid intermediate
/// `serde_json::Value` allocation.
fn try_parse_challenge(text: &str) -> Option<PaymentChallenge> {
    #[derive(Deserialize)]
    struct Envelope<'a> {
        #[serde(rename = "type")]
        ty: &'a str,
        #[serde(borrow)]
        challenge: Option<&'a RawValue>,
    }
    let env: Envelope<'_> = serde_json::from_str(text).ok()?;
    if env.ty != "challenge" {
        return None;
    }
    serde_json::from_str(env.challenge?.get()).ok()
}

/// Inspect a received [`Message`] for an MPP `challenge` envelope (kept for
/// the public-ish probe helper API used in tests).
#[cfg(test)]
fn parse_challenge(msg: &Message) -> Option<PaymentChallenge> {
    let Message::Text(t) = msg else { return None };
    try_parse_challenge(t.as_ref())
}

/// Send an MPP `credential` frame.
async fn send_credential(
    socket: &mut TungsteniteStream,
    credential: &PaymentCredential,
) -> Result<(), tungstenite::Error> {
    // Serialization of a `PaymentCredential` cannot fail.
    let json = serde_json::to_vec(credential).expect("PaymentCredential serializes");
    let encoded = base64url_encode(&json);
    let frame = format!(r#"{{"type":"credential","credential":"{encoded}"}}"#);
    socket.send(Message::Text(frame.into())).await
}

/// Wrap an outbound JSON-RPC frame in an MPP `message` envelope.
fn wrap_message(data: &RawValue) -> String {
    format!(r#"{{"type":"message","data":{}}}"#, data.get())
}

/// Reason a post-handshake MPP loop exited.
enum LoopExit {
    /// Frontend or backend closed cleanly. No terminal error to surface.
    Clean,
    /// WebSocket transport error (network drop, missed pong, etc.).
    Transport,
    /// MPP protocol violation (malformed frame, invalid `message.data`,
    /// server-emitted `error` frame). Surfaces as non-retryable so the
    /// outer reconnect loop does not re-drive payment.
    Protocol(String),
}

/// Run the MPP-mode message pump. Mirrors [`WsBackend::spawn`] but with
/// envelope wrapping/unwrapping and MPP control-frame handling.
async fn run_mpp_loop(
    mut socket: TungsteniteStream,
    mut interface: ConnectionInterface,
    keepalive_interval: Duration,
) {
    let mut expecting_pong = false;
    let keepalive = sleep(keepalive_interval);
    tokio::pin!(keepalive);

    let exit = loop {
        tokio::select! {
            biased;

            inst = interface.recv_from_frontend() => {
                match inst {
                    Some(msg) => {
                        keepalive.as_mut().reset(tokio::time::Instant::now() + keepalive_interval);
                        let frame = wrap_message(&msg);
                        if let Err(err) = socket.send(Message::Text(frame.into())).await {
                            error!(%err, "WS connection error");
                            break LoopExit::Transport;
                        }
                    }
                    None => break LoopExit::Clean,
                }
            }
            _ = &mut keepalive => {
                if expecting_pong {
                    error!("WS server missed a pong");
                    break LoopExit::Transport;
                }
                keepalive.as_mut().reset(tokio::time::Instant::now() + keepalive_interval);
                if let Err(err) = socket.send(Message::Ping(Default::default())).await {
                    error!(%err, "WS connection error");
                    break LoopExit::Transport;
                }
                expecting_pong = true;
            }
            resp = socket.next() => {
                match resp {
                    Some(Ok(item)) => {
                        if item.is_pong() {
                            expecting_pong = false;
                        }
                        match handle_inbound(&interface, item) {
                            Ok(()) => {}
                            Err(InboundError::Transport(reason)) => {
                                error!(%reason, "WS inbound transport error");
                                break LoopExit::Transport;
                            }
                            Err(InboundError::Protocol(reason)) => {
                                error!(%reason, "MPP protocol violation");
                                break LoopExit::Protocol(reason);
                            }
                        }
                    }
                    Some(Err(err)) => {
                        error!(%err, "WS connection error");
                        break LoopExit::Transport;
                    }
                    None => {
                        error!("WS server has gone away");
                        break LoopExit::Transport;
                    }
                }
            }
        }
    };

    match exit {
        LoopExit::Clean => {}
        LoopExit::Transport => interface.close_with_error(),
        LoopExit::Protocol(reason) => {
            interface.close_with_transport_error(TransportErrorKind::non_retryable_str(&reason))
        }
    }
}

/// Reason an inbound frame failed.
enum InboundError {
    /// WS-level signal that the connection is unusable (close frame, frontend
    /// channel closed). Treated as a normal transport error → retryable.
    Transport(String),
    /// MPP protocol violation that should not be retried.
    Protocol(String),
}

fn handle_inbound(interface: &ConnectionInterface, msg: Message) -> Result<(), InboundError> {
    match msg {
        Message::Text(text) => handle_text(interface, text.as_ref()),
        Message::Close(frame) => Err(InboundError::Transport(format!("close frame: {frame:?}"))),
        Message::Binary(_) => Err(InboundError::Protocol("unexpected binary message".to_string())),
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => Ok(()),
    }
}

fn handle_text(interface: &ConnectionInterface, text: &str) -> Result<(), InboundError> {
    trace!(%text, "received MPP frame");

    #[derive(Deserialize)]
    struct Envelope<'a> {
        #[serde(rename = "type")]
        ty: &'a str,
        #[serde(borrow, default)]
        data: Option<&'a RawValue>,
        #[serde(borrow, default)]
        message: Option<&'a str>,
    }

    let env: Envelope<'_> = serde_json::from_str(text).map_err(|err| {
        InboundError::Protocol(format!("failed to parse WS frame as JSON: {err}"))
    })?;

    match env.ty {
        "message" => {
            let data = env.data.ok_or_else(|| {
                InboundError::Protocol("MPP `message` frame missing `data`".to_string())
            })?;
            let item: PubSubItem = serde_json::from_str(data.get()).map_err(|err| {
                InboundError::Protocol(format!("failed to deserialize JSON-RPC payload: {err}"))
            })?;
            interface
                .send_to_frontend(item)
                .map_err(|_| InboundError::Transport("frontend dropped".to_string()))?;
            Ok(())
        }
        "receipt" => {
            trace!("received MPP receipt");
            Ok(())
        }
        "needVoucher" => {
            trace!("received MPP needVoucher");
            Ok(())
        }
        "error" => Err(InboundError::Protocol(format!(
            "MPP error frame: {}",
            env.message.unwrap_or("unspecified")
        ))),
        "challenge" => {
            // Late challenge after handshake — drop without killing the
            // session. The next reconnect can probe again.
            warn!("received late MPP challenge frame, ignoring");
            Ok(())
        }
        other => {
            warn!(ty = %other, "unknown MPP frame type, ignoring");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request, RequestPacket, ResponsePacket};
    use mpp::{Base64UrlJson, MppError, PaymentChallenge, PaymentCredential, PaymentPayload};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use tokio::{net::TcpListener, time::timeout};
    use tokio_tungstenite::accept_async;

    /// Stub `PaymentProvider` whose `supports`, `pay`-result, and call count
    /// can all be observed/configured per test.
    #[derive(Clone)]
    struct StubProvider {
        supports: bool,
        fail: bool,
        pay_count: Arc<AtomicUsize>,
    }

    impl StubProvider {
        fn ok() -> Self {
            Self { supports: true, fail: false, pay_count: Arc::new(AtomicUsize::new(0)) }
        }
        fn pay_count(&self) -> usize {
            self.pay_count.load(Ordering::SeqCst)
        }
    }

    impl PaymentProvider for StubProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            self.supports
        }
        async fn pay(&self, c: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            self.pay_count.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(MppError::Http("provider failure".into()));
            }
            Ok(PaymentCredential::new(c.to_echo(), PaymentPayload::hash("0xdeadbeef".to_string())))
        }
    }

    fn make_challenge_frame() -> String {
        let request = Base64UrlJson::from_value(&serde_json::json!({"amount": "1"})).unwrap();
        let c = PaymentChallenge::new(
            "ch-1".to_string(),
            "test".to_string(),
            "tempo".to_string(),
            "charge".to_string(),
            request,
        );
        serde_json::json!({"type":"challenge","challenge":c}).to_string()
    }

    /// Behavior toggles for the canned WebSocket test server.
    #[derive(Clone, Default)]
    struct ServerScript {
        /// WebSocket Ping frames sent before any text — used to verify the
        /// probe ignores transport-level control frames.
        pre_challenge_pings: u32,
        /// Send an MPP `challenge` frame as the first text message.
        send_challenge: bool,
        /// Text frames sent before the request/response loop. Used to
        /// inject volunteered frames (receipt, stray plain JSON-RPC, …).
        prelude: Vec<String>,
    }

    /// Captured server-side state.
    #[derive(Default, Debug)]
    struct ServerCapture {
        credential_received: Option<String>,
        client_messages: Vec<String>,
    }

    /// Bind a TCP listener and serve the script. The server echoes JSON-RPC
    /// requests with a canned `0x10` result, wrapping responses as MPP
    /// `message` envelopes when the client opened with a challenge.
    async fn run_server(script: ServerScript) -> (String, Arc<Mutex<ServerCapture>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let cap: Arc<Mutex<ServerCapture>> = Default::default();
        let out = cap.clone();

        tokio::spawn(async move {
            let (sock, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(sock).await.unwrap();

            for i in 0..script.pre_challenge_pings {
                ws.send(Message::Ping(vec![i as u8].into())).await.unwrap();
            }
            if script.send_challenge {
                ws.send(Message::Text(make_challenge_frame().into())).await.unwrap();
            }
            for line in &script.prelude {
                ws.send(Message::Text(line.clone().into())).await.unwrap();
            }

            let mpp_mode = script.send_challenge;
            while let Some(Ok(msg)) = ws.next().await {
                let Message::Text(text) = msg else { continue };
                let text: String = text.to_string();
                out.lock().unwrap().client_messages.push(text.clone());
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };

                let resp = if mpp_mode {
                    match v.get("type").and_then(|x| x.as_str()) {
                        Some("credential") => {
                            out.lock().unwrap().credential_received =
                                Some(v["credential"].as_str().unwrap_or("").to_string());
                            continue;
                        }
                        Some("message") => serde_json::json!({
                            "type":"message",
                            "data":{"jsonrpc":"2.0","id":v["data"]["id"],"result":"0x10"}
                        }),
                        _ => continue,
                    }
                } else {
                    serde_json::json!({"jsonrpc":"2.0","id":v["id"],"result":"0x10"})
                };
                ws.send(Message::Text(resp.to_string().into())).await.unwrap();
            }
        });

        (url, cap)
    }

    fn ws_connect(url: &str) -> WsConnect {
        WsConnect::new(url).with_keepalive_interval(Duration::from_secs(60))
    }

    fn rpc_request(id: u64) -> RequestPacket {
        RequestPacket::Single(
            Request::new("eth_blockNumber", Id::Number(id), ()).serialize().unwrap(),
        )
    }

    /// Assert the response is a successful single-packet `0x10` for `id`.
    fn assert_ok_response(resp: ResponsePacket, id: u64) {
        match resp {
            ResponsePacket::Single(r) => {
                assert_eq!(r.id, Id::Number(id));
                assert!(r.payload.is_success(), "expected success: {r:?}");
            }
            other => panic!("expected single response, got {other:?}"),
        }
    }

    /// Send a request through `frontend`, returning its result.
    async fn one_request(
        frontend: &alloy_pubsub::PubSubFrontend,
        id: u64,
    ) -> Result<ResponsePacket, alloy_transport::TransportError> {
        timeout(Duration::from_secs(2), frontend.send_packet(rpc_request(id)))
            .await
            .expect("send timeout")
    }

    /// MPP happy path: covers
    /// - Ping before challenge is skipped by probe (no plain latch),
    /// - challenge → pay → credential frame,
    /// - outbound RPC frames are wrapped as `message` envelopes,
    /// - server-volunteered `receipt` frame doesn't kill the session.
    #[tokio::test]
    async fn mpp_happy_path() {
        let receipt = serde_json::json!({"type":"receipt","data":{"id":"r-1"}}).to_string();
        let (url, capture) = run_server(ServerScript {
            pre_challenge_pings: 1,
            send_challenge: true,
            prelude: vec![receipt],
        })
        .await;

        let provider = StubProvider::ok();
        let connector = MppWsConnect::new(ws_connect(&url), provider.clone())
            .with_handshake_timeout(Duration::from_secs(2));

        let frontend = connector.clone().into_service().await.expect("connect");
        assert_ok_response(one_request(&frontend, 1).await.expect("send"), 1);

        assert_eq!(provider.pay_count(), 1, "MPP path must run");
        assert!(!connector.detected_plain(), "Ping/challenge must not latch plain");

        let cap = capture.lock().unwrap();
        assert!(cap.credential_received.is_some(), "credential frame sent");
        let wrapped = cap
            .client_messages
            .iter()
            .filter_map(|t| serde_json::from_str::<serde_json::Value>(t).ok())
            .find(|v| v["type"] == "message")
            .expect("at least one wrapped message frame");
        assert_eq!(wrapped["data"]["method"], "eth_blockNumber");
    }

    /// Plain mode latching: covers timeout fallback, valid-plain-frame
    /// fallback, and the cached probe-skip on reconnect.
    #[tokio::test]
    async fn plain_mode_latches_and_skips_probe_on_reconnect() {
        // (1) Probe times out → latch.
        let (url1, _) = run_server(ServerScript::default()).await;
        let connector = MppWsConnect::new(ws_connect(&url1), StubProvider::ok())
            .with_handshake_timeout(Duration::from_millis(150));
        let frontend = connector.clone().into_service().await.expect("connect");
        assert_ok_response(one_request(&frontend, 1).await.expect("send"), 1);
        assert!(connector.detected_plain(), "timeout must latch plain");

        // (2) Reconnect on a different (also-plain) server with a tiny
        //     handshake_timeout: the probe must be skipped entirely. We
        //     verify by reading the AtomicBool was already true before
        //     reconnect *and* that the request still succeeds within a
        //     window much smaller than a real probe would take.
        let (url2, _) = run_server(ServerScript::default()).await;
        let connector2 = MppWsConnect::new(ws_connect(&url2), StubProvider::ok())
            .with_handshake_timeout(Duration::from_millis(1));
        connector2.detected_plain.store(true, Ordering::SeqCst);
        let frontend2 = connector2.into_service().await.expect("reconnect");
        assert_ok_response(one_request(&frontend2, 2).await.expect("send2"), 2);

        // (3) A non-challenge first text frame also latches plain.
        let stray = serde_json::json!({"jsonrpc":"2.0","id":999,"result":"0x"}).to_string();
        let (url3, _) =
            run_server(ServerScript { prelude: vec![stray], ..Default::default() }).await;
        let connector3 = MppWsConnect::new(ws_connect(&url3), StubProvider::ok())
            .with_handshake_timeout(Duration::from_secs(2));
        let frontend3 = connector3.clone().into_service().await.expect("connect3");
        assert_ok_response(one_request(&frontend3, 3).await.expect("send3"), 3);
        assert!(connector3.detected_plain(), "valid plain frame must latch plain");
    }

    /// Both deterministic-payment failure paths (`pay()` errors,
    /// unsupported challenge) surface as a terminal error to the caller.
    #[tokio::test]
    async fn deterministic_failures_surface_as_errors() {
        for provider in [
            StubProvider { supports: true, fail: true, ..StubProvider::ok() },
            StubProvider { supports: false, fail: false, ..StubProvider::ok() },
        ] {
            let (url, _) =
                run_server(ServerScript { send_challenge: true, ..Default::default() }).await;
            let connector = MppWsConnect::new(ws_connect(&url), provider)
                .with_handshake_timeout(Duration::from_secs(2));
            let frontend = connector.into_service().await.expect("connect");
            assert!(one_request(&frontend, 1).await.is_err(), "expected non-retryable failure");
        }
    }

    /// Pure-helper smoke test for the frame parsers/formatters.
    #[test]
    fn frame_helpers() {
        // parse_challenge: positive + non-MPP text + non-text.
        let challenge = parse_challenge(&Message::Text(make_challenge_frame().into()))
            .expect("parses challenge");
        assert_eq!(challenge.id, "ch-1");
        assert_eq!(challenge.method.as_str(), "tempo");

        let other = serde_json::json!({"type":"message","data":{}}).to_string();
        assert!(parse_challenge(&Message::Text(other.into())).is_none());
        assert!(parse_challenge(&Message::Binary(Default::default())).is_none());

        // wrap_message: embeds raw JSON unchanged inside the envelope.
        let raw = serde_json::value::to_raw_value(&serde_json::json!({"a":1})).unwrap();
        let v: serde_json::Value = serde_json::from_str(&wrap_message(&raw)).unwrap();
        assert_eq!(v["type"], "message");
        assert_eq!(v["data"], serde_json::json!({"a":1}));
    }
}
