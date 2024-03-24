use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper allowing to catch all fields missing on the inner struct while
/// deserialize.
pub struct Extra<T> {
    #[serde(flatten)]
    inner: T,
    /// All fields not present in the inner struct.
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl<T> Extra<T> {
    /// Create a new `Extra`.
    pub fn new(inner: T) -> Self {
        Self { inner, other: HashMap::default() }
    }

    /// Unwrap the inner struct.
    pub fn unwrap(self) -> T {
        self.inner
    }
}

impl<T> Deref for Extra<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Extra<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
