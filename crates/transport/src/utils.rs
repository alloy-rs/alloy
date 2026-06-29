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
