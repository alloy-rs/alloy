//! Helper errors.

/// Helper type that is [`core::error::Error`] and wraps a value and an error message.
///
/// This can be used to return an object as part of an `Err` and is used for fallible conversions.
#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
pub struct ValueError<T> {
    msg: String,
    value: T
}

impl<T> ValueError<T> {

    /// Creates a new error with the given value and error message.
    pub fn new(value: T, msg: impl core::fmt::Display) -> Self {
        Self {
            msg: msg.to_string(),
            value,
        }
    }

    /// Consumes the type and returns the underlying value.
    pub fn into_value(self) -> T {
        self.value
    }

    /// Returns a reference to the value.
    pub const fn value(&self) -> &T { &self.value }
}