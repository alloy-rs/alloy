use alloy_json_rpc::RpcError;
use alloy_transport::{BoxTransport, TransportConnect, TransportError, TransportErrorKind};
use std::{str::FromStr, time::Duration};

#[cfg(any(feature = "ws-base", feature = "ipc"))]
use alloy_pubsub::PubSubConnect;

// Both re-export `http::header`.
#[cfg(feature = "hyper")]
use alloy_transport_http::hyper::header;
#[cfg(all(
    not(feature = "hyper"),
    feature = "reqwest",
    not(all(target_os = "wasi", target_env = "p1"))
))]
use reqwest::header;

/// Connection string for built-in transports.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuiltInConnectionString {
    /// HTTP transport.
    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    Http(url::Url),
    /// WebSocket transport.
    #[cfg(feature = "ws-base")]
    Ws(url::Url, Option<alloy_transport::Authorization>),
    /// IPC transport.
    #[cfg(feature = "ipc")]
    Ipc(std::path::PathBuf),
}

impl TransportConnect for BuiltInConnectionString {
    fn is_local(&self) -> bool {
        match self {
            #[cfg(any(feature = "reqwest", feature = "hyper"))]
            Self::Http(url) => alloy_transport::utils::guess_local_url(url),
            #[cfg(feature = "ws-base")]
            Self::Ws(url, _) => alloy_transport::utils::guess_local_url(url),
            #[cfg(feature = "ipc")]
            Self::Ipc(_) => true,
            #[cfg(not(any(
                feature = "reqwest",
                feature = "hyper",
                feature = "ws-base",
                feature = "ipc"
            )))]
            _ => false,
        }
    }

    async fn get_transport(&self) -> Result<BoxTransport, TransportError> {
        self.connect_boxed().await
    }
}

