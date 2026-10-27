//! Types used by tracing backends.

use alloy_primitives::TxHash;
use core::{fmt, marker::PhantomData};
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

/// The result of a single transaction trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum TraceResult<Ok, Err> {
    /// Untagged success variant
    Success {
        /// Trace results produced by the tracer
        result: Ok,
        /// transaction hash
        #[serde(skip_serializing_if = "Option::is_none", rename = "txHash")]
        #[doc(alias = "transaction_hash")]
        tx_hash: Option<TxHash>,
    },
    /// Untagged error variant
    Error {
        /// Trace failure produced by the tracer
        error: Err,
        /// transaction hash
        #[serde(skip_serializing_if = "Option::is_none", rename = "txHash")]
        #[doc(alias = "transaction_hash")]
        tx_hash: Option<TxHash>,
    },
}

impl<Ok, Err> TraceResult<Ok, Err> {
    /// Returns the hash of the transaction that was traced.
    #[doc(alias = "transaction_hash")]
    pub const fn tx_hash(&self) -> Option<TxHash> {
        *match self {
            Self::Success { tx_hash, .. } | Self::Error { tx_hash, .. } => tx_hash,
        }
    }

    /// Returns a reference to the result if it is a success variant.
    pub const fn success(&self) -> Option<&Ok> {
        match self {
            Self::Success { result, .. } => Some(result),
            Self::Error { .. } => None,
        }
    }

    /// Returns a reference to the error if it is an error variant.
    pub const fn error(&self) -> Option<&Err> {
        match self {
            Self::Error { error, .. } => Some(error),
            Self::Success { .. } => None,
        }
    }

    /// Checks if the result is a success.
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Checks if the result is an error.
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Creates a new success trace result.
    pub const fn new_success(result: Ok, tx_hash: Option<TxHash>) -> Self {
        Self::Success { result, tx_hash }
    }

    /// Creates a new error trace result.
    pub const fn new_error(error: Err, tx_hash: Option<TxHash>) -> Self {
        Self::Error { error, tx_hash }
    }
}

impl<'de, Ok, Err> Deserialize<'de> for TraceResult<Ok, Err>
where
    Ok: Deserialize<'de>,
    Err: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field {
            #[serde(rename = "result")]
            Result,
            #[serde(rename = "error")]
            Error,
            #[serde(rename = "txHash")]
            TxHash,
            #[serde(other)]
            Unknown,
        }

        struct TraceResultVisitor<Ok, Err>(PhantomData<fn() -> (Ok, Err)>);

        impl<'de, Ok, Err> Visitor<'de> for TraceResultVisitor<Ok, Err>
        where
            Ok: Deserialize<'de>,
            Err: Deserialize<'de>,
        {
            type Value = TraceResult<Ok, Err>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a trace result with a `result` or `error` field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut result = None;
                let mut error = None;
                let mut tx_hash = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Result => {
                            if result.is_some() {
                                return Err(de::Error::duplicate_field("result"));
                            }
                            result = Some(map.next_value()?);
                        }
                        Field::Error => {
                            if error.is_some() {
                                return Err(de::Error::duplicate_field("error"));
                            }
                            error = Some(map.next_value::<Option<Err>>()?);
                        }
                        Field::TxHash => {
                            if tx_hash.is_some() {
                                return Err(de::Error::duplicate_field("txHash"));
                            }
                            tx_hash = Some(map.next_value()?);
                        }
                        Field::Unknown => {
                            map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }
                let tx_hash = tx_hash.flatten();
                match (result, error.flatten()) {
                    (Some(result), _) => Ok(TraceResult::Success { result, tx_hash }),
                    (None, Some(error)) => Ok(TraceResult::Error { error, tx_hash }),
                    (None, None) => Err(de::Error::custom("missing field `result` or `error`")),
                }
            }
        }

        deserializer.deserialize_map(TraceResultVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    type Result = TraceResult<serde_json::Value, String>;

    #[test]
    fn deserialize_trace_result() {
        let hash = TxHash::repeat_byte(1);
        let value = json!({"txHash": hash, "result": {"a": 1}, "extra": [1, 2]});
        let expected = Result::new_success(json!({"a": 1}), Some(hash));
        assert_eq!(serde_json::from_value::<Result>(value.clone()).unwrap(), expected);
        assert_eq!(serde_json::from_str::<Result>(&value.to_string()).unwrap(), expected);

        let null_error = r#"{"result": 1, "error": null}"#;
        assert_eq!(
            serde_json::from_str::<Result>(null_error).unwrap(),
            Result::new_success(json!(1), None)
        );

        let escaped = r#"{"\u0065rror": "boom"}"#;
        assert_eq!(
            serde_json::from_str::<Result>(escaped).unwrap(),
            Result::new_error("boom".to_string(), None)
        );
    }

    #[test]
    fn deserialize_trace_result_invalid() {
        assert!(serde_json::from_str::<Result>(r#"{"txHash": null}"#).is_err());
        assert!(serde_json::from_str::<Result>(r#"{"result": 1, "result": 2}"#).is_err());
    }
}
