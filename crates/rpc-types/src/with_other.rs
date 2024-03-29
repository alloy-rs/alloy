use crate::other::OtherFields;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Wrapper allowing to catch all fields missing on the inner struct while
/// deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithOtherFields<T> {
    #[serde(flatten)]
    inner: T,
    /// All fields not present in the inner struct.
    #[serde(flatten)]
    pub other: OtherFields,
}

impl<T> WithOtherFields<T> {
    /// Create a new `Extra`.
    pub fn new(inner: T) -> Self {
        Self { inner, other: OtherFields::default() }
    }

    /// Unwrap the inner struct.
    pub fn unwrap(self) -> T {
        self.inner
    }
}

impl<T> Deref for WithOtherFields<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for WithOtherFields<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Default> Default for WithOtherFields<T> {
    fn default() -> Self {
        WithOtherFields::new(T::default())
    }
}