impl BuiltInConnectionString {
    /// Parse a connection string and connect to it in one go.
    ///
    /// This is a convenience method that combines `from_str` and `connect_boxed`.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_rpc_client::BuiltInConnectionString;
    ///
    /// let transport = BuiltInConnectionString::connect("http://localhost:8545").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(s: &str) -> Result<BoxTransport, TransportError> {
        let connection = Self::from_str(s)?;
        connection.connect_boxed().await
    }

    /// Parse a connection string and connect with custom configuration.
    ///
    /// This method allows for fine-grained control over connection settings
    /// such as authentication, retry behavior, and transport-specific options.
    ///
    /// # Examples
    ///
    /// Basic usage with authentication:
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_rpc_client::{BuiltInConnectionString, ConnectionConfig};
    /// use alloy_transport::Authorization;
    /// use std::time::Duration;
    ///
    /// // Configure connection with custom settings
    /// let config = ConnectionConfig::new()
    ///     .with_auth(Authorization::bearer("my-token"))
    ///     .with_max_retries(3)
    ///     .with_retry_interval(Duration::from_secs(2));
    ///
    /// // Connect to WebSocket endpoint with configuration
    /// let transport = BuiltInConnectionString::connect_with("ws://localhost:8545", config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_with(
        s: &str,
        config: ConnectionConfig,
    ) -> Result<BoxTransport, TransportError> {
        let connection = Self::from_str(s)?;
        connection.connect_boxed_with(config).await
    }

    /// Connect with the given connection string.
    ///
    /// # Notes
    ///
    /// - If the `hyper` feature is enabled, HTTP connections use hyper instead of reqwest.
    /// - WS will extract auth, however, auth is disabled for wasm.
    pub async fn connect_boxed(&self) -> Result<BoxTransport, TransportError> {
        self.connect_boxed_with(ConnectionConfig::default()).await
    }

    /// Connect with the given connection string and custom configuration.
    ///
    /// This method provides fine-grained control over connection settings.
    /// Configuration options are applied where supported by the transport.
    ///
    /// # Notes
    ///
    /// - If the `hyper` feature is enabled, HTTP connections use hyper instead of reqwest.
    /// - [`ConnectionConfig::auth`] is sent as the `Authorization` header of every HTTP request and
    ///   of the WS handshake. It takes precedence over credentials in the URL, which are then not
    ///   sent. A header set on an individual request takes precedence over it for HTTP. WS auth is
    ///   disabled for wasm.
    /// - [`ConnectionConfig::max_retries`] and [`ConnectionConfig::retry_interval`] control WS
    ///   reconnects only. HTTP has no persistent connection to retry; use `RetryBackoffLayer` to
    ///   retry HTTP requests.
    /// - [`ConnectionConfig::max_response_size`] limits HTTP response bodies and IPC messages, and
    ///   WS messages unless [`ConnectionConfig::ws_config`] is set.
    /// - IPC ignores all other configuration.
    pub async fn connect_boxed_with(
        &self,
        config: ConnectionConfig,
    ) -> Result<BoxTransport, TransportError> {
        let _ = &config; // Suppress unused warning when no transport reads it
        match self {
            // reqwest is enabled, hyper is not
            #[cfg(all(
                not(feature = "hyper"),
                feature = "reqwest",
                not(all(target_os = "wasi", target_env = "p1"))
            ))]
            Self::Http(url) => {
                let (url, auth) = http_auth(url, &config)?;
                let mut builder = reqwest::Client::builder();
                if let Some(auth) = auth {
                    // Default headers are only added when the request does not set them.
                    builder = builder.default_headers(header::HeaderMap::from_iter([(
                        header::AUTHORIZATION,
                        auth,
                    )]));
                }
                let client = builder.build().map_err(TransportErrorKind::custom)?;
                Ok(alloy_transport::Transport::boxed(
                    alloy_transport_http::Http::with_client(client, url)
                        .with_settings(config.http_settings()),
                ))
            }

            #[cfg(all(
                not(feature = "hyper"),
                feature = "reqwest",
                all(target_os = "wasi", target_env = "p1")
            ))]
            Self::Http(_) => Err(TransportErrorKind::custom_str(
                "reqwest HTTP transport is not supported on wasm32-wasip1",
            )),

            // hyper takes precedence over reqwest when enabled
            #[cfg(feature = "hyper")]
            Self::Http(url) => {
                let (url, auth) = http_auth(url, &config)?;
                let settings = config.http_settings();
                let Some(auth) = auth else {
                    return Ok(alloy_transport::Transport::boxed(
                        alloy_transport_http::HyperTransport::new_hyper(url)
                            .with_settings(settings),
                    ));
                };
                // Only add the header when the request does not set it.
                let client = alloy_transport_http::HyperClient::new().layer(
                    tower::util::MapRequestLayer::new(
                        move |mut req: alloy_transport_http::hyper::Request<_>| {
                            req.headers_mut()
                                .entry(header::AUTHORIZATION)
                                .or_insert_with(|| auth.clone());
                            req
                        },
                    ),
                );
                Ok(alloy_transport::Transport::boxed(
                    alloy_transport_http::Http::with_client(client, url).with_settings(settings),
                ))
            }

            #[cfg(feature = "ws-base")]
            Self::Ws(url, existing_auth) => {
                let mut ws_connect = alloy_transport_ws::WsConnect::new(url.clone());

                // Apply authentication: prioritize config over existing URL auth
                let auth = config.auth.as_ref().or(existing_auth.as_ref());
                #[cfg(not(target_family = "wasm"))]
                if let Some(auth) = auth {
                    ws_connect = ws_connect.with_auth(auth.clone());
                }
                #[cfg(target_family = "wasm")]
                let _ = auth; // Suppress unused warning on WASM

                // Apply WebSocket-specific config, or limit messages to the max response size.
                #[cfg(not(target_family = "wasm"))]
                {
                    let ws_config = config.ws_config.unwrap_or_else(|| {
                        alloy_transport_ws::WebSocketConfig::default()
                            .max_message_size(config.max_response_size)
                            .max_frame_size(config.max_response_size)
                    });
                    ws_connect = ws_connect.with_config(ws_config);
                }

                // Apply retry configuration
                if let Some(max_retries) = config.max_retries {
                    ws_connect = ws_connect.with_max_retries(max_retries);
                }
                if let Some(retry_interval) = config.retry_interval {
                    ws_connect = ws_connect.with_retry_interval(retry_interval);
                }

                ws_connect.into_service().await.map(alloy_transport::Transport::boxed)
            }

            #[cfg(feature = "ipc")]
            Self::Ipc(path) => alloy_transport_ipc::IpcConnect::new(path.to_owned())
                .with_max_response_size(config.max_response_size)
                .into_service()
                .await
                .map(alloy_transport::Transport::boxed),

            #[cfg(not(any(
                feature = "reqwest",
                feature = "hyper",
                feature = "ws-base",
                feature = "ipc"
            )))]
            _ => Err(TransportErrorKind::custom_str(
                "No transports enabled. Enable one of: reqwest, hyper, ws, ipc",
            )),
        }
    }

    /// Tries to parse the given string as an HTTP URL.
    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    pub fn try_as_http(s: &str) -> Result<Self, TransportError> {
        let url = if s.starts_with("localhost:") || s.parse::<std::net::SocketAddr>().is_ok() {
            let s = format!("http://{s}");
            url::Url::parse(&s)
        } else {
            url::Url::parse(s)
        }
        .map_err(TransportErrorKind::custom)?;

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            let msg = format!("invalid URL scheme: {scheme}; expected `http` or `https`");
            return Err(TransportErrorKind::custom_str(&msg));
        }

        Ok(Self::Http(url))
    }

    /// Tries to parse the given string as a WebSocket URL.
    #[cfg(feature = "ws-base")]
    pub fn try_as_ws(s: &str) -> Result<Self, TransportError> {
        let url = if s.starts_with("localhost:") || s.parse::<std::net::SocketAddr>().is_ok() {
            let s = format!("ws://{s}");
            url::Url::parse(&s)
        } else {
            url::Url::parse(s)
        }
        .map_err(TransportErrorKind::custom)?;

        let scheme = url.scheme();
        if scheme != "ws" && scheme != "wss" {
            let msg = format!("invalid URL scheme: {scheme}; expected `ws` or `wss`");
            return Err(TransportErrorKind::custom_str(&msg));
        }

        let auth = alloy_transport::Authorization::extract_from_url(&url);

        Ok(Self::Ws(url, auth))
    }

    /// Tries to parse the given string as an IPC path, returning an error if
    /// the path does not exist.
    #[cfg(feature = "ipc")]
    pub fn try_as_ipc(s: &str) -> Result<Self, TransportError> {
        let original = s;
        let s = s.strip_prefix("file://").or_else(|| s.strip_prefix("ipc://")).unwrap_or(s);

        // Check if it exists.
        let path = std::path::Path::new(s);
        let _meta = path.metadata().map_err(|e| {
            let msg = if original == s {
                format!("failed to read IPC path '{}': {e}", path.display())
            } else {
                format!("failed to read IPC path '{}' from '{original}': {e}", path.display())
            };
            TransportErrorKind::custom_str(&msg)
        })?;

        Ok(Self::Ipc(path.to_path_buf()))
    }
}

