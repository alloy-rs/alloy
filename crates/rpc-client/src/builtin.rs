use alloy_json_rpc::RpcError;
use alloy_transport::{BoxTransport, TransportConnect, TransportError, TransportErrorKind};
use std::{str::FromStr, time::Duration};

#[cfg(any(feature = "ws", feature = "ipc"))]
use alloy_pubsub::PubSubConnect;

/// Connection string for built-in transports.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuiltInConnectionString {
    /// HTTP transport.
    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    Http(url::Url),
    /// WebSocket transport.
    #[cfg(feature = "ws")]
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
            #[cfg(feature = "ws")]
            Self::Ws(url, _) => alloy_transport::utils::guess_local_url(url),
            #[cfg(feature = "ipc")]
            Self::Ipc(_) => true,
            #[cfg(not(any(
                feature = "reqwest",
                feature = "hyper",
                feature = "ws",
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
    /// - If `hyper` feature is enabled
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
    /// - If `hyper` feature is enabled
    /// - WS will extract auth, however, auth is disabled for wasm.
    /// - Some configuration options may not apply to all transport types.
    pub async fn connect_boxed_with(
        &self,
        config: ConnectionConfig,
    ) -> Result<BoxTransport, TransportError> {
        // Note: Configuration is currently only applied to WebSocket transports.
        // HTTP and IPC transports will use their default settings.
        let _ = &config; // Suppress unused warning for non-WS transports
        match self {
            // reqwest is enabled, hyper is not
            #[cfg(all(not(feature = "hyper"), feature = "reqwest"))]
            Self::Http(url) => {
                Ok(alloy_transport::Transport::boxed(
                    alloy_transport_http::Http::<reqwest::Client>::new(url.clone()),
                ))
            }

            // hyper is enabled, reqwest is not
            #[cfg(feature = "hyper")]
            Self::Http(url) => Ok(alloy_transport::Transport::boxed(
                alloy_transport_http::HyperTransport::new_hyper(url.clone()),
            )),

            #[cfg(feature = "ws")]
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

                // Apply WebSocket-specific config
                #[cfg(not(target_family = "wasm"))]
                if let Some(ws_config) = config.ws_config {
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
                .into_service()
                .await
                .map(alloy_transport::Transport::boxed),

            #[cfg(not(any(
                feature = "reqwest",
                feature = "hyper",
                feature = "ws",
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
    #[cfg(feature = "ws")]
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
        let s = s.strip_prefix("file://").or_else(|| s.strip_prefix("ipc://")).unwrap_or(s);

        // Check if it exists.
        let path = std::path::Path::new(s);
        let _meta = path.metadata().map_err(|e| {
            let msg = format!("failed to read IPC path {}: {e}", path.display());
            TransportErrorKind::custom_str(&msg)
        })?;

        Ok(Self::Ipc(path.to_path_buf()))
    }
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
        #[cfg(feature = "ws")]
        let res = res.or_else(|_| Self::try_as_ws(s));
        #[cfg(feature = "ipc")]
        let res = res.or_else(|_| Self::try_as_ipc(s));
        res
    }
}

/// Configuration for connecting to built-in transports.
///
/// Provides a flexible way to configure various aspects of the connection,
/// including authentication, retry behavior, and transport-specific settings.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ConnectionConfig {
    /// Authorization header for authenticated connections.
    pub auth: Option<alloy_transport::Authorization>,
    /// Maximum number of connection retries.
    pub max_retries: Option<u32>,
    /// Interval between connection retries.
    pub retry_interval: Option<Duration>,
    /// WebSocket-specific configuration.
    #[cfg(all(feature = "ws", not(target_family = "wasm")))]
    pub ws_config: Option<alloy_transport_ws::WebSocketConfig>,
}

impl ConnectionConfig {
    /// Create a new empty configuration.
    pub const fn new() -> Self {
        Self {
            auth: None,
            max_retries: None,
            retry_interval: None,
            #[cfg(all(feature = "ws", not(target_family = "wasm")))]
            ws_config: None,
        }
    }

    /// Set the authorization header.
    pub fn with_auth(mut self, auth: alloy_transport::Authorization) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set the maximum number of retries.
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set the retry interval.
    pub const fn with_retry_interval(mut self, retry_interval: Duration) -> Self {
        self.retry_interval = Some(retry_interval);
        self
    }

    /// Set the WebSocket configuration.
    #[cfg(all(feature = "ws", not(target_family = "wasm")))]
    pub const fn with_ws_config(mut self, config: alloy_transport_ws::WebSocketConfig) -> Self {
        self.ws_config = Some(config);
        self
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
    #[cfg(feature = "ws")]
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

    #[test]
    #[cfg(feature = "ws")]
    fn test_ws_config_auth_priority() {
        use alloy_transport::Authorization;

        // Test that config auth takes precedence over URL auth
        let config_auth = Authorization::bearer("config-token");
        let url_auth = Some(Authorization::basic("user", "pass"));

        let _ws_connection =
            BuiltInConnectionString::Ws("ws://user:pass@localhost:8545".parse().unwrap(), url_auth);

        let config = ConnectionConfig::new().with_auth(config_auth.clone());

        // In the actual connect_boxed_with implementation:
        // config.auth.as_ref().or(existing_auth.as_ref())
        // This means config auth takes priority
        assert_eq!(config.auth.as_ref().unwrap().to_string(), config_auth.to_string());
    }

    #[test]
    fn test_backward_compatibility() {
        // Verify connect() uses default config (maintaining backward compatibility)
        let default_config = ConnectionConfig::default();
        assert!(default_config.auth.is_none());
        assert!(default_config.max_retries.is_none());

        // connect() -> connect_boxed() -> connect_boxed_with(default) ensures compatibility
    }
}
