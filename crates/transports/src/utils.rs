use serde::Serialize;
use serde_json::{self, value::RawValue};

use std::future::Future;

use crate::error::TransportError;

/// Convert to a `Box<RawValue>` from a `Serialize` type, mapping the error
/// to a `TransportError`.
pub(crate) fn to_json_raw_value<S>(s: &S) -> Result<Box<RawValue>, TransportError>
where
    S: Serialize,
{
    RawValue::from_string(serde_json::to_string(s).map_err(TransportError::ser_err)?)
        .map_err(TransportError::ser_err)
}

pub trait Spawn {
    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_task(fut: impl Future<Output = ()> + Send + 'static) {
        tokio::spawn(fut);
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn_task(fut: impl Future<Output = ()> + 'static) {
        wasm_bindgen_futures::spawn_local(fut);
    }
}
