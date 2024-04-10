use std::str::FromStr;

use alloy_json_rpc::RpcError;
use alloy_transport::{BoxTransport, BoxTransportConnect, TransportError, TransportErrorKind};

#[cfg(feature = "pubsub")]
use alloy_pubsub::PubSubConnect;

/// Connection string for built-in transports.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuiltInConnectionString {
    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    /// HTTP transport.
    Http(url::Url),
    #[cfg(feature = "ws")]
    /// WebSocket transport.
    Ws(url::Url, Option<alloy_transport::Authorization>),
    #[cfg(feature = "ipc")]
    /// IPC transport.
    Ipc(String),
}

impl BoxTransportConnect for BuiltInConnectionString {
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

    fn get_boxed_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, BoxTransport, TransportError> {
        Box::pin(async move { self.connect_boxed().await })
    }
}

impl BuiltInConnectionString {
    /// Connect with the given connection string.
    ///
    /// # Notes
    ///
    /// - If `hyper` feature is enabled
    /// - WS will extract auth, however, auth is disabled for wasm.
    pub async fn connect_boxed(&self) -> Result<BoxTransport, TransportError> {
        // NB:
        // HTTP match will always produce hyper if the feature is enabled.
        // WS match arms are fall-through. Auth arm is disabled for wasm.
        match self {
            // reqwest is enabled, hyper is not
            #[cfg(all(not(feature = "hyper"), feature = "reqwest"))]
            Self::Http(url) => {
                Ok(
                    alloy_transport::Transport::boxed(
                        alloy_transport_http::Http::<reqwest::Client>::new(url.clone())
                    )
                )
            },

            // hyper is enabled, reqwest is not
            #[cfg(feature = "hyper")]
            Self::Http(_) => Err(TransportErrorKind::custom_str(
                "hyper not supported by BuiltinConnectionString. Please instantiate a hyper client manually",
            )),

            #[cfg(all(not(target_arch = "wasm32"), feature = "ws"))]
            Self::Ws(url, Some(auth)) => {
                alloy_transport_ws::WsConnect::with_auth(url.clone(), Some(auth.clone()))
                    .into_service()
                    .await
                    .map(alloy_transport::Transport::boxed)
            }

            #[cfg(feature = "ws")]
            Self::Ws(url, _) => alloy_transport_ws::WsConnect::new(url.clone())
                .into_service()
                .await
                .map(alloy_transport::Transport::boxed),

            #[cfg(feature = "ipc")]
            Self::Ipc(path) => alloy_transport_ipc::IpcConnect::new(path.to_owned())
                .into_service()
                .await
                .map(alloy_transport::Transport::boxed),

            #[cfg(not(any(feature = "reqwest", feature = "hyper", feature = "ws", feature = "ipc")))]
            _ => Err(TransportErrorKind::custom_str(
                "No transports enabled. Enable one of: reqwest, hyper, ws, ipc",
            )),
        }
    }

    /// Tries to parse the given string as an HTTP URL.
    #[cfg(any(feature = "reqwest", feature = "hyper"))]
    pub fn try_as_http(s: &str) -> Result<Self, TransportError> {
        let url = if s.starts_with("localhost:") || s.parse::<std::net::SocketAddr>().is_ok() {
            let s = format!("http://{}", s);
            url::Url::parse(&s)
        } else {
            url::Url::parse(s)
        }
        .map_err(TransportErrorKind::custom)?;

        if url.scheme() != "http" && url.scheme() != "https" {
            Err(TransportErrorKind::custom_str("Invalid scheme. Expected http or https"))?;
        }

        Ok(Self::Http(url))
    }

    /// Tries to parse the given string as a WebSocket URL.
    #[cfg(feature = "ws")]
    pub fn try_as_ws(s: &str) -> Result<Self, TransportError> {
        let url = if s.starts_with("localhost:") || s.parse::<std::net::SocketAddr>().is_ok() {
            let s = format!("ws://{}", s);
            url::Url::parse(&s)
        } else {
            url::Url::parse(s)
        }
        .map_err(TransportErrorKind::custom)?;

        if url.scheme() != "ws" && url.scheme() != "wss" {
            Err(TransportErrorKind::custom_str("Invalid scheme. Expected ws or wss"))?;
        }

        let auth = alloy_transport::Authorization::extract_from_url(&url);

        Ok(Self::Ws(url, auth))
    }

    /// Tries to parse the given string as an IPC path, returning an error if
    /// the path does not exist.
    #[cfg(feature = "ipc")]
    pub fn try_as_ipc(s: &str) -> Result<Self, TransportError> {
        let s = s.strip_prefix("file://").unwrap_or(s);
        let s = s.strip_prefix("ipc://").unwrap_or(s);

        // Check if s is a path and it exists
        let path = std::path::Path::new(&s);

        path.is_file().then_some(Self::Ipc(s.to_string())).ok_or_else(|| {
            TransportErrorKind::custom_str(&format!(
                "Invalid IPC path. File does not exist: {}",
                path.display()
            ))
        })
    }
}

impl FromStr for BuiltInConnectionString {
    type Err = RpcError<TransportErrorKind>;

    fn from_str(s: &str) -> Result<BuiltInConnectionString, Self::Err> {
        let res = Err(TransportErrorKind::custom_str(&format!(
            "No transports enabled. Enable one of: reqwest, hyper, ws, ipc. Connection info: '{}'",
            s
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

#[cfg(test)]
mod test {
    use super::*;
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
            BuiltInConnectionString::from_str("localhost:8545").unwrap(),
            BuiltInConnectionString::Http("http://localhost:8545".parse::<Url>().unwrap())
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
    fn test_parsing_ipc() {
        // Create a temp file and save it.
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file = temp_dir.path().join("reth.ipc");

        // Save it
        std::fs::write(&temp_file, "reth ipc").unwrap();
        assert!(temp_file.is_file());
        let temp_file_str = temp_file.to_str().unwrap().to_string();

        assert_eq!(
            BuiltInConnectionString::from_str(&format!("ipc://{}", temp_file_str)).unwrap(),
            BuiltInConnectionString::Ipc(temp_file_str.clone())
        );

        assert_eq!(
            BuiltInConnectionString::from_str(&format!("file://{}", temp_file_str)).unwrap(),
            BuiltInConnectionString::Ipc(temp_file_str.clone())
        );

        assert_eq!(
            BuiltInConnectionString::from_str(temp_file.to_str().unwrap()).unwrap(),
            BuiltInConnectionString::Ipc(temp_file_str.clone())
        );

        // Delete the written file after test
        std::fs::remove_file(temp_file).unwrap();
        assert_eq!(
            BuiltInConnectionString::from_str("http://user:pass@example.com").unwrap(),
            BuiltInConnectionString::Http("http://user:pass@example.com".parse::<Url>().unwrap())
        );
    }
}
