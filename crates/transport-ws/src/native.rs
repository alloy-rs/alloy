use crate::{WsBackend, DEFAULT_KEEPALIVE};
use alloy_pubsub::PubSubConnect;
use alloy_transport::{
    utils::Spawnable, Authorization, TransportError, TransportErrorKind, TransportResult,
};
use futures::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{
    tungstenite::{self, client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};

type TungsteniteStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// Simple connection details for a websocket connection.
#[derive(Clone, Debug)]
pub struct WsConnect {
    /// The URL to connect to.
    url: String,
    /// The authorization header to use.
    auth: Option<Authorization>,
    /// The websocket config.
    config: Option<WebSocketConfig>,
    /// The interval between keepalive pings.
    /// Default is 10 seconds.
    keepalive_interval: Duration,
}

impl WsConnect {
    /// Creates a new websocket connection configuration.
    ///
    /// If the URL contains credentials (e.g. `wss://user:pass@host`), they are
    /// automatically extracted and set as the [`Authorization`] header.
    pub fn new<S: Into<String>>(url: S) -> Self {
        let url = url.into();
        let auth =
            url::Url::parse(&url).ok().and_then(|parsed| Authorization::extract_from_url(&parsed));
        Self { url, auth, config: None, keepalive_interval: Duration::from_secs(DEFAULT_KEEPALIVE) }
    }

    /// Sets the authorization header.
    pub fn with_auth(mut self, auth: Authorization) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Sets the optional authorization header.
    ///
    /// This replaces the current [`Authorization`].
    pub fn with_auth_opt(mut self, auth: Option<Authorization>) -> Self {
        self.auth = auth;
        self
    }

    /// Sets the websocket config.
    pub const fn with_config(mut self, config: WebSocketConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Get the URL string of the connection.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the authorization header.
    pub const fn auth(&self) -> Option<&Authorization> {
        self.auth.as_ref()
    }

    /// Get the websocket config.
    pub const fn config(&self) -> Option<&WebSocketConfig> {
        self.config.as_ref()
    }

    /// Sets the keepalive ping interval.
    ///
    /// A ping is sent if no other messages have been sent within this interval.
    /// If the server does not respond with a pong before the next ping is due,
    /// the connection is considered dead and will be closed.
    ///
    /// Default is 10 seconds.
    pub const fn with_keepalive_interval(mut self, keepalive_interval: Duration) -> Self {
        self.keepalive_interval = keepalive_interval;
        self
    }
}

impl IntoClientRequest for WsConnect {
    fn into_client_request(self) -> tungstenite::Result<tungstenite::handshake::client::Request> {
        let mut request: http::Request<()> = self.url.into_client_request()?;
        if let Some(auth) = self.auth {
            let mut auth_value = http::HeaderValue::from_str(&auth.to_string())?;
            auth_value.set_sensitive(true);

            request.headers_mut().insert(http::header::AUTHORIZATION, auth_value);
        }

        request.into_client_request()
    }
}

impl PubSubConnect for WsConnect {
    fn is_local(&self) -> bool {
        alloy_transport::utils::guess_local_url(&self.url)
    }

    async fn connect(&self) -> TransportResult<alloy_pubsub::ConnectionHandle> {
        #[cfg(any(feature = "aws-lc-rs", feature = "ring"))]
        install_default_crypto_provider();

        let request = self.clone().into_client_request();
        let req = request.map_err(TransportErrorKind::custom)?;
        let (socket, _) = tokio_tungstenite::connect_async_with_config(req, self.config, false)
            .await
            .map_err(handshake_error)?;

        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
        let backend = WsBackend { socket, interface, keepalive_interval: self.keepalive_interval };

        backend.spawn();

        Ok(handle)
    }
}

// Keep handshake HTTP evidence visible to the pubsub reconnect policy.
fn handshake_error(error: tungstenite::Error) -> TransportError {
    match error {
        tungstenite::Error::Http(response) => TransportErrorKind::http_error(
            response.status().as_u16(),
            String::from_utf8_lossy(response.body().as_deref().unwrap_or_default()).into_owned(),
        ),
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
            TransportErrorKind::custom(std::io::Error::from(std::io::ErrorKind::ConnectionReset))
        }
        tungstenite::Error::Protocol(tungstenite::error::ProtocolError::HandshakeIncomplete) => {
            TransportErrorKind::custom(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
        }
        error => TransportErrorKind::custom(error),
    }
}

/// Install a default rustls crypto provider if none is set.
///
/// Required since rustls 0.23+ no longer auto-installs one.
#[cfg(any(feature = "aws-lc-rs", feature = "ring"))]
fn install_default_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        return;
    }
    #[cfg(feature = "aws-lc-rs")]
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    #[cfg(all(feature = "ring", not(feature = "aws-lc-rs")))]
    let provider = rustls::crypto::ring::default_provider();
    // install_default returns Err if a concurrent caller raced us past the get_default check;
    // either provider is valid.
    let _ = rustls::crypto::CryptoProvider::install_default(provider);
}

