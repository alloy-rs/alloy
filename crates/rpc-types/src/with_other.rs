use crate::{other::OtherFields, TransactionRequest};
use alloy_consensus::{TxEnvelope, TypedTransaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::{Deref, DerefMut};

/// Wrapper allowing to catch all fields missing on the inner struct while
/// deserialize.
#[derive(Debug, Clone, Serialize)]
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

impl<'de, T> Deserialize<'de> for WithOtherFields<T>
where
    T: Deserialize<'de> + Serialize,
{
    fn deserialize<D>(deserializer: D) -> Result<WithOtherFields<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        struct WithOtherFieldsHelper<T> {
            #[serde(flatten)]
            inner: T,
            #[serde(flatten)]
            other: OtherFields,
        }

        let mut helper = WithOtherFieldsHelper::deserialize(deserializer)?;
        let inner_serialzed = serde_json::to_value(&helper.inner).map_err(D::Error::custom)?;
        match &inner_serialzed {
            Value::Object(map) => {
                for key in map.keys() {
                    helper.other.remove(key);
                }
            }
            _ => {}
        }

        Ok(WithOtherFields { inner: helper.inner, other: helper.other })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use serde_json::json;

    #[derive(Serialize, Deserialize)]
    struct Inner {
        a: u64,
    }

    #[derive(Serialize, Deserialize)]
    struct InnerWrapper {
        #[serde(flatten)]
        inner: Inner,
    }

    #[test]
    fn test_correct_other() {
        let with_other: WithOtherFields<InnerWrapper> =
            serde_json::from_str("{\"a\": 1, \"b\": 2}").unwrap();
        assert_eq!(with_other.inner.inner.a, 1);
        assert_eq!(
            with_other.other,
            OtherFields::new(BTreeMap::from_iter(vec![("b".to_string(), json!(2))]))
        );
    }
}
