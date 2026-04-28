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
use std::{future::Future, time::Duration};
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
#[derive(Clone, Debug)]
pub struct MppWsConnect<P, V = NoVoucher> {
    url: String,
    auth: Option<Authorization>,
    config: Option<WebSocketConfig>,
    max_retries: u32,
    retry_interval: Duration,
    keepalive_interval: Duration,
    payment_provider: P,
    voucher_provider: V,
    receipt_tx: watch::Sender<Option<Receipt>>,
    events_tx: broadcast::Sender<MppEvent>,
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
            payment_provider,
            voucher_provider: NoVoucher,
            receipt_tx,
            events_tx,
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
            payment_provider: self.payment_provider,
            voucher_provider,
            receipt_tx: self.receipt_tx,
            events_tx: self.events_tx,
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
            self.receipt_tx.clone(),
            self.events_tx.clone(),
        )
        .spawn_task();

        Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
    }
}

/// Bridge between alloy's JSON-RPC channels and MPP frames.
async fn run_translator<P, V>(
    mut socket: WsStream,
    mut interface: ConnectionInterface,
    payment_provider: P,
    voucher_provider: V,
    keepalive_interval: Duration,
    receipt_tx: watch::Sender<Option<Receipt>>,
    events_tx: broadcast::Sender<MppEvent>,
) where
    P: PaymentProvider + 'static,
    V: VoucherProvider,
{
    let mut errored = false;
    let mut expecting_pong = false;
    // `false` until the first credential has been sent; gates outbound JSON-RPC
    // so application traffic never races the initial challenge.
    let mut handshake_complete = false;
    let keepalive = sleep(keepalive_interval);
    tokio::pin!(keepalive);

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
                            errored = true;
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
                            errored = true;
                            break;
                        }
                        handshake_complete = true;
                    }
                    Ok(Err(err)) => {
                        error!(?err, "MPP payment provider failed");
                        let _ = events_tx.send(MppEvent::Error(err.to_string()));
                        errored = true;
                        break;
                    }
                    Err(join_err) => {
                        error!(%join_err, "MPP payment task panicked");
                        errored = true;
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
                            errored = true;
                            break;
                        }
                        handshake_complete = true;
                    }
                    Ok(Err(err)) => {
                        error!(?err, "MPP voucher provider failed");
                        let _ = events_tx.send(MppEvent::Error(err.to_string()));
                        errored = true;
                        break;
                    }
                    Err(join_err) => {
                        error!(%join_err, "MPP voucher task panicked");
                        errored = true;
                        break;
                    }
                }
            }

            // 4. Keepalive ping.
            _ = &mut keepalive => {
                if expecting_pong {
                    error!("WS server missed a pong");
                    errored = true;
                    break;
                }
                keepalive.as_mut().reset(Instant::now() + keepalive_interval);
                if let Err(err) = socket.send(Message::Ping(Default::default())).await {
                    error!(%err, "WS connection error");
                    errored = true;
                    break;
                }
                expecting_pong = true;
            }

            // 5. Inbound from socket → MPP frame.
            resp = socket.next() => {
                match resp {
                    Some(Ok(item)) => {
                        if item.is_pong() {
                            expecting_pong = false;
                        }
                        let r = handle_message(
                            item,
                            &interface,
                            &payment_provider,
                            &voucher_provider,
                            &receipt_tx,
                            &events_tx,
                            &mut pending_pay,
                            &mut pending_voucher,
                        ).await;
                        if r.is_err() {
                            errored = true;
                            break;
                        }
                    }
                    Some(Err(err)) => {
                        error!(%err, "WS connection error");
                        errored = true;
                        break;
                    }
                    None => {
                        error!("WS server has gone away");
                        errored = true;
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

    if errored {
        interface.close_with_error();
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
) -> Result<(), ()> {
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
            Err(())
        }
        Message::Binary(_) => {
            error!("Received binary WS frame; expected text");
            Err(())
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
) -> Result<(), ()> {
    let server_msg: WsServerMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(err) => {
            error!(%err, %text, "failed to deserialize MPP frame");
            return Err(());
        }
    };

    match server_msg {
        WsServerMessage::Challenge { challenge, error } => {
            if pending_pay.is_some() {
                error!("server issued a second challenge while a payment was in flight; closing");
                return Err(());
            }
            if let Some(err) = error {
                warn!(%err, "MPP challenge carried error message");
            }
            let parsed: PaymentChallenge = match serde_json::from_value(challenge) {
                Ok(c) => c,
                Err(err) => {
                    error!(%err, "failed to deserialize PaymentChallenge");
                    return Err(());
                }
            };
            debug!(method = %parsed.method, intent = %parsed.intent, "MPP challenge received");
            let _ = events_tx.send(MppEvent::Challenge(parsed.clone()));

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
                    return Err(());
                }
            };
            interface.send_to_frontend(item).map_err(|err| {
                error!(item=?err.0, "failed to forward to frontend");
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
                return Err(());
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
                    return Err(());
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
            Err(())
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
}
