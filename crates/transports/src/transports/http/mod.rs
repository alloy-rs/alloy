#[cfg(feature = "hyper")]
mod hyper;

#[cfg(feature = "reqwest")]
mod reqwest;

use crate::client::RpcClient;

use std::{str::FromStr, sync::atomic::AtomicU64};
use url::Url;

/// An Http transport.
///
/// The user must provide an internal http client and a URL to which to
/// connect. It implements `Service<Box<RawValue>>`, and can be used directly
/// by an [`RpcClient`].
///
/// Currently supported clients are:
#[cfg_attr(feature = "reqwest", doc = " - [`::reqwest::Client`]")]
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
    pub fn with_client(client: T, url: Url) -> Self {
        Self { client, url }
    }

    /// True if the connection has no hostname, or the hostname is `localhost`
    /// or `127.0.0.1`.
    pub fn is_local(&self) -> bool {
        self.url
            .host_str()
            .map_or(true, |host| host == "localhost" || host == "127.0.0.1")
    }
}

impl<T> RpcClient<Http<T>>
where
    T: Default,
{
    /// Create a new [`RpcClient`] from a URL.
    pub fn new_http(url: Url) -> Self {
        let transport = Http::new(url);
        let is_local = transport.is_local();
        Self {
            transport,
            is_local,
            id: AtomicU64::new(0),
        }
    }
}

impl<T> FromStr for RpcClient<Http<T>>
where
    T: Default,
{
    type Err = <Url as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self::new_http)
    }
}
