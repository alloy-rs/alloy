use alloy_transport::{TransportError, TransportErrorKind};
use core::fmt;
#[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
use std::time::Duration;

/// Error returned when reading a response body.
#[derive(Debug)]
pub(crate) enum BodyError<E> {
    /// Reading the body failed.
    Read(E),
    /// The body exceeded [`HttpTransportSettings::max_response_size`].
    ///
    /// [`HttpTransportSettings::max_response_size`]: crate::HttpTransportSettings::max_response_size
    TooLarge(usize),
    /// No body data arrived within the read timeout.
    #[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
    TimedOut(Duration),
    /// The request deadline passed while reading the body.
    #[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
    RequestTimedOut(Duration),
}

impl<E: std::error::Error + Send + Sync + 'static> BodyError<E> {
    /// Converts this into a [`TransportError`], keeping the read error as is.
    pub(crate) fn into_transport_error(self) -> TransportError {
        match self {
            Self::Read(err) => TransportErrorKind::custom(err),
            err => TransportErrorKind::custom_str(&err.to_string()),
        }
    }
}

impl<E: fmt::Display> fmt::Display for BodyError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => err.fmt(f),
            Self::TooLarge(max) => write!(f, "response body exceeds {max} bytes"),
            #[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
            Self::TimedOut(duration) => {
                write!(f, "response body read timed out after {duration:?}")
            }
            #[cfg(all(not(target_family = "wasm"), feature = "hyper"))]
            Self::RequestTimedOut(duration) => {
                write!(f, "HTTP request timed out after {duration:?}")
            }
        }
    }
}