/// Returns the URL and `Authorization` header value to use for an HTTP connection.
///
/// [`ConnectionConfig::auth`] takes precedence over credentials in the URL, as it does for WS. When
/// it is set, the userinfo is removed from the URL, because reqwest would otherwise send it as a
/// request header that wins over the configured one.
#[cfg(any(
    feature = "hyper",
    all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1")))
))]
fn http_auth(
    url: &url::Url,
    config: &ConnectionConfig,
) -> Result<(url::Url, Option<header::HeaderValue>), TransportError> {
    let mut url = url.clone();
    let Some(auth) = &config.auth else { return Ok((url, None)) };

    // These only fail for URLs that cannot carry credentials, which have none to remove.
    let _ = url.set_username("");
    let _ = url.set_password(None);

    let mut value =
        header::HeaderValue::from_str(&auth.to_string()).map_err(TransportErrorKind::custom)?;
    value.set_sensitive(true);
    Ok((url, Some(value)))
}

impl FromStr for BuiltInConnectionString {
    type Err = RpcError<TransportErrorKind>;

    #[allow(clippy::let_and_return)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = Err(TransportErrorKind::custom_str(&format!(
            "No transports enabled. Enable one of: reqwest, hyper, ws, ipc. Connection info: '{s}'"
        )));
        #[cfg(any(feature = "reqwest", feature = "hyper"))]
        let res = res.or_else(|_| Self::try_as_http(s));
        #[cfg(feature = "ws-base")]
        let res = res.or_else(|_| Self::try_as_ws(s));
        #[cfg(feature = "ipc")]
        let res = res.or_else(|_| Self::try_as_ipc(s));
        res
    }
}

