use crate::{WsBackend, DEFAULT_KEEPALIVE};
use alloy_pubsub::PubSubConnect;
use alloy_transport::{utils::Spawnable, Authorization, TransportErrorKind, TransportResult};
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
    /// Max number of retries before failing and exiting the connection.
    /// Default is 10.
    max_retries: u32,
    /// The base interval between retries.
    ///
    /// Reconnect retries use capped exponential backoff from this base interval.
    /// Default is 3 seconds.
    retry_interval: Duration,
    /// The interval between keepalive pings.
    /// Default is 10 seconds.
    keepalive_interval: Duration,
    /// The deadline for one connection attempt, including the TCP, TLS and WebSocket handshakes.
    /// Default is no deadline.
    connect_timeout: Option<Duration>,
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
        Self {
            url,
            auth,
            config: None,
            max_retries: 10,
            retry_interval: Duration::from_secs(3),
            keepalive_interval: Duration::from_secs(DEFAULT_KEEPALIVE),
            connect_timeout: None,
        }
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

    /// Sets the max number of retries before failing and exiting the connection.
    /// Default is 10.
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the base interval between retries.
    ///
    /// Reconnect retries use capped exponential backoff from this base interval.
    /// Default is 3 seconds.
    pub const fn with_retry_interval(mut self, retry_interval: Duration) -> Self {
        self.retry_interval = retry_interval;
        self
    }

    /// Sets the keepalive ping interval.
    ///
    /// A ping is sent if nothing has been received from the server within this interval.
    /// If nothing, including the pong, is received before the next ping is due, the connection
    /// is considered dead and will be closed. Requests sent in the meantime do not delay the
    /// ping, so a connection that stopped answering is detected while requests keep being sent.
    ///
    /// Default is 10 seconds.
    pub const fn with_keepalive_interval(mut self, keepalive_interval: Duration) -> Self {
        self.keepalive_interval = keepalive_interval;
        self
    }

    /// Sets the deadline for one connection attempt, including the TCP, TLS and WebSocket
    /// handshakes.
    ///
    /// It applies to the initial connection and to every reconnect attempt. An attempt that
    /// exceeds it fails, and reconnects are retried as configured with
    /// [`with_max_retries`](Self::with_max_retries).
    ///
    /// Default is no deadline.
    pub const fn with_connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = Some(connect_timeout);
        self
    }

    /// Get the deadline for one connection attempt, if any.
    pub const fn connect_timeout(&self) -> Option<Duration> {
        self.connect_timeout
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
        let connect = tokio_tungstenite::connect_async_with_config(req, self.config, false);
        let (socket, _) = match self.connect_timeout {
            Some(connect_timeout) => {
                tokio::time::timeout(connect_timeout, connect).await.map_err(|_| {
                    TransportErrorKind::custom_str(&format!(
                        "WS connection timed out after {connect_timeout:?}"
                    ))
                })?
            }
            None => connect.await,
        }
        .map_err(TransportErrorKind::custom)?;

        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
        let backend = WsBackend { socket, interface, keepalive_interval: self.keepalive_interval };

        backend.spawn();

        Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
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
                // This prioritizes new dispatches over responses from the
                // server. The keepalive timer is reset only by messages
                // received from the server, so a ping is sent if nothing has
                // been received in the keepalive interval, however many
                // dispatches were sent. Resetting it on dispatches would hide
                // a connection that stopped answering for as long as requests
                // keep being sent.
                tokio::select! {
                    biased;
                    // we've received a new dispatch, so we send it via
                    // websocket. We handle new work before processing any
                    // responses from the server.
                    inst = self.interface.recv_from_frontend() => {
                        match inst {
                            Some(msg) => {
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
                    // Send a ping to the server, if nothing has been received
                    // within the keepalive interval.
                    _ = &mut keepalive => {
                        // Nothing received since the previous ping, meaning
                        // the connection is errored.
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
                                // Any message from the server shows the
                                // connection is alive.
                                expecting_pong = false;
                                keepalive.set(sleep(self.keepalive_interval));
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
    fn auth_from_url() {
        for (url, auth) in [
            ("wss://user:pass@example.com/path", Some(Authorization::basic("user", "pass"))),
            ("ws://user@example.com", Some(Authorization::basic("user", ""))),
            ("wss://example.com/rpc", None),
            ("ws://localhost:8545", None),
        ] {
            let ws = WsConnect::new(url);
            assert_eq!(ws.url(), url);
            assert_eq!(ws.auth(), auth.as_ref(), "{url}");
        }
    }

    #[test]
    fn explicit_auth_overrides_url_auth() {
        let ws =
            WsConnect::new("wss://user:pass@example.com").with_auth(Authorization::bearer("tok"));
        assert_eq!(ws.auth(), Some(&Authorization::bearer("tok")));
    }

    /// Request with the given id, as the frontend sends it.
    fn request(id: u64) -> alloy_json_rpc::SerializedRequest {
        alloy_json_rpc::Request::new("eth_chainId", alloy_json_rpc::Id::Number(id), ())
            .serialize()
            .unwrap()
    }

    #[tokio::test]
    async fn keepalive_detects_silent_server_while_requests_are_sent() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // Completes one WebSocket handshake, then never reads or writes again, like a server
        // behind a half-open TCP connection. Reconnects are refused.
        let (silent_tx, silent_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            drop(listener);
            let socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let _ = silent_tx.send(socket);
        });

        let client = WsConnect::new(format!("ws://{addr}"))
            .with_keepalive_interval(Duration::from_millis(100))
            .with_max_retries(1)
            .with_retry_interval(Duration::from_millis(10))
            .into_service()
            .await
            .unwrap();
        let _socket = silent_rx.await.unwrap();

        // Keep sending requests more often than the keepalive interval.
        let first = tokio::spawn(client.send(request(0)));
        let sender = {
            let client = client.clone();
            tokio::spawn(async move {
                for id in 1..=u64::MAX {
                    tokio::spawn(client.send(request(id)));
                    sleep(Duration::from_millis(20)).await;
                }
            })
        };

        let response = tokio::time::timeout(Duration::from_secs(5), first)
            .await
            .expect("a silent server must be detected while requests are sent")
            .unwrap();
        sender.abort();
        assert!(response.is_err(), "{response:?}");
    }

    #[tokio::test]
    async fn connect_times_out_on_unanswered_handshake() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // Accepts the TCP connection but never answers the WebSocket handshake.
        tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });

        let ws =
            WsConnect::new(format!("ws://{addr}")).with_connect_timeout(Duration::from_millis(100));
        let err = tokio::time::timeout(Duration::from_secs(5), ws.connect())
            .await
            .expect("the connect deadline must end the attempt")
            .unwrap_err();
        assert_eq!(err.to_string(), "WS connection timed out after 100ms");
    }
}
