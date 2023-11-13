use crate::error::TransportError;
use serde::Serialize;
use serde_json::{self, value::RawValue};
use std::future::Future;
use url::Url;

/// Guess whether the URL is local, based on the hostname.
///
/// The ouput of this function is best-efforts, and should be checked if
/// possible. It simply returns `true` if the connection has no hostname,
/// or the hostname is `localhost` or `127.0.0.1`.
pub fn guess_local_url(s: impl AsRef<str>) -> bool {
    fn _guess_local_url(url: &str) -> bool {
        if let Ok(url) = url.parse::<Url>() {
            url.host_str()
                .map_or(true, |host| host == "localhost" || host == "127.0.0.1")
        } else {
            false
        }
    }
    _guess_local_url(s.as_ref())
}

/// Convert to a `Box<RawValue>` from a `Serialize` type, mapping the error
/// to a `TransportError`.
pub fn to_json_raw_value<S>(s: &S) -> Result<Box<RawValue>, TransportError>
where
    S: Serialize,
{
    RawValue::from_string(serde_json::to_string(s).map_err(TransportError::ser_err)?)
        .map_err(TransportError::ser_err)
}

#[doc(hidden)]
pub trait Spawnable {
    /// Spawn the future as a task.
    ///
    /// In WASM this will be a `wasm-bindgen-futures::spawn_local` call, while
    /// in native it will be a `tokio::spawn` call.
    fn spawn_task(self);
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> Spawnable for T
where
    T: Future<Output = ()> + Send + 'static,
{
    fn spawn_task(self) {
        tokio::spawn(self);
    }
}

#[cfg(target_arch = "wasm32")]
impl<T> Spawnable for T
where
    T: Future<Output = ()> + 'static,
{
    fn spawn_task(self) {
        wasm_bindgen_futures::spawn_local(self);
    }
}
