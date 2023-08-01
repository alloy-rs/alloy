use reqwest::Url;
use serde_json::value::RawValue;
use std::{future::Future, pin::Pin, str::FromStr, sync::atomic::AtomicU64, task};
use tower::Service;

use crate::{client::RpcClient, error::TransportError};

impl<T> RpcClient<Http<T>>
where
    T: Default,
{
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

#[derive(Debug, Clone)]
pub struct Http<T> {
    client: T,
    url: Url,
}

impl<T> Http<T> {
    pub fn new(url: Url) -> Self
    where
        T: Default,
    {
        Self {
            client: Default::default(),
            url,
        }
    }

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

impl Service<Box<RawValue>> for Http<reqwest::Client> {
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx).map_err(Into::into)
    }

    #[inline]
    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        let replacement = self.client.clone();
        let client = std::mem::replace(&mut self.client, replacement);

        let url = self.url.clone();

        Box::pin(async move {
            let req = req;
            let resp = client.post(url).json(&req).send().await?;
            let json = resp.text().await?;

            RawValue::from_string(json).map_err(|err| TransportError::deser_err(err, ""))
        })
    }
}
