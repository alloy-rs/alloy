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

use alloy_transport::{utils::guess_local_url, DEFAULT_MAX_RESPONSE_SIZE};
use core::str::FromStr;
use std::{marker::PhantomData, time::Duration};
use url::Url;

/// The default timeout for establishing a connection, used by the default HTTP clients.
///
/// This is set on the clients created by [`Http::new`] and `HyperClient::new`. It is not applied
/// on WASM, where the browser controls connection timeouts.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// The default read timeout, used by the default HTTP clients.
///
/// The request fails if the server sends no response headers within this time, or if it then
/// stops sending the body for this long. It is not a deadline for the whole request, so a large
/// response that keeps arriving is never cut off; see
/// [`HttpTransportSettings::with_request_timeout`] for that.
///
/// This is set on the clients created by [`Http::new`] and `HyperClient::new`. It is not applied
/// on WASM, where the browser controls read timeouts.
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(120);

/// The maximum number of bytes of an HTTP error response body kept in a [`HttpError`].
///
/// [`HttpError`]: alloy_transport::HttpError
#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
const MAX_ERROR_BODY_SIZE: usize = 4 * 1024;

#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
fn json_rpc_error_response(body: &[u8]) -> Option<alloy_json_rpc::ResponsePacket> {
    let response = serde_json::from_slice::<alloy_json_rpc::ResponsePacket>(body).ok()?;
    response.is_error().then_some(response)
}

#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
fn http_error_response(
    status: u16,
    body: &[u8],
    retry_after: Option<Duration>,
) -> alloy_transport::TransportResult<alloy_json_rpc::ResponsePacket> {
    if let Some(response) = json_rpc_error_response(body) {
        return Ok(response);
    }

    Err(alloy_transport::TransportErrorKind::http_error_with_retry_after(
        status,
        error_body(body),
        retry_after,
    ))
}

/// Converts an HTTP error response body to a string, truncated to [`MAX_ERROR_BODY_SIZE`] bytes.
#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
fn error_body(body: &[u8]) -> String {
    if body.len() <= MAX_ERROR_BODY_SIZE {
        return String::from_utf8_lossy(body).into_owned();
    }
    // A character split at the cut is replaced with U+FFFD.
    let (head, rest) = body.split_at(MAX_ERROR_BODY_SIZE);
    format!("{}... <{} more bytes truncated>", String::from_utf8_lossy(head), rest.len())
}

/// Parses the value of a `Retry-After` header, delay-seconds form only.
#[cfg(any(
    all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))),
    all(not(target_family = "wasm"), feature = "hyper")
))]
fn parse_retry_after(value: Option<&str>) -> Option<std::time::Duration> {
    value.and_then(|value| value.trim().parse().ok()).map(std::time::Duration::from_secs)
}

/// Converts a received HTTP response into a [`ResponsePacket`](alloy_json_rpc::ResponsePacket).
///
/// The body is inspected regardless of the status code, as an error response may carry a JSON-RPC
/// error.
#[cfg(any(
    all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))),
    all(not(target_family = "wasm"), feature = "hyper")
))]
fn handle_response<B, E>(
    status: u16,
    retry_after: Option<std::time::Duration>,
    body: Result<B, body::BodyError<E>>,
) -> alloy_transport::TransportResult<alloy_json_rpc::ResponsePacket>
where
    B: AsRef<[u8]>,
    E: std::error::Error + Send + Sync + 'static,
{
    let is_success = (200..300).contains(&status);

    let body = match body {
        Ok(body) => body,
        // A failed body read on an error response still carries retryable metadata.
        Err(err) if !is_success => {
            return Err(alloy_transport::TransportErrorKind::http_error_with_retry_after(
                status,
                format!("<failed to read response body: {err}>"),
                retry_after,
            ));
        }
        Err(err) => return Err(err.into_transport_error()),
    };
    let body = body.as_ref();

    if tracing::enabled!(tracing::Level::TRACE) {
        tracing::trace!(body = %String::from_utf8_lossy(body), "response body");
    } else {
        tracing::debug!(bytes = body.len(), "retrieved response body");
    }

    if !is_success {
        return http_error_response(status, body, retry_after);
    }

    // Deserialize a Box<RawValue> from the body. If deserialization fails, return
    // the body as a string in the error. The conversion to String
    // is lossy and may not cover all the bytes in the body.
    serde_json::from_slice(body).map_err(|err| {
        alloy_transport::TransportError::deser_err(err, String::from_utf8_lossy(body))
    })
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
/// connect. It implements `Service<RequestPacket>`, and therefore
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
    ///
    /// The client is used as is, including its timeouts. The transport has the default
    /// [`HttpTransportSettings`], which limit the size of a response.
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
///
/// These apply to any client, including one passed to [`Http::with_client`]. By default, response
/// bodies are limited to [`DEFAULT_MAX_RESPONSE_SIZE`] bytes and there is no request deadline. Use
/// [`HttpTransportSettings::unlimited`] to remove all limits.
///
/// Timeouts for connecting and reading are properties of the client instead, see
/// [`DEFAULT_CONNECT_TIMEOUT`] and [`DEFAULT_READ_TIMEOUT`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpTransportSettings {
    max_response_size: Option<usize>,
    request_timeout: Option<Duration>,
}

