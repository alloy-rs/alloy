#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[cfg(all(not(target_arch = "wasm32"), feature = "hyper"))]
mod hyper;

#[cfg(feature = "reqwest")]
mod reqwest;

use alloy_transport::utils::guess_local_url;
use url::Url;

/// An Http transport.
///
/// The user must provide an internal http client and a URL to which to
/// connect. It implements `Service<Box<RawValue>>`, and therefore
/// [`Transport`].
///
/// [`Transport`]: alloy_transport::Transport
///
/// Currently supported clients are:
#[cfg_attr(feature = "reqwest", doc = " - [`::reqwest::Client`]")]
#[cfg_attr(feature = "hyper", doc = " - [`::hyper::client::Client`]")]
#[derive(Debug, Clone)]
pub struct Http<T> {
    client: T,
    url: Url,
}

impl<T> Http<T> {
    /// Create a new [`Http`] transport.
    pub fn new(url: Url) -> Self
    where
        T: Default,
    {
        Self {
            client: Default::default(),
            url,
        }
    }

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
    /// The ouput of this function is best-efforts, and should be checked if
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
