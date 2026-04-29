//! MPP WebSocket transport for alloy.
//!
//! Wraps every outgoing JSON-RPC frame in an MPP `message` envelope and
//! intercepts inbound `challenge`, `needVoucher`, `receipt`, and `error`
//! frames to drive a user-supplied [`PaymentProvider`] (and optional
//! [`VoucherProvider`] for streaming sessions).

use alloy_json_rpc::PubSubItem;
use alloy_pubsub::{ConnectionHandle, ConnectionInterface, PubSubConnect};
use alloy_transport::{
    utils::{guess_local_url, Spawnable},
    Authorization, TransportErrorKind, TransportResult,
};
use alloy_transport_ws::WebSocketConfig;
use futures::{SinkExt, StreamExt};
use http::{header::AUTHORIZATION, HeaderValue, Request as HttpRequest};
use mpp::{
    client::{
        ws::{WsClientMessage, WsServerMessage},
        PaymentProvider,
    },
    format_authorization, MppError, PaymentChallenge, PaymentCredential, Receipt,
};
use serde_json::value::RawValue;
use std::{
    fmt,
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{
        broadcast::{self, Receiver as BroadcastReceiver},
        watch::{self, Receiver as WatchReceiver},
    },
    task::{JoinError, JoinHandle},
    time::{sleep, Instant},
};
use tokio_tungstenite::{
    tungstenite::{self, client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

const DEFAULT_KEEPALIVE_SECS: u64 = 10;
const DEFAULT_HANDSHAKE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_EVENTS_CAPACITY: usize = 64;

/// Server-issued request for a fresh session voucher.
///
/// Emitted as part of an MPP `needVoucher` frame when a streaming session's
/// channel balance has been exhausted and the server is asking the client
/// for a new signed cumulative voucher.
#[derive(Clone, Debug)]
pub struct VoucherRequest {
    /// Channel ID (`"0x…"`).
    pub channel_id: String,
    /// Cumulative amount the server needs the next voucher to cover.
    pub required_cumulative: String,
    /// Cumulative amount the server has already accepted vouchers for.
    pub accepted_cumulative: String,
    /// Total channel deposit on-chain.
    pub deposit: String,
}

/// Optional companion to [`PaymentProvider`] for streaming/session intents.
///
/// When the server emits a `needVoucher` frame, the translator calls
/// [`VoucherProvider::next_voucher`] to obtain the next signed voucher.
/// The default [`NoVoucher`] implementation rejects all voucher requests,
/// which is the right behavior for charge-only flows.
pub trait VoucherProvider: Clone + Send + Sync + 'static {
    /// Produce a fresh voucher credential for the given session request.
    fn next_voucher(
        &self,
        request: &VoucherRequest,
    ) -> impl Future<Output = Result<PaymentCredential, MppError>> + Send;
}

/// Default no-op [`VoucherProvider`].
///
/// Returns an error for every voucher request. Use this when your
/// `PaymentProvider` only handles one-shot `charge` intents.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoVoucher;

impl VoucherProvider for NoVoucher {
    async fn next_voucher(&self, _: &VoucherRequest) -> Result<PaymentCredential, MppError> {
        Err(MppError::bad_request(
            "voucher provider not configured; configure MppWsConnect::with_voucher_provider",
        ))
    }
}

/// Significant MPP events observed by the translator.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum MppEvent {
    /// A challenge was received from the server.
    Challenge(PaymentChallenge),
    /// A credential was successfully sent in response to a challenge.
    CredentialSent,
    /// A `needVoucher` frame was received.
    NeedVoucher(VoucherRequest),
    /// A voucher credential was sent.
    VoucherSent,
    /// A receipt was received.
    Receipt(Receipt),
    /// The server emitted an error frame.
    Error(String),
}

/// Side channel for observing MPP-specific events on a connection.
///
/// Obtain one with [`MppWsConnect::mpp_handle`] **before** calling
/// `connect_pubsub`/`connect_client`. Receivers are persistent across
/// reconnects — the senders live on `MppWsConnect` itself, and every
/// successive translator task publishes into the same channels.
#[derive(Debug)]
pub struct MppHandle {
    /// The most recently observed receipt (`None` until the first one).
    pub receipt: WatchReceiver<Option<Receipt>>,
    /// Stream of MPP events; lossy under backpressure (uses
    /// [`tokio::sync::broadcast`]).
    pub events: BroadcastReceiver<MppEvent>,
}

