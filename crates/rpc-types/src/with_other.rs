use crate::{other::OtherFields, TransactionRequest};
use alloy_consensus::{TxEnvelope, TypedTransaction};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Wrapper allowing to catch all fields missing on the inner struct while
/// deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithOtherFields<T> {
    /// The inner struct.
    #[serde(flatten)]
    pub inner: T,
    /// All fields not present in the inner struct.
    #[serde(flatten)]
    pub other: OtherFields,
}

impl<T> WithOtherFields<T> {
    /// Create a new `Extra`.
    pub fn new(inner: T) -> Self {
        Self { inner, other: OtherFields::default() }
    }
}

impl From<TypedTransaction> for WithOtherFields<TransactionRequest> {
    fn from(tx: TypedTransaction) -> Self {
        Self::new(tx.into())
    }
}

impl From<TxEnvelope> for WithOtherFields<TransactionRequest> {
    fn from(envelope: TxEnvelope) -> Self {
        Self::new(envelope.into())
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
