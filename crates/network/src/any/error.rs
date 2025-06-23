//! Error types for converting between `Any` types.

use core::{error::Error, fmt};

/// A ConversionError that can capture any error type that implements the `Error` trait.
pub struct AnyConversionError {
    inner: Box<dyn Error + Send + Sync + 'static>,
}

impl AnyConversionError {
    /// Creates a new `AnyConversionError` wrapping the given error value.
    pub fn new<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self { inner: Box::new(error) }
    }

    /// Returns a reference to the underlying error value.
    pub fn as_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.inner.as_ref()
    }
}

impl fmt::Debug for AnyConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Display for AnyConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl Error for AnyConversionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}