/// Reason a translator session ended; controls whether the pubsub layer
/// reconnects.
#[derive(Clone, Copy, Debug)]
enum TerminationReason {
    /// Socket-level failure. Reconnect using the configured retry policy.
    Transient,
    /// Deterministic MPP failure (provider error, server `error` frame,
    /// malformed/unexpected frame). Do not reconnect.
    Fatal,
}

/// Connection details for an MPP-over-WebSocket transport.
///
/// `MppWsConnect` is a drop-in [`PubSubConnect`] that speaks the
/// [MPP WS wire protocol](https://github.com/tempoxyz/mpp-rs). Outbound
/// JSON-RPC frames are wrapped as `{"type":"message","data":<payload>}`;
/// server-issued `challenge` / `needVoucher` frames are settled by the
/// supplied [`PaymentProvider`] / [`VoucherProvider`].
///
/// On reconnect, [`PaymentProvider::pay`] is called again with the server's
/// new challenge — providers should handle repeated calls cheaply.
///
/// Only socket-level failures are retried; deterministic MPP failures are
/// terminal and short-circuit further reconnect attempts.
#[derive(Clone)]
pub struct MppWsConnect<P, V = NoVoucher> {
    url: String,
    auth: Option<Authorization>,
    config: Option<WebSocketConfig>,
    max_retries: u32,
    retry_interval: Duration,
    keepalive_interval: Duration,
    handshake_timeout: Duration,
    payment_provider: P,
    voucher_provider: V,
    receipt_tx: watch::Sender<Option<Receipt>>,
    events_tx: broadcast::Sender<MppEvent>,
    /// Latched on fatal MPP failure; short-circuits future `connect()` calls.
    fatal: Arc<AtomicBool>,
}

// Manual Debug impl: redact secrets in `url` and `auth`; omit providers
// and internal channels.
impl<P, V> fmt::Debug for MppWsConnect<P, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MppWsConnect")
            .field("url", &redact_url_userinfo(&self.url))
            .field("auth", &self.auth.as_ref().map(|_| "<redacted>"))
            .field("max_retries", &self.max_retries)
            .field("retry_interval", &self.retry_interval)
            .field("keepalive_interval", &self.keepalive_interval)
            .field("handshake_timeout", &self.handshake_timeout)
            .finish_non_exhaustive()
    }
}

/// Strip `user:pass@` from a URL string for safe logging.
fn redact_url_userinfo(raw: &str) -> String {
    match Url::parse(raw) {
        Ok(mut u) if !u.username().is_empty() || u.password().is_some() => {
            let _ = u.set_username("");
            let _ = u.set_password(None);
            u.to_string()
        }
        Ok(_) => raw.to_string(),
        Err(_) => "<unparseable>".to_string(),
    }
}

