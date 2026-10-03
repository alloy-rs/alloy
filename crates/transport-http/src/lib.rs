#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))))]
pub use reqwest;
#[cfg(all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))))]
mod reqwest_transport;

#[cfg(all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))))]
#[doc(inline)]
pub use reqwest_transport::*;

#[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
pub use hyper;
#[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
pub use hyper_util;

mod layers;
#[cfg(all(not(target_family = "wasm"), feature = "jwt-auth"))]
pub use layers::{AuthLayer, AuthService};
#[cfg(all(not(target_family = "wasm"), feature = "traceparent"))]
pub use layers::{TraceParentLayer, TraceParentService};

#[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
mod hyper_transport;
#[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
#[doc(inline)]
pub use hyper_transport::{HyperClient, HyperResponse, HyperResponseFut, HyperTransport};

#[cfg(any(
    all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))),
    all(not(target_family = "wasm"), feature = "hyper")
))]
mod body;

use alloy_transport::utils::guess_local_url;
use core::str::FromStr;
use std::marker::PhantomData;
use url::Url;

#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
fn json_rpc_error_response(body: &[u8]) -> Option<alloy_json_rpc::ResponsePacket> {
    let response = serde_json::from_slice::<alloy_json_rpc::ResponsePacket>(body).ok()?;
    response.is_error().then_some(response)
}

#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
fn http_error_response(
    status: u16,
    body: &[u8],
    retry_after: Option<std::time::Duration>,
) -> alloy_transport::TransportResult<alloy_json_rpc::ResponsePacket> {
    if let Some(response) = json_rpc_error_response(body) {
        return Ok(response);
    }

    Err(alloy_transport::TransportErrorKind::http_error_with_retry_after(
        status,
        String::from_utf8_lossy(body).into_owned(),
        retry_after,
    ))
}

/// Connection details for an HTTP transport.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct HttpConnect<T> {
    /// The URL to connect to.
    url: Url,

    _pd: PhantomData<T>,
}

impl<T> HttpConnect<T> {
    /// Create a new [`HttpConnect`] with the given URL.
    pub const fn new(url: Url) -> Self {
        Self { url, _pd: PhantomData }
    }

    /// Get a reference to the URL.
    pub const fn url(&self) -> &Url {
        &self.url
    }
}

impl<T> FromStr for HttpConnect<T> {
    type Err = url::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.parse()?))
    }
}

/// An Http transport.
///
/// The user must provide an internal http client and a URL to which to
/// connect. It implements `Service<Box<RawValue>>`, and therefore
/// [`Transport`].
///
/// [`Transport`]: alloy_transport::Transport
///
/// Currently supported clients are:
#[cfg_attr(feature = "reqwest", doc = " - [`reqwest`](::reqwest::Client)")]
#[cfg_attr(feature = "hyper", doc = " - [`hyper`](hyper_util::client::legacy::Client)")]
#[derive(Clone, Debug)]
pub struct Http<T> {
    client: T,
    url: Url,
    settings: HttpTransportSettings,
}

impl<T> Http<T> {
    /// Create a new [`Http`] transport with a custom client.
    pub const fn with_client(client: T, url: Url) -> Self {
        Self { client, url, settings: HttpTransportSettings::new() }
    }

    /// Set the [`HttpTransportSettings`].
    pub const fn with_settings(mut self, settings: HttpTransportSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Set the URL.
    pub fn set_url(&mut self, url: Url) {
        self.url = url;
    }

    /// Set the client.
    pub fn set_client(&mut self, client: T) {
        self.client = client;
    }

    /// Set the [`HttpTransportSettings`].
    pub const fn set_settings(&mut self, settings: HttpTransportSettings) {
        self.settings = settings;
    }

    /// Guess whether the URL is local, based on the hostname.
    ///
    /// The output of this function is best-efforts, and should be checked if
    /// possible. It simply returns `true` if the connection has no hostname,
    /// or the hostname is `localhost` or `127.0.0.1`.
    pub fn guess_local(&self) -> bool {
        guess_local_url(&self.url)
    }

    /// Get a reference to the client.
    pub const fn client(&self) -> &T {
        &self.client
    }

    /// Get a reference to the URL.
    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    /// Get a reference to the [`HttpTransportSettings`].
    pub const fn settings(&self) -> &HttpTransportSettings {
        &self.settings
    }
}

/// Settings for an [`Http`] transport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HttpTransportSettings {
    max_response_size: Option<usize>,
}

impl HttpTransportSettings {
    /// Create settings without any limits.
    pub const fn new() -> Self {
        Self { max_response_size: None }
    }