impl Default for HttpTransportSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransportSettings {
    /// Create the default settings: a maximum response size of [`DEFAULT_MAX_RESPONSE_SIZE`] bytes
    /// and no request timeout.
    pub const fn new() -> Self {
        Self { max_response_size: Some(DEFAULT_MAX_RESPONSE_SIZE), request_timeout: None }
    }

    /// Create settings without any limits.
    ///
    /// The transport then buffers response bodies of any size, so this should only be used with
    /// trusted endpoints.
    pub const fn unlimited() -> Self {
        Self { max_response_size: None, request_timeout: None }
    }

    /// Set the maximum size of a response body in bytes, or `None` for no limit.
    ///
    /// Larger responses are rejected before JSON decoding: up front if the content length is known
    /// to be too large, otherwise as soon as the bytes read exceed the limit. Defaults to
    /// [`DEFAULT_MAX_RESPONSE_SIZE`].
    pub const fn with_max_response_size(mut self, max_response_size: Option<usize>) -> Self {
        self.max_response_size = max_response_size;
        self
    }

    /// Set a deadline for each request, from sending it until the whole response body is read.
    ///
    /// There is no deadline by default, as some responses, like `debug_trace*`, can take a long
    /// time to produce and transfer.
    pub const fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = Some(request_timeout);
        self
    }

    /// Get the maximum size of a response body in bytes, if any.
    pub const fn max_response_size(&self) -> Option<usize> {
        self.max_response_size
    }

    /// Get the request deadline, if any.
    pub const fn request_timeout(&self) -> Option<Duration> {
        self.request_timeout
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

    #[test]
    fn truncates_long_http_error_body() {
        let body = [b"<html>".as_slice(), &[b'a'; 10_000]].concat();
        let error = super::http_error_response(502, &body, None).unwrap_err();

        let TransportError::Transport(error) = error else { panic!("expected transport error") };
        let http = error.as_http_error().unwrap();
        assert_eq!(http.status, 502);
        let kept = &body[..super::MAX_ERROR_BODY_SIZE];
        let expected = format!(
            "{}... <{} more bytes truncated>",
            std::str::from_utf8(kept).unwrap(),
            body.len() - kept.len()
        );
        assert_eq!(http.body, expected);

        // A body at the limit is kept as is.
        let body = vec![b'a'; super::MAX_ERROR_BODY_SIZE];
        let error = super::http_error_response(502, &body, None).unwrap_err();
        let TransportError::Transport(error) = error else { panic!("expected transport error") };
        assert_eq!(error.as_http_error().unwrap().body.as_bytes(), body);
    }

    #[test]
    fn default_settings_are_finite() {
        use crate::HttpTransportSettings;
        use alloy_transport::DEFAULT_MAX_RESPONSE_SIZE;

        let settings = HttpTransportSettings::default();
        assert_eq!(settings, HttpTransportSettings::new());
        assert_eq!(settings.max_response_size(), Some(DEFAULT_MAX_RESPONSE_SIZE));
        assert_eq!(settings.request_timeout(), None);
        assert_eq!(
            crate::Http::with_client((), "http://localhost".parse().unwrap()).settings(),
            &settings
        );

        let unlimited = HttpTransportSettings::unlimited();
        assert_eq!(unlimited.max_response_size(), None);
        assert_eq!(unlimited.request_timeout(), None);
        assert_eq!(settings.clone().with_max_response_size(None), unlimited);

        let settings =
            settings.with_max_response_size(Some(1)).with_request_timeout(Duration::from_secs(1));
        assert_eq!(settings.max_response_size(), Some(1));
        assert_eq!(settings.request_timeout(), Some(Duration::from_secs(1)));
    }

    #[cfg(not(target_family = "wasm"))]
    mod limits {
        use crate::{Http, HttpTransportSettings};
        use alloy_json_rpc::{Id, Request, RequestPacket, ResponsePacket};
        use alloy_transport::{TransportError, DEFAULT_MAX_RESPONSE_SIZE};
        use std::{future::Future, time::Duration};
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpStream,
        };
        use tower::Service;
        use url::Url;

        const BODY: &str = r#"{"jsonrpc":"2.0","id":0,"result":"0x1"}"#;
        const CHUNKED_HEAD: &str = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                                    transfer-encoding: chunked\r\n\r\n";

        /// Accepts the first connection on a local socket, reads the request, and hands the stream
        /// to `respond`.
        async fn serve<F, Fut>(respond: F) -> Url
        where
            F: FnOnce(TcpStream) -> Fut + Send + 'static,
            Fut: Future<Output = ()> + Send,
        {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap()).parse().unwrap();
            tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let _ = stream.read(&mut [0; 4096]).await;
                respond(stream).await;
            });
            url
        }

        /// Serves `response`, then keeps the connection open until the client closes it.
        async fn serve_once(response: String) -> Url {
            serve(|mut stream| async move {
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.read(&mut [0; 4096]).await;
            })
            .await
        }

        /// Serves `head`, then keeps the connection open without sending anything else.
        async fn serve_stalled(head: &'static str) -> Url {
            serve(move |mut stream| async move {
                let _ = stream.write_all(head.as_bytes()).await;
                let _ = stream.read(&mut [0; 4096]).await;
            })
            .await
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
            format!("{CHUNKED_HEAD}{:x}\r\n{a}\r\n{:x}\r\n{b}\r\n0\r\n\r\n", a.len(), b.len())
        }

        async fn call<C>(mut transport: Http<C>) -> Result<ResponsePacket, TransportError>
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            let req = Request::new("eth_blockNumber", Id::Number(0), ()).serialize().unwrap();
            // Fail instead of hanging if a limit is not enforced.
            tokio::time::timeout(
                Duration::from_secs(10),
                transport.call(RequestPacket::Single(req)),
            )
            .await
            .expect("request did not complete")
        }

        /// Returns whether the error or one of its sources is a timeout.
        fn is_timeout(err: &TransportError) -> bool {
            let mut source: Option<&dyn std::error::Error> = Some(err);
            while let Some(err) = source {
                if err.to_string().contains("timed out") {
                    return true;
                }
                source = err.source();
            }
            false
        }

        fn with_settings<C>(
            new: fn(Url) -> Http<C>,
            settings: HttpTransportSettings,
        ) -> impl Fn(Url) -> Http<C> {
            move |url| new(url).with_settings(settings.clone())
        }

        async fn assert_max_response_size<C>(new: fn(Url) -> Http<C>)
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            let with_max =
                |max| with_settings(new, HttpTransportSettings::new().with_max_response_size(max));
            let unlimited = with_settings(new, HttpTransportSettings::unlimited());

            for response in [content_length_response(), chunked_response()] {
                call(new(serve_once(response.clone()).await)).await.unwrap();
                call(with_max(Some(BODY.len()))(serve_once(response.clone()).await)).await.unwrap();
                call(unlimited(serve_once(response.clone()).await)).await.unwrap();

                let err = call(with_max(Some(BODY.len() - 1))(serve_once(response).await))
                    .await
                    .unwrap_err();
                assert!(err.to_string().contains("response body exceeds"), "{err}");
            }
        }

        /// A chunked body is rejected as soon as it exceeds the limit, before the end of the body
        /// and before JSON decoding.
        async fn assert_rejects_endless_chunked_body<C>(new: fn(Url) -> Http<C>)
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            let url = serve(|mut stream| async move {
                let _ = stream.write_all(CHUNKED_HEAD.as_bytes()).await;
                let chunk = format!("2000\r\n{}\r\n", " ".repeat(0x2000));
                while stream.write_all(chunk.as_bytes()).await.is_ok() {}
            })
            .await;
            let max = 64 * 1024;
            let transport = with_settings(
                new,
                HttpTransportSettings::new().with_max_response_size(Some(max)),
            )(url);
            let err = call(transport).await.unwrap_err();
            assert!(
                err.to_string().contains(&format!("response body exceeds {max} bytes")),
                "{err}"
            );
        }

        /// A content length above the default limit is rejected up front, unless the transport is
        /// unlimited.
        async fn assert_rejects_oversized_content_length<C>(new: fn(Url) -> Http<C>)
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            // Send a few bytes of the advertised body, then close the connection.
            let serve_oversized = || {
                serve(|mut stream| async move {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                         content-length: {}\r\n\r\n{{",
                        DEFAULT_MAX_RESPONSE_SIZE + 1
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                })
            };

            let err = call(new(serve_oversized().await)).await.unwrap_err();
            let expected = format!("response body exceeds {DEFAULT_MAX_RESPONSE_SIZE} bytes");
            assert!(err.to_string().contains(&expected), "{err}");

            let unlimited = with_settings(new, HttpTransportSettings::unlimited());
            let err = call(unlimited(serve_oversized().await)).await.unwrap_err();
            assert!(!err.to_string().contains("response body exceeds"), "{err}");
        }

        async fn assert_request_timeout<C>(new: fn(Url) -> Http<C>)
        where
            Http<C>: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
        {
            let settings =
                HttpTransportSettings::new().with_request_timeout(Duration::from_millis(100));

            // No response at all.
            let err = call(with_settings(new, settings.clone())(serve_stalled("").await))
                .await
                .unwrap_err();
            assert!(is_timeout(&err), "{err}");

            // The body stops arriving.
            let url = serve_stalled("HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\n{").await;
            let err = call(with_settings(new, settings)(url)).await.unwrap_err();
            assert!(is_timeout(&err), "{err}");
        }

        #[cfg(feature = "reqwest")]
        mod reqwest {
            use super::*;

            #[tokio::test]
            async fn max_response_size() {
                assert_max_response_size(Http::new).await;
            }

            #[tokio::test]
            async fn rejects_endless_chunked_body() {
                assert_rejects_endless_chunked_body(Http::new).await;
            }

            #[tokio::test]
            async fn rejects_oversized_content_length() {
                assert_rejects_oversized_content_length(Http::new).await;
            }

            #[tokio::test]
            async fn request_timeout() {
                assert_request_timeout(Http::new).await;
            }

            #[test]
            fn default_client_has_timeouts() {
                let builder = format!("{:?}", Http::default_client_builder());
                assert!(builder.contains("connect_timeout: 10s"), "{builder}");
                let client =
                    format!("{:?}", Http::new("http://localhost".parse().unwrap()).client());
                assert!(client.contains("read_timeout: 120s"), "{client}");
            }

            #[tokio::test]
            async fn read_timeout() {
                let new = |url| {
                    let client = Http::default_client_builder()
                        .read_timeout(Duration::from_millis(100))
                        .build()
                        .unwrap();
                    Http::with_client(client, url)
                };

                let err = call(new(serve_stalled("").await)).await.unwrap_err();
                assert!(is_timeout(&err), "{err}");
            }
        }

        #[cfg(feature = "hyper")]
        mod hyper {
            use super::*;
            use crate::HyperClient;

            #[tokio::test]
            async fn max_response_size() {
                assert_max_response_size(Http::new_hyper).await;
            }

            #[tokio::test]
            async fn rejects_endless_chunked_body() {
                assert_rejects_endless_chunked_body(Http::new_hyper).await;
            }

            #[tokio::test]
            async fn rejects_oversized_content_length() {
                assert_rejects_oversized_content_length(Http::new_hyper).await;
            }

            #[tokio::test]
            async fn request_timeout() {
                assert_request_timeout(Http::new_hyper).await;
            }

            #[test]
            fn default_client_has_read_timeout() {
                let client = HyperClient::new();
                assert_eq!(client.read_timeout(), Some(crate::DEFAULT_READ_TIMEOUT));
                let client = client.layer(tower::layer::util::Identity::new());
                assert_eq!(client.read_timeout(), Some(crate::DEFAULT_READ_TIMEOUT));
                assert_eq!(client.with_read_timeout(None).read_timeout(), None);
            }

            #[tokio::test]
            async fn read_timeout() {
                let new = |url| {
                    let client =
                        HyperClient::new().with_read_timeout(Some(Duration::from_millis(100)));
                    Http::with_client(client, url)
                };

                // No response at all.
                let err = call(new(serve_stalled("").await)).await.unwrap_err();
                assert!(
                    err.to_string().contains("no HTTP response received within 100ms"),
                    "{err}"
                );

                // The body stops arriving.
                let url = serve_stalled(
                    "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n1\r\n{\r\n",
                )
                .await;
                let err = call(new(url)).await.unwrap_err();
                assert!(
                    err.to_string().contains("response body read timed out after 100ms"),
                    "{err}"
                );
            }
        }
    }
}

