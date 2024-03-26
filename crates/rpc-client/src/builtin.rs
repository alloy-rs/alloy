use std::str::FromStr;

use alloy_json_rpc::RpcError;
use alloy_transport::{
    utils::guess_local_url, BoxTransport, BoxTransportConnect, Transport, TransportError,
    TransportErrorKind,
};
use reqwest::Url;
use std::net::SocketAddr;

#[cfg(feature = "pubsub")]
use alloy_pubsub::PubSubConnect;

/// Connection string for built-in transports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltInConnectionString {
    /// HTTP transport.
    Http(Url),
    #[cfg(feature = "ws")]
    /// WebSocket transport.
    Ws(Url, Option<alloy_transport::Authorization>),
    #[cfg(feature = "ipc")]
    /// IPC transport.
    Ipc(String),
}

impl BoxTransportConnect for BuiltInConnectionString {
    fn is_local(&self) -> bool {
        match self {
            Self::Http(url) => guess_local_url(url),
            #[cfg(feature = "ws")]
            Self::Ws(url, _) => guess_local_url(url),
            #[cfg(feature = "ipc")]
            Self::Ipc(_) => true,
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
    /// - If `hyper` feature is enabled, the HTTP transport will be `hyper`.
    /// - WS will extract auth, however, auth is disabled for wasm.
    pub async fn connect_boxed(&self) -> Result<BoxTransport, TransportError> {
        // NB:
        // HTTP match will always produce hyper if the feature is enabled.
        // WS match arms are fall-through. Auth arm is disabled for wasm.
        match self {
            #[cfg(not(feature = "hyper"))]
            Self::Http(url) => {
                Ok(alloy_transport_http::Http::<reqwest::Client>::new(url.clone()).boxed())
            }

            #[cfg(feature = "hyper")]
            Self::Http(url) => {
                Ok(alloy_transport_http::Http::<hyper::Client<_>>::new(url.clone()).boxed())
            }

            #[cfg(all(not(target = "wasm"), feature = "ws"))]
            Self::Ws(url, Some(auth)) => {
                alloy_transport_ws::WsConnect::with_auth(url.clone(), Some(auth.clone()))
                    .into_service()
                    .await
                    .map(Transport::boxed)
            }

            #[cfg(feature = "ws")]
            Self::Ws(url, _) => alloy_transport_ws::WsConnect::new(url.clone())
                .into_service()
                .await
                .map(Transport::boxed),

            #[cfg(feature = "ipc")]
            Self::Ipc(path) => alloy_transport_ipc::IpcConnect::new(path.to_owned())
                .into_service()
                .await
                .map(Transport::boxed),
        }
    }

    /// Tries to parse the given string as an HTTP URL.
    pub fn try_as_http(s: &str) -> Result<Self, TransportError> {
        let url = if s.starts_with("localhost:") || s.parse::<SocketAddr>().is_ok() {
            let s = format!("http://{}", s);
            Url::parse(&s)
        } else {
            Url::parse(s)
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
        let url = if s.starts_with("localhost:") || s.parse::<SocketAddr>().is_ok() {
            let s = format!("ws://{}", s);
            Url::parse(&s)
        } else {
            Url::parse(s)
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
        // Check if s is a path and it exists
        let path = std::path::Path::new(&s);

        path.is_file()
            .then_some(Self::Ipc(s.to_string()))
            .ok_or_else(|| TransportErrorKind::custom_str("Invalid IPC path. File does not exist."))
    }
}

impl FromStr for BuiltInConnectionString {
    type Err = RpcError<TransportErrorKind>;

    fn from_str(s: &str) -> Result<BuiltInConnectionString, Self::Err> {
        let res = Self::try_as_http(s);
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
                "ws://127.0.0.1:8545".parse::<Url>().unwrap(),
                Some(Authorization::Basic("alice:bob".to_string()))
            )
        );
    }

    #[test]
    #[cfg(feature = "ipc")]
    fn test_parsing_ipc() {
        assert_eq!(
            BuiltInConnectionString::from_str("ipc:///tmp/reth.ipc").unwrap(),
            BuiltInConnectionString::Ipc("ipc:///tmp/reth.ipc".to_string())
        );

        assert_eq!(
            BuiltInConnectionString::from_str("file:///tmp/reth.ipc").unwrap(),
            BuiltInConnectionString::Ipc("file:///tmp/reth.ipc".to_string())
        );

        // Create a temp file and save it.
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file = temp_dir.path().join("reth.ipc");

        // Save it
        std::fs::write(&temp_file, "reth ipc").unwrap();

        assert_eq!(
            BuiltInConnectionString::from_str(temp_file.to_str().unwrap()).unwrap(),
            BuiltInConnectionString::Ipc(temp_file.to_str().unwrap().to_string())
        );
        // Delete the written file after test
        std::fs::remove_file(temp_file).unwrap();
        assert_eq!(
            BuiltInConnectionString::from_str("http://user:pass@example.com").unwrap(),
            BuiltInConnectionString::Http("http://user:pass@example.com".parse::<Url>().unwrap())
        );
    }
}