impl WsBackend<TungsteniteStream> {
    /// Handle a message from the server.
    #[expect(clippy::result_unit_err)]
    pub fn handle(&mut self, msg: Message) -> Result<(), ()> {
        match msg {
            Message::Text(text) => self.handle_text(&text),
            Message::Close(frame) => {
                if frame.is_some() {
                    error!(?frame, "Received close frame with data");
                } else {
                    error!("WS server has gone away");
                }
                Err(())
            }
            Message::Binary(_) => {
                error!("Received binary message, expected text");
                Err(())
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => Ok(()),
        }
    }

    /// Send a message to the server.
    pub async fn send(&mut self, msg: Box<RawValue>) -> Result<(), tungstenite::Error> {
        self.socket.send(Message::Text(msg.get().to_owned().into())).await
    }

    /// Spawn a new backend task.
    pub fn spawn(mut self) {
        let fut = async move {
            let mut errored = false;
            let mut expecting_pong = false;
            let keepalive = sleep(self.keepalive_interval);
            tokio::pin!(keepalive);
            loop {
                // We bias the loop as follows
                // 1. New dispatch to server.
                // 2. Keepalive.
                // 3. Response or notification from server.
                // This ensures that keepalive is sent only if no other messages
                // have been sent in the keepalive interval. And prioritizes new
                // dispatches over responses from the server. This will fail if
                // the client saturates the task with dispatches, but that's
                // probably not a big deal.
                tokio::select! {
                    biased;
                    // we've received a new dispatch, so we send it via
                    // websocket. We handle new work before processing any
                    // responses from the server.
                    inst = self.interface.recv_from_frontend() => {
                        match inst {
                            Some(msg) => {
                                // Reset the keepalive timer.
                                keepalive.set(sleep(self.keepalive_interval));
                                if let Err(err) = self.send(msg).await {
                                    error!(%err, "WS connection error");
                                    errored = true;
                                    break
                                }
                            },
                            // dispatcher has gone away, or shutdown was received
                            None => {
                                break
                            },
                        }
                    },
                    // Send a ping to the server, if no other messages have been
                    // sent within the keepalive interval.
                    _ = &mut keepalive => {
                        // Still expecting a pong from the previous ping,
                        // meaning connection is errored.
                        if expecting_pong {
                            error!("WS server missed a pong");
                            errored = true;
                            break
                        }
                        // Reset the keepalive timer.
                        keepalive.set(sleep(self.keepalive_interval));
                        if let Err(err) = self.socket.send(Message::Ping(Default::default())).await {
                            error!(%err, "WS connection error");
                            errored = true;
                            break
                        }
                        // Expecting to receive a pong before the next
                        // keepalive timer resolves.
                        expecting_pong = true;
                    }
                    resp = self.socket.next() => {
                        match resp {
                            Some(Ok(item)) => {
                                if item.is_pong() {
                                    expecting_pong = false;
                                }
                                errored = self.handle(item).is_err();
                                if errored { break }
                            },
                            Some(Err(err)) => {
                                error!(%err, "WS connection error");
                                errored = true;
                                break
                            }
                            None => {
                                error!("WS server has gone away");
                                errored = true;
                                break
                            },
                        }
                    }
                }
            }
            if errored {
                self.interface.close_with_error();
            }
        };
        fut.spawn_task()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_auth_from_url() {
        let ws = WsConnect::new("wss://user:pass@example.com/path");
        assert_eq!(ws.url(), "wss://user:pass@example.com/path");
        assert_eq!(ws.auth(), Some(&Authorization::basic("user", "pass")));
    }

    #[test]
    fn parse_username_only_from_url() {
        let ws = WsConnect::new("ws://user@example.com");
        assert_eq!(ws.url(), "ws://user@example.com");
        assert_eq!(ws.auth(), Some(&Authorization::basic("user", "")));
    }

    #[test]
    fn no_auth_when_url_has_no_credentials() {
        let ws = WsConnect::new("wss://example.com/rpc");
        assert_eq!(ws.url(), "wss://example.com/rpc");
        assert!(ws.auth().is_none());
    }

    #[test]
    fn explicit_auth_overrides_url_auth() {
        let ws =
            WsConnect::new("wss://user:pass@example.com").with_auth(Authorization::bearer("tok"));
        assert_eq!(ws.auth(), Some(&Authorization::bearer("tok")));
    }

    #[test]
    fn no_auth_for_localhost_username() {
        let ws = WsConnect::new("ws://localhost:8545");
        assert!(ws.auth().is_none());
    }
}

#[cfg(test)]
mod handshake_error_tests {
    use super::*;
    use alloy_json_rpc::{Id, Request};
    use serde_json::{json, Value};
    use tokio::{net::TcpListener, sync::oneshot, time::timeout};

    #[tokio::test]
    async fn subscription_recovers_after_temporary_handshake_rejection() {
        timeout(Duration::from_secs(3), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (ready_tx, ready_rx) = oneshot::channel();
            let (done_tx, done_rx) = oneshot::channel();
            let server = tokio::spawn(async move {
                let (socket, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
                let request: Value = serde_json::from_str(
                    socket.next().await.unwrap().unwrap().to_text().unwrap(),
                ).unwrap();
                assert_eq!(request, json!({"jsonrpc":"2.0", "id":91,
                    "method":"eth_subscribe", "params":["newHeads"]}));
                socket.send(Message::Text(json!({"jsonrpc":"2.0", "id":91,
                    "result":"original-subscription"}).to_string().into())).await.unwrap();
                ready_rx.await.unwrap();
                drop(socket);

                let (socket, _) = listener.accept().await.unwrap();
                let rejected = tokio_tungstenite::accept_hdr_async(socket,
                    |_: &tungstenite::handshake::server::Request,
                     _: tungstenite::handshake::server::Response| {
                        Err(http::Response::builder().status(503)
                            .body(Some("temporarily unavailable".into())).unwrap())
                    }).await;
                assert!(rejected.is_err());

                let (socket, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
                let replay: Value = serde_json::from_str(
                    socket.next().await.unwrap().unwrap().to_text().unwrap(),
                ).unwrap();
                assert_eq!(replay, request, "replay must preserve ID and subscription parameters");
                socket.send(Message::Text(json!({"jsonrpc":"2.0", "id":91,
                    "result":"replacement-subscription"}).to_string().into())).await.unwrap();
                socket.send(Message::Text(json!({"jsonrpc":"2.0", "method":"eth_subscription",
                    "params":{"subscription":"replacement-subscription", "result":{"number":"0x123"}}})
                    .to_string().into())).await.unwrap();
                done_rx.await.unwrap();
            });
            let frontend = WsConnect::new(format!("ws://{address}")).into_service().await.unwrap();
            let request = Request::new("eth_subscribe", Id::Number(91), ("newHeads",))
                .serialize().unwrap();
            let response = frontend.send(request).await.unwrap();
            let local_id = response.try_success_as().unwrap().unwrap();
            let mut subscription = frontend.get_subscription(local_id).await.unwrap();
            ready_tx.send(()).unwrap();
            let notification: Value = serde_json::from_str(subscription.recv().await.unwrap().get()).unwrap();
            assert_eq!(notification, json!({"number":"0x123"}));
            assert_eq!(*subscription.local_id(), local_id);
            done_tx.send(()).unwrap();
            drop(frontend);
            server.await.unwrap();
        }).await.expect("subscription must survive the HTTP 503 reconnect");
    }

    #[tokio::test]
    async fn handshake_failure_retains_http_status_and_body() {
        for status in [408, 429, 500, 502, 503, 504, 401, 403] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (socket, _) = listener.accept().await.unwrap();
                let result = tokio_tungstenite::accept_hdr_async(
                    socket,
                    move |_: &tungstenite::handshake::server::Request,
                          _: tungstenite::handshake::server::Response| {
                        Err(http::Response::builder()
                            .status(status)
                            .body(Some("handshake rejected".into()))
                            .unwrap())
                    },
                )
                .await;
                assert!(result.is_err());
            });
            let error = WsConnect::new(format!("ws://{address}")).connect().await.unwrap_err();
            match error.as_transport_err() {
                Some(TransportErrorKind::HttpError(error)) => {
                    assert_eq!(error.status, status);
                    assert_eq!(error.body, "handshake rejected");
                }
                other => panic!("handshake status {status} lost its HTTP type: {other:?}"),
            }
            server.await.unwrap();
        }
    }
}
