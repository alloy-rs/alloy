use crate::{TransportError, TransportResult};
use serde::Serialize;
use serde_json::value::{to_raw_value, RawValue};
use std::future::Future;
use url::Url;

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
/// the host is `localhost`, `127.0.0.1`, or the IPv6 loopback `::1`.
pub fn guess_local_url(s: impl AsRef<str>) -> bool {
    fn _guess_local_url(url: &str) -> bool {
        url.parse::<Url>().is_ok_and(|url| {
            url.host_str()
                .is_none_or(|host| host == "localhost" || host == "127.0.0.1" || host == "::1")
        })
    }
    _guess_local_url(s.as_ref())
}

/// Returns an RPC URL safe for display by retaining only its scheme, host, and port.
///
/// Username, password, path, query, and fragment are stripped, so provider URLs that carry an API
/// key in any of those places can be included in error messages and logs. Input that does not parse
/// as a URL is replaced with a `<redacted>` placeholder rather than echoed back, since a malformed
/// URL may still contain a secret.
///
/// # Examples
///
/// ```
/// use alloy_transport::utils::redact_url;
///
/// assert_eq!(
///     redact_url("https://user:pass@example.com:8545/key?token=secret"),
///     "https://example.com:8545/"
/// );
/// assert_eq!(redact_url("not a url"), "<redacted>");
/// ```
pub fn redact_url(s: impl AsRef<str>) -> String {
    fn _redact_url(url: &str) -> String {
        let Ok(mut url) = url.parse::<Url>() else {
            return "<redacted>".to_owned();
        };
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_path("");
        url.set_query(None);
        url.set_fragment(None);
        url.into()
    }
    _redact_url(s.as_ref())
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
    use super::*;

    #[test]
    fn redacts_url_credentials_and_resource() {
        assert_eq!(
            redact_url("https://user:password@example.com:8545/private-key?token=secret#fragment"),
            "https://example.com:8545/"
        );
        assert_eq!(
            redact_url("https://eth-mainnet.example.com/v2/api-key"),
            "https://eth-mainnet.example.com/"
        );
        assert_eq!(redact_url("not a URL with secret"), "<redacted>");
    }
}