    /// Set the maximum size of a response body in bytes.
    ///
    /// Larger responses are rejected while the body is read, before it is decoded. There is no
    /// limit by default.
    pub const fn with_max_response_size(mut self, max_response_size: usize) -> Self {
        self.max_response_size = Some(max_response_size);
        self
    }

    /// Get the maximum size of a response body in bytes, if any.
    pub const fn max_response_size(&self) -> Option<usize> {
        self.max_response_size
    }
}

#[cfg(all(test, any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper"))))]
mod tests {
    use alloy_transport::TransportError;
    use std::time::Duration;

    const JSON_RPC_ERROR: &[u8] = br#"{
        "jsonrpc": "2.0",
        "id": 1766,
        "error": {
            "code": -32000,
            "message": "filter not found"
        }
    }"#;

    #[test]
    fn parses_json_rpc_errors_from_http_error_body() {
        let response =
            super::json_rpc_error_response(JSON_RPC_ERROR).expect("valid JSON-RPC error response");

        assert!(response.is_error());
        assert_eq!(response.first_error_code(), Some(-32000));
        assert_eq!(response.first_error_message(), Some("filter not found"));
    }

    #[test]
    fn ignores_non_json_rpc_error_body() {
        assert!(super::json_rpc_error_response(b"too many requests").is_none());
    }

    #[test]
    fn json_rpc_error_body_takes_precedence_over_retry_after() {
        let response =
            super::http_error_response(429, JSON_RPC_ERROR, Some(Duration::from_secs(52)))
                .expect("valid JSON-RPC error response");

        assert!(response.is_error());
        assert_eq!(response.first_error_code(), Some(-32000));
    }

    #[test]
    fn non_json_rpc_body_carries_retry_after() {
        let error =
            super::http_error_response(429, b"too many requests", Some(Duration::from_secs(52)))
                .unwrap_err();

        let TransportError::Transport(error) = error else { panic!("expected transport error") };
        assert_eq!(error.retry_after(), Some(Duration::from_secs(52)));
        assert_eq!(error.as_http_error().unwrap().status, 429);
    }

    #[cfg(not(target_family = "wasm"))]
    mod max_response_size {
        use crate::{Http, HttpTransportSettings};
        use alloy_json_rpc::{Id, Request, RequestPacket, ResponsePacket};
        use alloy_transport::TransportError;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tower::Service;
        use url::Url;

        const BODY: &str = r#"{"jsonrpc":"2.0","id":0,"result":"0x1"}"#;

        /// Serves `response` to the first connection on a local socket.
        async fn serve_once(response: String) -> Url {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap()).parse().unwrap();
            tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = [0; 4096];
                let _ = stream.read(&mut buf).await;
                let _ = stream.write_all(response.as_bytes()).await;
                // Keep the connection open until the client is done with it.
                let _ = stream.read(&mut buf).await;
            });
            url
        }

        fn content_length_response() -> String {
            format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{BODY}",
                BODY.len()
            )
        }

        fn chunked_response() -> String {
            let (a, b) = BODY.split_at(BODY.len() / 2);
            format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\
                 connection: close\r\n\r\n{:x}\r\n{a}\r\n{:x}\r\n{b}\r\n0\r\n\r\n",
                a.len(),
                b.len()
            )
        }

        async fn call<C>(mut transport: Http<C>) -> Result<ResponsePacket, TransportError>
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            let req = Request::new("eth_blockNumber", Id::Number(0), ()).serialize().unwrap();
            transport.call(RequestPacket::Single(req)).await
        }

        async fn assert_max_response_size<C>(new: fn(Url) -> Http<C>)
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            let with_max = |url, max| {
                new(url).with_settings(HttpTransportSettings::new().with_max_response_size(max))
            };

            for response in [content_length_response(), chunked_response()] {
                call(new(serve_once(response.clone()).await)).await.unwrap();
                call(with_max(serve_once(response.clone()).await, BODY.len())).await.unwrap();

                let err =
                    call(with_max(serve_once(response).await, BODY.len() - 1)).await.unwrap_err();
                assert!(err.to_string().contains("response body exceeds"), "{err}");
            }
        }

        #[cfg(feature = "reqwest")]
        #[tokio::test]
        async fn reqwest_transport() {
            assert_max_response_size(Http::new).await;
        }

        #[cfg(feature = "hyper")]
        #[tokio::test]
        async fn hyper_transport() {
            assert_max_response_size(Http::new_hyper).await;
        }
    }
}
