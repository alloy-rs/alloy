//! Helper errors.

use alloc::{borrow::Cow, boxed::Box, string::ToString};
use core::fmt::Display;

/// Helper type that is [`core::error::Error`] and wraps a value and an error message.
///
/// This can be used to return an object as part of an `Err` and is used for fallible conversions.
#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
pub struct ValueError<T> {
    msg: Cow<'static, str>,
    value: Box<T>,
}

impl<T> ValueError<T> {
    /// Creates a new error with the given value and error message.
    pub fn new(value: T, msg: impl Display) -> Self {
        Self { msg: Cow::Owned(msg.to_string()), value: Box::new(value) }
    }

    /// Creates a new error with a static error message.
    pub fn new_static(value: T, msg: &'static str) -> Self {
        Self { msg: Cow::Borrowed(msg), value: Box::new(value) }
    }

    /// Converts the value to the given alternative that is `From<T>`.
    pub fn convert<U>(self) -> ValueError<U>
    where
        U: From<T>,
    {
        self.map(U::from)
    }

    /// Maps the error's value with the given closure.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ValueError<U> {
        ValueError { msg: self.msg, value: Box::new(f(*self.value)) }
    }

    /// Consumes the type and returns the underlying value.
    pub fn into_value(self) -> T {
        *self.value
    }

    /// Returns a reference to the value.
    pub const fn value(&self) -> &T {
        &self.value
    }
}

/// The error for conversions or processing of transactions of type using components that lack the
/// knowledge or capability to do so.
#[derive(Debug, thiserror::Error)]
#[error("Unsupported transaction type: {0}")]
pub struct UnsupportedTransactionType<TxType: Display>(TxType);
