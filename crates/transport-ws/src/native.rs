use crate::{WsBackend, DEFAULT_KEEPALIVE};
use alloy_pubsub::PubSubConnect;
use alloy_transport::{utils::Spawnable, Authorization, TransportErrorKind, TransportResult};
use futures::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::{
    net::{lookup_host, TcpStream},
    time::{sleep, timeout},
};
use tokio_tungstenite::{
    tungstenite::{self, client::IntoClientRequest, error::UrlError, Message},
    MaybeTlsStream, WebSocketStream,
};

type TungsteniteStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// The default connect timeout in seconds.
const DEFAULT_CONNECT_TIMEOUT: u64 = 10;

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
    /// The timeout for each TCP connect attempt, and for the TLS handshake and websocket upgrade
    /// that follow.
    /// Default is 10 seconds.
    connect_timeout: Duration,
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
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT),
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
    /// A ping is sent if no other messages have been sent within this interval.
    /// If the server does not respond with a pong before the next ping is due,
    /// the connection is considered dead and will be closed.
    ///
    /// Default is 10 seconds.
    pub const fn with_keepalive_interval(mut self, keepalive_interval: Duration) -> Self {
        self.keepalive_interval = keepalive_interval;
        self
    }

    /// Sets the timeout for establishing a connection.
    ///
    /// The timeout applies to each TCP connect attempt, one per address the host resolves to, and
    /// separately to the TLS handshake and websocket upgrade that follow. It applies to the
    /// initial connect as well as to every reconnect attempt. If it expires for the last address
    /// or for the handshake, the attempt fails with a retryable transport error, so reconnects
    /// count it against the max number of retries.
    ///
    /// Default is 10 seconds.
    pub const fn with_connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout;
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
        let stream = connect_tcp(req.uri(), self.connect_timeout).await?;
        #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
        let handshake =
            tokio_tungstenite::client_async_tls_with_config(req, stream, self.config, None);
        // Without a TLS feature, `connect_tcp` rejects `wss` URLs.
        #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
        let handshake = tokio_tungstenite::client_async_with_config(
            req,
            MaybeTlsStream::Plain(stream),
            self.config,
        );
        let (socket, _) = timeout(self.connect_timeout, handshake)
            .await
            .map_err(|_| {
                TransportErrorKind::custom_str(&format!(
                    "connecting timed out after {:?}",
                    self.connect_timeout
                ))
            })?
            .map_err(TransportErrorKind::custom)?;

        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
        let backend = WsBackend { socket, interface, keepalive_interval: self.keepalive_interval };

        backend.spawn();

        Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
    }
}

/// Opens a TCP connection to the host of `uri`, trying the addresses it resolves to in turn.
///
/// Each address gets its own `connect_timeout`, so that an address that silently drops the
/// connection attempt does not use up the time of the addresses after it.
async fn connect_tcp(uri: &http::Uri, connect_timeout: Duration) -> TransportResult<TcpStream> {
    let url_err = |err| TransportErrorKind::custom(tungstenite::Error::Url(err));
    let default_port = match uri.scheme_str() {
        Some("ws") => 80,
        #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
        Some("wss") => 443,
        #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
        Some("wss") => return Err(url_err(UrlError::TlsFeatureNotEnabled)),
        _ => return Err(url_err(UrlError::UnsupportedUrlScheme)),
    };
    let host = uri.host().ok_or_else(|| url_err(UrlError::NoHostName))?;
    // IPv6 literals are bracketed in URLs, but not when resolved.
    let host = host.strip_prefix('[').and_then(|host| host.strip_suffix(']')).unwrap_or(host);
    let addrs = lookup_host((host, uri.port_u16().unwrap_or(default_port)))
        .await
        .map_err(TransportErrorKind::custom)?;

    let mut last_err = None;
    for addr in addrs {
        match timeout(connect_timeout, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(err)) => last_err = Some(TransportErrorKind::custom(err)),
            Err(_) => {
                last_err = Some(TransportErrorKind::custom_str(&format!(
                    "connecting to {addr} timed out after {connect_timeout:?}"
                )))
            }
        }
    }
    Err(last_err.unwrap_or_else(|| TransportErrorKind::custom_str("host resolved to no address")))
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

    #[tokio::test]
    async fn connect_times_out_on_stalled_handshake() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // Accept the TCP connection but never answer the upgrade request.
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
            drop(socket);
        });

        let connect = WsConnect::new(format!("ws://{addr}"))
            .with_connect_timeout(Duration::from_millis(200))
            .into_service();
        let err = timeout(Duration::from_secs(30), connect)
            .await
            .expect("connect should time out instead of hanging")
            .unwrap_err();
        assert!(!err.as_transport_err().unwrap().is_non_retryable());
        assert_eq!(err.to_string(), "connecting timed out after 200ms");

        server.abort();
    }
}