impl<P> MppWsConnect<P, NoVoucher> {
    /// Create a new MPP WebSocket connector.
    ///
    /// If the URL contains userinfo (`wss://user:pass@host`), it is extracted
    /// as a basic `Authorization` header for the WS handshake.
    pub fn new<S: Into<String>>(url: S, payment_provider: P) -> Self {
        let url = url.into();
        let auth =
            Url::parse(&url).ok().and_then(|parsed| Authorization::extract_from_url(&parsed));
        let (receipt_tx, _) = watch::channel(None);
        let (events_tx, _) = broadcast::channel(DEFAULT_EVENTS_CAPACITY);
        Self {
            url,
            auth,
            config: None,
            max_retries: 10,
            retry_interval: Duration::from_secs(3),
            keepalive_interval: Duration::from_secs(DEFAULT_KEEPALIVE_SECS),
            handshake_timeout: Duration::from_secs(DEFAULT_HANDSHAKE_TIMEOUT_SECS),
            payment_provider,
            voucher_provider: NoVoucher,
            receipt_tx,
            events_tx,
            fatal: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<P, V> MppWsConnect<P, V> {
    /// Sets the authorization header for the underlying WS handshake.
    pub fn with_auth(mut self, auth: Authorization) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Sets the websocket config.
    pub const fn with_config(mut self, config: WebSocketConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Sets the max number of reconnect retries.
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the base reconnect retry interval.
    pub const fn with_retry_interval(mut self, retry_interval: Duration) -> Self {
        self.retry_interval = retry_interval;
        self
    }

    /// Sets the keepalive ping interval.
    pub const fn with_keepalive_interval(mut self, keepalive_interval: Duration) -> Self {
        self.keepalive_interval = keepalive_interval;
        self
    }

    /// Sets the per-phase handshake timeout.
    ///
    /// Bounds the time spent waiting for the server's `challenge` /
    /// `needVoucher` and for the local `pay()` / `next_voucher()` to complete.
    /// On expiry the connection is closed with a fatal error and not
    /// reconnected.
    pub const fn with_handshake_timeout(mut self, handshake_timeout: Duration) -> Self {
        self.handshake_timeout = handshake_timeout;
        self
    }

    /// Plug in a [`VoucherProvider`] for streaming/session intents.
    pub fn with_voucher_provider<V2: VoucherProvider>(
        self,
        voucher_provider: V2,
    ) -> MppWsConnect<P, V2> {
        MppWsConnect {
            url: self.url,
            auth: self.auth,
            config: self.config,
            max_retries: self.max_retries,
            retry_interval: self.retry_interval,
            keepalive_interval: self.keepalive_interval,
            handshake_timeout: self.handshake_timeout,
            payment_provider: self.payment_provider,
            voucher_provider,
            receipt_tx: self.receipt_tx,
            events_tx: self.events_tx,
            fatal: self.fatal,
        }
    }

    /// Get a side-channel handle to observe receipts and MPP events.
    ///
    /// Call this **before** handing the connector to a `ProviderBuilder`
    /// or `ClientBuilder`. The receivers stay valid across reconnects.
    pub fn mpp_handle(&self) -> MppHandle {
        MppHandle { receipt: self.receipt_tx.subscribe(), events: self.events_tx.subscribe() }
    }

    /// Get the URL of the connection.
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl<P: Clone, V: Clone> IntoClientRequest for MppWsConnect<P, V> {
    fn into_client_request(self) -> tungstenite::Result<tungstenite::handshake::client::Request> {
        let mut request: HttpRequest<()> = self.url.into_client_request()?;
        if let Some(auth) = self.auth {
            let mut auth_value = HeaderValue::from_str(&auth.to_string())?;
            auth_value.set_sensitive(true);
            request.headers_mut().insert(AUTHORIZATION, auth_value);
        }
        request.into_client_request()
    }
}

impl<P, V> PubSubConnect for MppWsConnect<P, V>
where
    P: PaymentProvider + 'static,
    V: VoucherProvider,
{
    fn is_local(&self) -> bool {
        guess_local_url(&self.url)
    }

    async fn connect(&self) -> TransportResult<ConnectionHandle> {
        // Refuse to reconnect after a fatal MPP failure.
        if self.fatal.load(Ordering::Acquire) {
            return Err(TransportErrorKind::custom_str(
                "MPP connection terminated by a fatal error; not reconnecting",
            ));
        }

        #[cfg(any(feature = "aws-lc-rs", feature = "ring"))]
        install_default_crypto_provider();

        let request = self.clone().into_client_request().map_err(TransportErrorKind::custom)?;
        let (socket, _) = tokio_tungstenite::connect_async_with_config(request, self.config, false)
            .await
            .map_err(TransportErrorKind::custom)?;

        let (handle, interface) = ConnectionHandle::new();

        run_translator(
            socket,
            interface,
            self.payment_provider.clone(),
            self.voucher_provider.clone(),
            self.keepalive_interval,
            self.handshake_timeout,
            self.receipt_tx.clone(),
            self.events_tx.clone(),
            self.fatal.clone(),
        )
        .spawn_task();

        Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
    }
}

/// Bridge between alloy's JSON-RPC channels and MPP frames.
#[allow(clippy::too_many_arguments)]
async fn run_translator<P, V>(
    mut socket: WsStream,
    mut interface: ConnectionInterface,
    payment_provider: P,
    voucher_provider: V,
    keepalive_interval: Duration,
    handshake_timeout: Duration,
    receipt_tx: watch::Sender<Option<Receipt>>,
    events_tx: broadcast::Sender<MppEvent>,
    fatal: Arc<AtomicBool>,
) where
    P: PaymentProvider + 'static,
    V: VoucherProvider,
{
    let mut termination: Option<TerminationReason> = None;
    let mut expecting_pong = false;
    // `false` until the first credential has been sent; gates outbound JSON-RPC
    // so application traffic never races the initial challenge.
    let mut handshake_complete = false;
    let keepalive = sleep(keepalive_interval);
    tokio::pin!(keepalive);

    // Bounds time spent waiting for a challenge or a pay()/next_voucher()
    // task to complete.
    let handshake_deadline = sleep(handshake_timeout);
    tokio::pin!(handshake_deadline);

    // Off-loop tasks: spawning `pay`/`next_voucher` keeps the keepalive arm
    // responsive while signing/broadcasting takes place.
    let mut pending_pay: Option<JoinHandle<Result<PaymentCredential, MppError>>> = None;
    let mut pending_voucher: Option<JoinHandle<Result<PaymentCredential, MppError>>> = None;

    loop {
        let payment_in_flight = pending_pay.is_some() || pending_voucher.is_some();
        let frontend_open = handshake_complete && !payment_in_flight;

        tokio::select! {
            biased;

            // 1. Outbound JSON-RPC from the alloy frontend → MPP `message` frame.
            //    Held while a credential exchange is in flight, so RPC never
            //    races a challenge.
            inst = interface.recv_from_frontend(), if frontend_open => {
                match inst {
                    Some(rpc) => {
                        keepalive.as_mut().reset(Instant::now() + keepalive_interval);
                        if let Err(err) = send_jsonrpc(&mut socket, rpc).await {
                            error!(%err, "WS connection error");
                            termination = Some(TerminationReason::Transient);
                            break;
                        }
                    }
                    None => break,
                }
            }

            // 2. Pay task completion.
            res = poll_join(&mut pending_pay), if pending_pay.is_some() => {
                pending_pay = None;
                match res {
                    Ok(Ok(cred)) => {
                        if let Err(()) = send_credential(&mut socket, &cred, &events_tx, false).await {
                            termination = Some(TerminationReason::Transient);
                            break;
                        }
                        handshake_complete = true;
                    }
                    Ok(Err(err)) => {
                        error!(?err, "MPP payment provider failed");
                        let _ = events_tx.send(MppEvent::Error(format!(
                            "{err}; not reconnecting"
                        )));
                        termination = Some(TerminationReason::Fatal);
                        break;
                    }
                    Err(join_err) => {
                        error!(%join_err, "MPP payment task panicked");
                        let _ = events_tx.send(MppEvent::Error(format!(
                            "payment task panicked: {join_err}; not reconnecting"
                        )));
                        termination = Some(TerminationReason::Fatal);
                        break;
                    }
                }
            }

            // 3. Voucher task completion.
            res = poll_join(&mut pending_voucher), if pending_voucher.is_some() => {
                pending_voucher = None;
                match res {
                    Ok(Ok(cred)) => {
                        if let Err(()) = send_credential(&mut socket, &cred, &events_tx, true).await {
                            termination = Some(TerminationReason::Transient);
                            break;
                        }
                        handshake_complete = true;
                    }
                    Ok(Err(err)) => {
                        error!(?err, "MPP voucher provider failed");
                        let _ = events_tx.send(MppEvent::Error(format!(
                            "{err}; not reconnecting"
                        )));
                        termination = Some(TerminationReason::Fatal);
                        break;
                    }
                    Err(join_err) => {
                        error!(%join_err, "MPP voucher task panicked");
                        let _ = events_tx.send(MppEvent::Error(format!(
                            "voucher task panicked: {join_err}; not reconnecting"
                        )));
                        termination = Some(TerminationReason::Fatal);
                        break;
                    }
                }
            }

            // 4. Keepalive ping.
            _ = &mut keepalive => {
                if expecting_pong {
                    error!("WS server missed a pong");
                    termination = Some(TerminationReason::Transient);
                    break;
                }
                keepalive.as_mut().reset(Instant::now() + keepalive_interval);
                if let Err(err) = socket.send(Message::Ping(Default::default())).await {
                    error!(%err, "WS connection error");
                    termination = Some(TerminationReason::Transient);
                    break;
                }
                expecting_pong = true;
            }

            // 5. Handshake / payment timeout. Only armed while not `frontend_open`.
            _ = &mut handshake_deadline, if !frontend_open => {
                error!("MPP handshake timed out");
                let _ = events_tx.send(MppEvent::Error(
                    "MPP handshake timed out; not reconnecting".to_string(),
                ));
                termination = Some(TerminationReason::Fatal);
                break;
            }

            // 6. Inbound from socket → MPP frame.
            resp = socket.next() => {
                let pay_was_pending = pending_pay.is_some();
                let voucher_was_pending = pending_voucher.is_some();
                match resp {
                    Some(Ok(item)) => {
                        if item.is_pong() {
                            expecting_pong = false;
                        }
                        match handle_message(
                            item,
                            &interface,
                            &payment_provider,
                            &voucher_provider,
                            &receipt_tx,
                            &events_tx,
                            &mut pending_pay,
                            &mut pending_voucher,
                        ).await {
                            Ok(()) => {
                                // Reset the handshake deadline when a new
                                // pay()/next_voucher() task is spawned.
                                let entered_pay = !pay_was_pending && pending_pay.is_some();
                                let entered_voucher =
                                    !voucher_was_pending && pending_voucher.is_some();
                                if entered_pay || entered_voucher {
                                    handshake_deadline
                                        .as_mut()
                                        .reset(Instant::now() + handshake_timeout);
                                }
                            }
                            Err(reason) => {
                                termination = Some(reason);
                                break;
                            }
                        }
                    }
                    Some(Err(err)) => {
                        error!(%err, "WS connection error");
                        termination = Some(TerminationReason::Transient);
                        break;
                    }
                    None => {
                        error!("WS server has gone away");
                        termination = Some(TerminationReason::Transient);
                        break;
                    }
                }
            }
        }
    }

    // Cancel any background tasks still running on shutdown.
    if let Some(h) = pending_pay {
        h.abort();
    }
    if let Some(h) = pending_voucher {
        h.abort();
    }

    match termination {
        Some(TerminationReason::Transient) => interface.close_with_error(),
        Some(TerminationReason::Fatal) => {
            // Latch so subsequent `connect()` calls short-circuit.
            fatal.store(true, Ordering::Release);
            interface.close_with_error();
        }
        None => {}
    }
}

/// Wait on a possibly-`None` `JoinHandle`. Caller must guard with
/// `pending.is_some()` in the surrounding `select!`.
async fn poll_join<T>(pending: &mut Option<JoinHandle<T>>) -> Result<T, JoinError> {
    pending.as_mut().expect("guarded by select! `if` clause").await
}

async fn send_jsonrpc(socket: &mut WsStream, rpc: Box<RawValue>) -> Result<(), tungstenite::Error> {
    // Parse the JSON-RPC envelope so it serializes back as a structured object
    // (matching mpp-rs's `WsClientMessage::Data { data: serde_json::Value }`).
    let data: serde_json::Value = serde_json::from_str(rpc.get())
        .map_err(|e| tungstenite::Error::Io(std::io::Error::other(e)))?;
    let frame = WsClientMessage::Data { data };
    socket.send(Message::Text(frame.to_text().into())).await
}

async fn send_credential(
    socket: &mut WsStream,
    cred: &PaymentCredential,
    events_tx: &broadcast::Sender<MppEvent>,
    is_voucher: bool,
) -> Result<(), ()> {
    let auth = match format_authorization(cred) {
        Ok(s) => s,
        Err(err) => {
            error!(?err, "failed to format MPP credential");
            return Err(());
        }
    };
    let frame = WsClientMessage::Credential { credential: auth };
    if let Err(err) = socket.send(Message::Text(frame.to_text().into())).await {
        error!(%err, "failed to send MPP credential");
        return Err(());
    }
    let _ =
        events_tx.send(if is_voucher { MppEvent::VoucherSent } else { MppEvent::CredentialSent });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_message<P: PaymentProvider + 'static, V: VoucherProvider>(
    msg: Message,
    interface: &ConnectionInterface,
    payment_provider: &P,
    voucher_provider: &V,
    receipt_tx: &watch::Sender<Option<Receipt>>,
    events_tx: &broadcast::Sender<MppEvent>,
    pending_pay: &mut Option<JoinHandle<Result<PaymentCredential, MppError>>>,
    pending_voucher: &mut Option<JoinHandle<Result<PaymentCredential, MppError>>>,
) -> Result<(), TerminationReason> {
    match msg {
        Message::Text(text) => {
            handle_text(
                &text,
                interface,
                payment_provider,
                voucher_provider,
                receipt_tx,
                events_tx,
                pending_pay,
                pending_voucher,
            )
            .await
        }
        Message::Close(frame) => {
            error!(?frame, "Received WS close frame");
            Err(TerminationReason::Transient)
        }
        Message::Binary(_) => {
            error!("Received binary WS frame; expected text");
            Err(TerminationReason::Fatal)
        }
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_text<P: PaymentProvider + 'static, V: VoucherProvider>(
    text: &str,
    interface: &ConnectionInterface,
    payment_provider: &P,
    voucher_provider: &V,
    receipt_tx: &watch::Sender<Option<Receipt>>,
    events_tx: &broadcast::Sender<MppEvent>,
    pending_pay: &mut Option<JoinHandle<Result<PaymentCredential, MppError>>>,
    pending_voucher: &mut Option<JoinHandle<Result<PaymentCredential, MppError>>>,
) -> Result<(), TerminationReason> {
    let server_msg: WsServerMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(err) => {
            error!(%err, %text, "failed to deserialize MPP frame");
            let _ = events_tx
                .send(MppEvent::Error(format!("malformed MPP frame: {err}; not reconnecting")));
            return Err(TerminationReason::Fatal);
        }
    };

    match server_msg {
        WsServerMessage::Challenge { challenge, error } => {
            if pending_pay.is_some() {
                error!("server issued a second challenge while a payment was in flight; closing");
                let _ = events_tx.send(MppEvent::Error(
                    "server issued a second challenge while a payment was in flight; not \
                     reconnecting"
                        .to_string(),
                ));
                return Err(TerminationReason::Fatal);
            }
            if let Some(err) = error {
                warn!(%err, "MPP challenge carried error message");
            }
            let parsed: PaymentChallenge = match serde_json::from_value(challenge) {
                Ok(c) => c,
                Err(err) => {
                    error!(%err, "failed to deserialize PaymentChallenge");
                    let _ = events_tx.send(MppEvent::Error(format!(
                        "malformed PaymentChallenge: {err}; not reconnecting"
                    )));
                    return Err(TerminationReason::Fatal);
                }
            };
            debug!(method = %parsed.method, intent = %parsed.intent, "MPP challenge received");
            let _ = events_tx.send(MppEvent::Challenge(parsed.clone()));

            // Skip `pay()` if the provider doesn't support (method, intent).
            if !payment_provider.supports(parsed.method.as_str(), parsed.intent.as_str()) {
                error!(method = %parsed.method, intent = %parsed.intent, "unsupported MPP challenge");
                let _ = events_tx.send(MppEvent::Error(format!(
                    "PaymentProvider does not support method={}, intent={}; not reconnecting",
                    parsed.method, parsed.intent,
                )));
                return Err(TerminationReason::Fatal);
            }

            // Spawn `pay()` so the main loop stays responsive (keepalive,
            // socket reads) while the provider signs/broadcasts.
            let provider = payment_provider.clone();
            *pending_pay = Some(tokio::spawn(async move { provider.pay(&parsed).await }));
            Ok(())
        }
        WsServerMessage::Data { data } => {
            // mpp-rs encodes the inner payload as a JSON-encoded string.
            let item: PubSubItem = match serde_json::from_str(&data) {
                Ok(i) => i,
                Err(err) => {
                    error!(%err, %data, "failed to deserialize JSON-RPC payload from MPP data frame");
                    let _ = events_tx.send(MppEvent::Error(format!(
                        "malformed MPP Data payload: {err}; not reconnecting"
                    )));
                    return Err(TerminationReason::Fatal);
                }
            };
            interface.send_to_frontend(item).map_err(|err| {
                error!(item=?err.0, "failed to forward to frontend");
                TerminationReason::Fatal
            })
        }
        WsServerMessage::NeedVoucher {
            channel_id,
            required_cumulative,
            accepted_cumulative,
            deposit,
        } => {
            if pending_voucher.is_some() {
                error!("server issued a second NeedVoucher while a voucher was in flight; closing");
                let _ = events_tx.send(MppEvent::Error(
                    "server issued a second NeedVoucher while a voucher was in flight; not \
                     reconnecting"
                        .to_string(),
                ));
                return Err(TerminationReason::Fatal);
            }
            let req =
                VoucherRequest { channel_id, required_cumulative, accepted_cumulative, deposit };
            debug!(?req, "MPP needVoucher received");
            let _ = events_tx.send(MppEvent::NeedVoucher(req.clone()));

            let provider = voucher_provider.clone();
            *pending_voucher = Some(tokio::spawn(async move { provider.next_voucher(&req).await }));
            Ok(())
        }
        WsServerMessage::Receipt { receipt } => {
            let parsed: Receipt = match serde_json::from_value(receipt) {
                Ok(r) => r,
                Err(err) => {
                    error!(%err, "failed to deserialize Receipt");
                    let _ = events_tx.send(MppEvent::Error(format!(
                        "malformed Receipt: {err}; not reconnecting"
                    )));
                    return Err(TerminationReason::Fatal);
                }
            };
            debug!(?parsed, "MPP receipt received");
            let _ = receipt_tx.send(Some(parsed.clone()));
            let _ = events_tx.send(MppEvent::Receipt(parsed));
            Ok(())
        }
        WsServerMessage::Error { error } => {
            error!(%error, "MPP error frame");
            let _ = events_tx.send(MppEvent::Error(error));
            Err(TerminationReason::Fatal)
        }
    }
}

#[cfg(any(feature = "aws-lc-rs", feature = "ring"))]
fn install_default_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        return;
    }
    #[cfg(feature = "aws-lc-rs")]
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    #[cfg(all(feature = "ring", not(feature = "aws-lc-rs")))]
    let provider = rustls::crypto::ring::default_provider();
    let _ = rustls::crypto::CryptoProvider::install_default(provider);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpp::PaymentPayload;

    #[derive(Clone)]
    struct DummyProvider;
    impl PaymentProvider for DummyProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            false
        }
        async fn pay(&self, ch: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            Ok(PaymentCredential::new(ch.to_echo(), PaymentPayload::hash("0x00")))
        }
    }

    #[test]
    fn into_client_request_sets_auth_header() {
        let connect = MppWsConnect::new("ws://example.com/rpc", DummyProvider)
            .with_auth(Authorization::bearer("tok"));
        let req = connect.into_client_request().unwrap();
        let v = req.headers().get(AUTHORIZATION).unwrap();
        assert_eq!(v.to_str().unwrap(), "Bearer tok");
    }

    #[test]
    fn is_local_heuristic() {
        assert!(MppWsConnect::new("ws://localhost:8545", DummyProvider).is_local());
        assert!(MppWsConnect::new("ws://127.0.0.1:8545", DummyProvider).is_local());
        assert!(!MppWsConnect::new("wss://eth.example/rpc", DummyProvider).is_local());
    }

    #[test]
    fn mpp_handle_initial_state() {
        let connect = MppWsConnect::new("ws://localhost", DummyProvider);
        let handle = connect.mpp_handle();
        assert!(handle.receipt.borrow().is_none());
        // Multiple subscribers are independent.
        let _other = connect.mpp_handle();
    }

    #[tokio::test]
    async fn no_voucher_returns_error() {
        let req = VoucherRequest {
            channel_id: "0xabc".into(),
            required_cumulative: "100".into(),
            accepted_cumulative: "50".into(),
            deposit: "200".into(),
        };
        assert!(NoVoucher.next_voucher(&req).await.is_err());
    }

    #[test]
    fn with_voucher_provider_swaps_v_type() {
        // Compile-time assertion that the type parameter changes as advertised.
        #[derive(Clone)]
        struct V;
        impl VoucherProvider for V {
            async fn next_voucher(
                &self,
                _: &VoucherRequest,
            ) -> Result<PaymentCredential, MppError> {
                Err(MppError::bad_request("never"))
            }
        }
        let connect: MppWsConnect<DummyProvider, V> =
            MppWsConnect::new("ws://localhost", DummyProvider).with_voucher_provider(V);
        // The connector remains usable.
        assert!(connect.is_local());
    }

    #[test]
    fn new_extracts_basic_auth_from_url_userinfo() {
        let connect = MppWsConnect::new("ws://alice:secret@example.com/rpc", DummyProvider);
        let req = connect.into_client_request().unwrap();
        let v = req.headers().get(AUTHORIZATION).expect("auth header set");
        // The exact base64 of "alice:secret" is fixed.
        assert_eq!(v.to_str().unwrap(), "Basic YWxpY2U6c2VjcmV0");
    }

    #[test]
    fn new_without_userinfo_has_no_auth_header() {
        let connect = MppWsConnect::new("ws://example.com/rpc", DummyProvider);
        let req = connect.into_client_request().unwrap();
        assert!(req.headers().get(AUTHORIZATION).is_none());
    }

    #[test]
    fn with_auth_overrides_url_extracted_auth() {
        // URL has userinfo, but explicit `with_auth` should win.
        let connect = MppWsConnect::new("ws://alice:secret@example.com/rpc", DummyProvider)
            .with_auth(Authorization::bearer("override"));
        let req = connect.into_client_request().unwrap();
        let v = req.headers().get(AUTHORIZATION).unwrap();
        assert_eq!(v.to_str().unwrap(), "Bearer override");
    }

    #[test]
    fn url_accessor_returns_configured_url() {
        let connect = MppWsConnect::new("ws://example.com/rpc", DummyProvider);
        assert_eq!(connect.url(), "ws://example.com/rpc");
    }

    #[test]
    fn is_local_handles_more_url_shapes() {
        // Path-only does not change the locality of the host.
        assert!(MppWsConnect::new("ws://localhost/rpc", DummyProvider).is_local());
        assert!(MppWsConnect::new("ws://127.0.0.1:8545/v1/ws", DummyProvider).is_local());
        // Public hosts and TLS schemes are not local.
        assert!(!MppWsConnect::new("ws://eth.example.com", DummyProvider).is_local());
        assert!(!MppWsConnect::new("wss://eth.example.com:8545/rpc", DummyProvider).is_local());
        // 0.0.0.0 is *not* treated as local by the upstream heuristic, despite
        // being a bind-any address; pin this behavior to catch surprises.
        assert!(!MppWsConnect::new("ws://0.0.0.0:8545", DummyProvider).is_local());
    }

    #[test]
    fn mpp_handle_multiple_subscribers_are_independent() {
        let connect = MppWsConnect::new("ws://localhost", DummyProvider);
        let h1 = connect.mpp_handle();
        let h2 = connect.mpp_handle();
        // Both watchers see the initial empty state.
        assert!(h1.receipt.borrow().is_none());
        assert!(h2.receipt.borrow().is_none());
        // Two independent receivers exist; sender count is 1 (the connector's tx).
        assert_eq!(connect.events_tx.receiver_count(), 2);
    }

    #[test]
    fn debug_redacts_url_userinfo_and_auth() {
        let connect = MppWsConnect::new("wss://alice:supersecret@example.com/rpc", DummyProvider)
            .with_auth(Authorization::bearer("token-xyz"));
        let s = format!("{connect:?}");
        assert!(!s.contains("supersecret"), "password leaked: {s}");
        assert!(!s.contains("alice"), "username leaked: {s}");
        assert!(!s.contains("token-xyz"), "bearer token leaked: {s}");
        assert!(s.contains("<redacted>"), "auth not marked redacted: {s}");
        assert!(s.contains("example.com"), "host should remain: {s}");
    }

    #[test]
    fn builder_setters_apply() {
        // Ensure the const builder methods chain cleanly. Field correctness
        // is covered indirectly by the integration tests; here we just assert
        // the type compiles and is usable across all setters.
        let cfg = WebSocketConfig::default();
        let connect = MppWsConnect::new("ws://localhost:8545", DummyProvider)
            .with_config(cfg)
            .with_max_retries(7)
            .with_retry_interval(Duration::from_millis(250))
            .with_keepalive_interval(Duration::from_millis(500))
            .with_auth(Authorization::raw("custom-token"));
        // Round-trips through IntoClientRequest with the explicit auth header.
        let req = connect.into_client_request().unwrap();
        let v = req.headers().get(AUTHORIZATION).unwrap();
        assert_eq!(v.to_str().unwrap(), "custom-token");
    }
}
