use crate::{WsBackend, DEFAULT_KEEPALIVE};
use alloy_pubsub::PubSubConnect;
use alloy_transport::{utils::Spawnable, Authorization, TransportErrorKind, TransportResult};
use futures::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::time::{sleep, Instant};
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
        self.socket.send(Message::Text(String::from(Box::<str>::from(msg)).into())).await
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
                // 2. Response or notification from server.
                // 3. Keepalive.
                // This prioritizes new dispatches over responses from the
                // server. A ping is sent only if nothing has been received from
                // the server in the keepalive interval. Outbound traffic must
                // not postpone it: writes keep succeeding on a half-open
                // connection that will never answer. Whatever was already
                // received, such as a pong held back by a burst of dispatches,
                // is handled before the keepalive checks for a missed pong.
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
                    resp = self.socket.next() => {
                        match resp {
                            Some(Ok(item)) => {
                                // Any message from the server shows the connection is alive.
                                expecting_pong = false;
                                keepalive.as_mut().reset(Instant::now() + self.keepalive_interval);
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
                    // Send a ping to the server, if no messages have been
                    // received within the keepalive interval.
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

    /// A server that completes the handshake and then stops reading looks alive to a client that
    /// keeps sending: writes land in socket buffers and no pong ever comes back. The keepalive
    /// must still notice the missing pong and reconnect.
    #[tokio::test]
    async fn missed_pong_detected_under_steady_outbound_traffic() {
        use alloy_json_rpc::{Id, Request};
        use tokio::{net::TcpListener, sync::mpsc, time::timeout};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (accepted_tx, mut accepted) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut silent = Vec::new();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                // Hold the socket open without ever reading from it.
                silent.push(ws);
                let _ = accepted_tx.send(());
            }
        });

        let frontend = WsConnect::new(format!("ws://{addr}"))
            .with_keepalive_interval(Duration::from_millis(100))
            .into_service()
            .await
            .unwrap();
        accepted.recv().await.unwrap();

        // Dispatch requests more often than the keepalive interval.
        let sender = tokio::spawn(async move {
            let mut id = 0;
            loop {
                id += 1;
                let req = Request::new("eth_blockNumber", Id::Number(id), ()).serialize().unwrap();
                tokio::spawn(frontend.send(req));
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });

        let reconnected = timeout(Duration::from_secs(2), accepted.recv()).await;
        sender.abort();
        assert!(
            matches!(reconnected, Ok(Some(()))),
            "a server that never answers pings must be detected as dead"
        );
    }

    /// A burst of dispatches that outlasts the keepalive interval delays reading the pong to the
    /// ping sent before it. That pong must be handled before the keepalive checks for a missed
    /// one, or a healthy connection is dropped.
    #[tokio::test]
    async fn pong_received_during_outbound_burst_keeps_connection() {
        use alloy_json_rpc::{Id, Request};
        use tokio::{
            net::TcpListener,
            sync::{mpsc, oneshot},
            time::timeout,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (accepted_tx, mut accepted) = mpsc::unbounded_channel();
        let (start_reading, start_rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let _ = accepted_tx.send(());
            tokio::spawn(async move {
                let _ = start_rx.await;
                // Reading lets tungstenite answer pings with pongs.
                while let Some(Ok(_)) = ws.next().await {}
            });
            // A reconnect means the connection was declared dead.
            let _ = listener.accept().await;
            let _ = accepted_tx.send(());
        });

        let keepalive = Duration::from_millis(500);
        let frontend = WsConnect::new(format!("ws://{addr}"))
            .with_keepalive_interval(keepalive)
            .into_service()
            .await
            .unwrap();
        accepted.recv().await.unwrap();

        // After the first ping, queue more than the socket buffers hold while the server isn't
        // reading, so dispatching stalls past the keepalive deadline.
        tokio::time::sleep(keepalive * 3 / 2).await;
        let payload = "0".repeat(32 * 1024);
        for id in 0..1024 {
            let req = Request::new("eth_call", Id::Number(id), [&payload]).serialize().unwrap();
            tokio::spawn(frontend.send(req));
        }
        tokio::time::sleep(keepalive * 2).await;
        start_reading.send(()).unwrap();

        assert!(
            timeout(keepalive * 3, accepted.recv()).await.is_err(),
            "a connection whose pong arrived during a burst must not be dropped"
        );
    }

    #[test]
    fn explicit_auth_overrides_url_auth() {
        let ws =
            WsConnect::new("wss://user:pass@example.com").with_auth(Authorization::bearer("tok"));
        assert_eq!(ws.auth(), Some(&Authorization::bearer("tok")));
    }
}
