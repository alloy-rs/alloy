use alloy_transport::{TransportError, TransportErrorKind};
use core::fmt;

/// Error returned when reading a response body.
#[derive(Debug)]
pub(crate) enum BodyError<E> {
    /// Reading the body failed.
    Read(E),
    /// The body exceeded [`HttpTransportSettings::max_response_size`].
    ///
    /// [`HttpTransportSettings::max_response_size`]: crate::HttpTransportSettings::max_response_size
    TooLarge(usize),
}

impl<E: std::error::Error + Send + Sync + 'static> BodyError<E> {
    /// Converts this into a [`TransportError`], keeping the read error as is.
    pub(crate) fn into_transport_error(self) -> TransportError {
        match self {
            Self::Read(err) => TransportErrorKind::custom(err),
            err @ Self::TooLarge(_) => TransportErrorKind::custom_str(&err.to_string()),
        }
    }
}

impl<E: fmt::Display> fmt::Display for BodyError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => err.fmt(f),
            Self::TooLarge(max) => write!(f, "response body exceeds {max} bytes"),
        }
    }
}
