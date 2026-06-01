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

use alloy_transport::utils::guess_local_url;
use core::str::FromStr;
use std::marker::PhantomData;
use url::Url;

#[cfg(any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper")))]
fn json_rpc_error_response(body: &[u8]) -> Option<alloy_json_rpc::ResponsePacket> {
    let response = serde_json::from_slice::<alloy_json_rpc::ResponsePacket>(body).ok()?;
    response.is_error().then_some(response)
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
}

impl<T> Http<T> {
    /// Create a new [`Http`] transport with a custom client.
    pub const fn with_client(client: T, url: Url) -> Self {
        Self { client, url }
    }

    /// Set the URL.
    pub fn set_url(&mut self, url: Url) {
        self.url = url;
    }

    /// Set the client.
    pub fn set_client(&mut self, client: T) {
        self.client = client;
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
}

#[cfg(all(test, any(feature = "reqwest", all(not(target_family = "wasm"), feature = "hyper"))))]
mod tests {
    #[test]
    fn parses_json_rpc_errors_from_http_error_body() {
        let body = br#"{
            "jsonrpc": "2.0",
            "id": 1766,
            "error": {
                "code": -32000,
                "message": "filter not found"
            }
        }"#;

        let response = super::json_rpc_error_response(body).expect("valid JSON-RPC error response");

        assert!(response.is_error());
        assert_eq!(response.first_error_code(), Some(-32000));
        assert_eq!(response.first_error_message(), Some("filter not found"));
    }

    #[test]
    fn ignores_non_json_rpc_error_body() {
        assert!(super::json_rpc_error_response(b"too many requests").is_none());
    }
}