/// Configuration for connecting to built-in transports.
///
/// Provides a flexible way to configure various aspects of the connection,
/// including authentication, retry behavior, resource limits, and transport-specific settings.
///
/// By default, responses are limited to [`DEFAULT_MAX_RESPONSE_SIZE`] bytes.
///
/// [`DEFAULT_MAX_RESPONSE_SIZE`]: alloy_transport::DEFAULT_MAX_RESPONSE_SIZE
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ConnectionConfig {
    /// Authorization header for authenticated connections.
    ///
    /// Applied to HTTP requests and to the WebSocket handshake, in place of any credentials in the
    /// URL. Ignored for IPC.
    pub auth: Option<alloy_transport::Authorization>,
    /// Maximum number of connection retries.
    ///
    /// Only used for WebSocket reconnects. Ignored for HTTP and IPC.
    pub max_retries: Option<u32>,
    /// Base interval between connection retries.
    ///
    /// WebSocket reconnect retries use capped exponential backoff from this base interval.
    /// Ignored for HTTP and IPC.
    pub retry_interval: Option<Duration>,
    /// Maximum size of a single response in bytes, or `None` for no limit.
    ///
    /// Applied to HTTP response bodies, IPC messages, and the maximum WebSocket message and frame
    /// size. A [`Self::ws_config`] takes precedence for WebSocket. Not applied to WebSocket on
    /// WASM. Defaults to [`DEFAULT_MAX_RESPONSE_SIZE`].
    ///
    /// [`DEFAULT_MAX_RESPONSE_SIZE`]: alloy_transport::DEFAULT_MAX_RESPONSE_SIZE
    pub max_response_size: Option<usize>,
    /// WebSocket-specific configuration.
    ///
    /// Takes precedence over [`Self::max_response_size`] for the maximum message and frame size.
    #[cfg(all(feature = "ws-base", not(target_family = "wasm")))]
    pub ws_config: Option<alloy_transport_ws::WebSocketConfig>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionConfig {
    /// Create a new configuration with no authentication, retry, or WebSocket settings, and a
    /// maximum response size of [`DEFAULT_MAX_RESPONSE_SIZE`] bytes.
    ///
    /// [`DEFAULT_MAX_RESPONSE_SIZE`]: alloy_transport::DEFAULT_MAX_RESPONSE_SIZE
    pub const fn new() -> Self {
        Self {
            auth: None,
            max_retries: None,
            retry_interval: None,
            max_response_size: Some(alloy_transport::DEFAULT_MAX_RESPONSE_SIZE),
            #[cfg(all(feature = "ws-base", not(target_family = "wasm")))]
            ws_config: None,
        }
    }

    /// Set the authorization header.
    pub fn with_auth(mut self, auth: alloy_transport::Authorization) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set the maximum number of retries.
    ///
    /// Only used for WebSocket reconnects. Ignored for HTTP and IPC.
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set the base retry interval.
    ///
    /// WebSocket reconnect retries use capped exponential backoff from this base interval.
    /// Ignored for HTTP and IPC.
    pub const fn with_retry_interval(mut self, retry_interval: Duration) -> Self {
        self.retry_interval = Some(retry_interval);
        self
    }

    /// Set the maximum size of a single response in bytes, or `None` for no limit.
    ///
    /// See [`Self::max_response_size`].
    pub const fn with_max_response_size(mut self, max_response_size: Option<usize>) -> Self {
        self.max_response_size = max_response_size;
        self
    }

    /// Set the WebSocket configuration.
    ///
    /// Takes precedence over [`Self::max_response_size`] for the maximum message and frame size.
    #[cfg(all(feature = "ws-base", not(target_family = "wasm")))]
    pub const fn with_ws_config(mut self, config: alloy_transport_ws::WebSocketConfig) -> Self {
        self.ws_config = Some(config);
        self
    }

    /// Returns the HTTP transport settings for this configuration.
    #[cfg(any(
        feature = "hyper",
        all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1")))
    ))]
    const fn http_settings(&self) -> alloy_transport_http::HttpTransportSettings {
        alloy_transport_http::HttpTransportSettings::new()
            .with_max_response_size(self.max_response_size)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use similar_asserts::assert_eq;
    use url::Url;

    #[test]
    fn test_parsing_urls() {
        assert_eq!(
            BuiltInConnectionString::from_str("http://localhost:8545").unwrap(),
            BuiltInConnectionString::Http("http://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInConnectionString::from_str("localhost:8545").unwrap(),
            BuiltInConnectionString::Http("http://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInConnectionString::from_str("https://localhost:8545").unwrap(),
            BuiltInConnectionString::Http("https://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInConnectionString::from_str("http://127.0.0.1:8545").unwrap(),
            BuiltInConnectionString::Http("http://127.0.0.1:8545".parse::<Url>().unwrap())
        );

        assert_eq!(
            BuiltInConnectionString::from_str("http://localhost").unwrap(),
            BuiltInConnectionString::Http("http://localhost".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInConnectionString::from_str("127.0.0.1:8545").unwrap(),
            BuiltInConnectionString::Http("http://127.0.0.1:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInConnectionString::from_str("http://user:pass@example.com").unwrap(),
            BuiltInConnectionString::Http("http://user:pass@example.com".parse::<Url>().unwrap())
        );
    }

    #[test]
    #[cfg(feature = "ws-base")]
    fn test_parsing_ws() {
        use alloy_transport::Authorization;

        assert_eq!(
            BuiltInConnectionString::from_str("ws://localhost:8545").unwrap(),
            BuiltInConnectionString::Ws("ws://localhost:8545".parse::<Url>().unwrap(), None)
        );
        assert_eq!(
            BuiltInConnectionString::from_str("wss://localhost:8545").unwrap(),
            BuiltInConnectionString::Ws("wss://localhost:8545".parse::<Url>().unwrap(), None)
        );
        assert_eq!(
            BuiltInConnectionString::from_str("ws://127.0.0.1:8545").unwrap(),
            BuiltInConnectionString::Ws("ws://127.0.0.1:8545".parse::<Url>().unwrap(), None)
        );

        assert_eq!(
            BuiltInConnectionString::from_str("ws://alice:pass@127.0.0.1:8545").unwrap(),
            BuiltInConnectionString::Ws(
                "ws://alice:pass@127.0.0.1:8545".parse::<Url>().unwrap(),
                Some(Authorization::basic("alice", "pass"))
            )
        );
    }

    #[test]
    #[cfg(feature = "ipc")]
    #[cfg_attr(windows, ignore = "TODO: windows IPC")]
    fn empty_ipc_path_error_includes_path() {
        let err = BuiltInConnectionString::try_as_ipc("ipc://").unwrap_err().to_string();

        assert!(
            err.contains("failed to read IPC path '' from 'ipc://'"),
            "unexpected IPC path error: {err}"
        );
    }

    #[test]
    #[cfg(feature = "ipc")]
    #[cfg_attr(windows, ignore = "TODO: windows IPC")]
    fn test_parsing_ipc() {
        use alloy_node_bindings::Anvil;

        // Spawn an Anvil instance to create an IPC socket, as it's different from a normal file.
        let temp_dir = tempfile::tempdir().unwrap();
        let ipc_path = temp_dir.path().join("anvil.ipc");
        let ipc_arg = format!("--ipc={}", ipc_path.display());
        let _anvil = Anvil::new().arg(ipc_arg).spawn();
        let path_str = ipc_path.to_str().unwrap();

        assert_eq!(
            BuiltInConnectionString::from_str(&format!("ipc://{path_str}")).unwrap(),
            BuiltInConnectionString::Ipc(ipc_path.clone())
        );

        assert_eq!(
            BuiltInConnectionString::from_str(&format!("file://{path_str}")).unwrap(),
            BuiltInConnectionString::Ipc(ipc_path.clone())
        );

        assert_eq!(
            BuiltInConnectionString::from_str(ipc_path.to_str().unwrap()).unwrap(),
            BuiltInConnectionString::Ipc(ipc_path.clone())
        );
    }

    /// Serves one JSON-RPC response on a local socket and returns the raw request it received.
    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    async fn serve_once() -> (String, tokio::task::JoinHandle<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Read the whole request before responding: closing the socket with unread bytes
            // makes the client see a connection reset instead of the response.
            let mut buf = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert_ne!(n, 0, "connection closed before the request was complete");
                buf.extend_from_slice(&chunk[..n]);
                let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else { continue };
                let head = String::from_utf8_lossy(&buf[..end]).to_lowercase();
                let len = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .map_or(0, |len| len.trim().parse::<usize>().unwrap());
                if buf.len() >= end + 4 + len {
                    break;
                }
            }
            let body = r#"{"jsonrpc":"2.0","id":0,"result":"0x1"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&buf).to_lowercase()
        });
        (url, handle)
    }

    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    async fn send(url: &str, config: ConnectionConfig, header: Option<&'static str>) {
        use alloy_json_rpc::{Id, Request, RequestPacket};
        use tower::Service;

        let mut transport = BuiltInConnectionString::connect_with(url, config).await.unwrap();
        let mut req = Request::new("eth_blockNumber", Id::Number(0), ());
        if let Some(header) = header {
            req.meta
                .headers_mut()
                .insert(http::header::AUTHORIZATION, http::HeaderValue::from_static(header));
        }
        transport.call(RequestPacket::Single(req.serialize().unwrap())).await.unwrap();
    }

    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    #[tokio::test]
    async fn http_applies_config_auth() {
        let (url, server) = serve_once().await;
        let config =
            ConnectionConfig::new().with_auth(alloy_transport::Authorization::bearer("t0k"));
        send(&url, config, None).await;
        let request = server.await.unwrap();
        assert!(request.contains("\r\nauthorization: bearer t0k\r\n"), "{request}");
    }

    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    #[tokio::test]
    async fn http_request_header_overrides_config_auth() {
        let (url, server) = serve_once().await;
        let config =
            ConnectionConfig::new().with_auth(alloy_transport::Authorization::bearer("t0k"));
        send(&url, config, Some("Bearer other")).await;
        let request = server.await.unwrap();
        assert!(request.contains("\r\nauthorization: bearer other\r\n"), "{request}");
        assert_eq!(request.matches("authorization:").count(), 1, "{request}");
    }

    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    #[tokio::test]
    async fn http_without_auth_sends_no_authorization() {
        let (url, server) = serve_once().await;
        send(&url, ConnectionConfig::new(), None).await;
        let request = server.await.unwrap();
        assert!(!request.contains("authorization:"), "{request}");
    }

    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    #[tokio::test]
    async fn http_config_auth_overrides_url_credentials() {
        let (url, server) = serve_once().await;
        let url = url.replacen("http://", "http://user:pass@", 1);
        let config =
            ConnectionConfig::new().with_auth(alloy_transport::Authorization::bearer("t0k"));
        send(&url, config, None).await;
        let request = server.await.unwrap();
        assert!(request.contains("\r\nauthorization: bearer t0k\r\n"), "{request}");
        assert_eq!(request.matches("authorization:").count(), 1, "{request}");
    }

    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    #[tokio::test]
    async fn http_rejects_invalid_config_auth() {
        let config =
            ConnectionConfig::new().with_auth(alloy_transport::Authorization::raw("t0k\nen"));
        let res = BuiltInConnectionString::connect_with("http://127.0.0.1:8545", config).await;
        assert!(res.is_err());
    }
}
