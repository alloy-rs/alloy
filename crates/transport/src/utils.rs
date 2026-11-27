use crate::{TransportError, TransportResult};
use serde::Serialize;
use serde_json::value::{to_raw_value, RawValue};
use std::future::Future;
use url::{Host, Url};

/// Convert to a `Box<RawValue>` from a `Serialize` type, mapping the error
/// to a `TransportError`.
pub fn to_json_raw_value<S>(s: &S) -> TransportResult<Box<RawValue>>
where
    S: Serialize,
{
    to_raw_value(s).map_err(TransportError::ser_err)
}

/// Guess whether the URL is local, based on the hostname or IP.
///
/// Best-effort heuristic: returns `true` if the connection has no hostname, or
/// the host is `localhost` or an IPv4/IPv6 loopback address.
pub fn guess_local_url(s: impl AsRef<str>) -> bool {
    fn _guess_local_url(url: &str) -> bool {
        url.parse::<Url>().is_ok_and(|url| match url.host() {
            None | Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(address)) => address.is_loopback(),
            Some(Host::Ipv6(address)) => address.is_loopback(),
            Some(Host::Domain(_)) => false,
        })
    }
    _guess_local_url(s.as_ref())
}

/// Returns a copy of the URL with its username and password, if any, replaced
/// by `redacted`, for use in `Debug` output.
pub fn redact_url(url: &Url) -> Url {
    let mut url = url.clone();
    if !url.username().is_empty() {
        let _ = url.set_username("redacted");
    }
    if url.password().is_some() {
        let _ = url.set_password(Some("redacted"));
    }
    url
}

#[doc(hidden)]
pub trait Spawnable {
    /// Spawn the future as a task.
    ///
    /// In wasm32-unknown-unknown this will be a `wasm-bindgen-futures::spawn_local` call,
    /// in wasm32-wasip1 it will be a `tokio::task::spawn_local` call,
    /// and native will be a `tokio::spawn` call.
    fn spawn_task(self);
}

#[cfg(not(target_family = "wasm"))]
impl<T> Spawnable for T
where
    T: Future<Output = ()> + Send + 'static,
{
    fn spawn_task(self) {
        tokio::spawn(self);
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl<T> Spawnable for T
where
    T: Future<Output = ()> + 'static,
{
    fn spawn_task(self) {
        #[cfg(not(feature = "wasm-bindgen"))]
        panic!("The 'wasm-bindgen' feature must be enabled");

        #[cfg(feature = "wasm-bindgen")]
        wasm_bindgen_futures::spawn_local(self);
    }
}

#[cfg(all(target_family = "wasm", target_os = "wasi"))]
impl<T> Spawnable for T
where
    T: Future<Output = ()> + 'static,
{
    fn spawn_task(self) {
        tokio::task::spawn_local(self);
    }
}

#[cfg(test)]
mod tests {
    use super::{guess_local_url, redact_url};
    use url::Url;

    #[test]
    fn guesses_local_urls() {
        for (url, local) in [
            ("http://localhost:8545", true),
            ("http://127.0.0.1:8545", true),
            ("http://127.0.0.2:8545", true),
            ("http://127.255.255.254:8545", true),
            ("http://126.255.255.255:8545", false),
            ("http://128.0.0.1:8545", false),
            ("http://[::2]:8545", false),
            ("ws://[::1]:8546", true),
            ("http://[0:0:0:0:0:0:0:1]:8545", true),
            ("https://eth.example.com", false),
            ("file:///tmp/geth.ipc", true),
            ("not a url", false),
        ] {
            assert_eq!(guess_local_url(url), local, "{url}");
        }
    }

    #[test]
    fn redacts_url_credentials() {
        for (url, expected) in [
            ("https://user:pass@host/rpc", "https://redacted:redacted@host/rpc"),
            ("https://user@host/rpc", "https://redacted@host/rpc"),
            ("https://:pass@host/rpc", "https://:redacted@host/rpc"),
            ("wss://host:8546/rpc?key=1", "wss://host:8546/rpc?key=1"),
        ] {
            let url: Url = url.parse().unwrap();
            assert_eq!(redact_url(&url).as_str(), expected);
        }
    }
}