#[cfg(all(
    test,
    any(
        all(feature = "reqwest", not(all(target_os = "wasi", target_env = "p1"))),
        all(not(target_family = "wasm"), feature = "hyper")
    )
))]
mod response_tests {
    use super::{body, handle_response, parse_retry_after};
    use alloy_transport::{TransportError, TransportErrorKind};
    use std::{io, time::Duration};

    const SUCCESS: &[u8] = br#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#;
    const JSON_RPC_ERROR: &[u8] =
        br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"filter not found"}}"#;

    fn transport_kind(err: TransportError) -> TransportErrorKind {
        let TransportError::Transport(kind) = err else { panic!("expected transport error") };
        kind
    }

    #[test]
    fn parses_retry_after_seconds_only() {
        assert_eq!(parse_retry_after(Some(" 52 ")), Some(Duration::from_secs(52)));
        assert_eq!(parse_retry_after(Some("Wed, 21 Oct 2015 07:28:00 GMT")), None);
        assert_eq!(parse_retry_after(None), None);
    }

    #[test]
    fn maps_responses_by_status_and_body() {
        let ok = handle_response(200, None, Ok::<_, body::BodyError<io::Error>>(SUCCESS)).unwrap();
        assert!(ok.is_success());

        let err = handle_response(200, None, Ok::<_, body::BodyError<io::Error>>(&b"not json"[..]))
            .unwrap_err();
        assert!(err.is_deser_error());

        let rpc_err =
            handle_response(500, None, Ok::<_, body::BodyError<io::Error>>(JSON_RPC_ERROR))
                .unwrap();
        assert_eq!(rpc_err.first_error_code(), Some(-32000));

        let retry_after = Some(Duration::from_secs(5));
        let err = handle_response(
            429,
            retry_after,
            Ok::<_, body::BodyError<io::Error>>(&b"slow down"[..]),
        );
        let kind = transport_kind(err.unwrap_err());
        assert_eq!(kind.retry_after(), retry_after);
        assert_eq!(kind.as_http_error().unwrap().body, "slow down");
    }

    #[test]
    fn maps_body_read_failures() {
        let read_err = || body::BodyError::Read(io::Error::other("connection reset"));

        let retry_after = Some(Duration::from_secs(3));
        let err = handle_response::<&[u8], _>(503, retry_after, Err(read_err())).unwrap_err();
        let kind = transport_kind(err);
        assert_eq!(kind.retry_after(), retry_after);
        let http = kind.as_http_error().unwrap();
        assert_eq!(http.status, 503);
        assert_eq!(http.body, "<failed to read response body: connection reset>");

        let err = handle_response::<&[u8], _>(200, None, Err(read_err())).unwrap_err();
        assert!(matches!(transport_kind(err), TransportErrorKind::Custom(_)));

        let too_large = body::BodyError::<io::Error>::TooLarge(10);
        let err = handle_response::<&[u8], _>(500, None, Err(too_large)).unwrap_err();
        let kind = transport_kind(err);
        let http = kind.as_http_error().unwrap();
        assert_eq!(http.body, "<failed to read response body: response body exceeds 10 bytes>");
    }
}
