//! Call-many requests that are decoded after the caller's limit is known.

use super::{CallManyLimitExceeded, CallManyList, CallManySeed};
use alloc::boxed::Box;
use serde::{de::DeserializeSeed, Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;

/// The calls of an `eth_callMany`, `debug_traceCallMany` or `trace_callMany` request, whose
/// decoding is deferred until the caller's call limit is known.
///
/// Deserializing this type only captures the raw JSON value, without allocating any calls.
/// [`Self::parse`] decodes it later with [`CallManySeed`] and the limit supplied by the caller.
///
/// Use it as the calls parameter of an RPC server method. Server frameworks such as `jsonrpsee`
/// deserialize all parameters before the handler runs, so a plain `Vec` parameter would be
/// decoded without any limit, allocating a full transaction request for every `{}` in the input.
///
/// Serializing [`Self::Raw`] directly to JSON text preserves the original JSON.
///
/// Deserialization requires a `serde_json` deserializer and does not work inside
/// `#[serde(untagged)]` or `#[serde(flatten)]` containers, which buffer their input.
///
/// # Example
///
/// ```
/// use alloy_rpc_types_eth::{Bundle, RawCallMany, RawCallManyError, TransactionRequest};
///
/// let input = r#"[{"transactions":[{},{}]},{"transactions":[{}]}]"#;
/// let calls: RawCallMany<Vec<Bundle<TransactionRequest>>> = serde_json::from_str(input)?;
///
/// let err = calls.clone().parse(2).unwrap_err();
/// assert!(matches!(err, RawCallManyError::LimitExceeded(_)));
///
/// let bundles = calls.parse(3)?;
/// assert_eq!(bundles.len(), 2);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum RawCallMany<T> {
    /// Raw JSON input, kept until [`RawCallMany::parse`] applies the caller's limit.
    Raw(Box<RawValue>),
    /// Decoded calls, for example ones built by a client or an internal caller.
    Parsed(T),
}

impl<T> From<T> for RawCallMany<T> {
    fn from(calls: T) -> Self {
        Self::Parsed(calls)
    }
}

impl<'de, T> Deserialize<'de> for RawCallMany<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Box::<RawValue>::deserialize(deserializer).map(Self::Raw)
    }
}

impl<T: CallManyList> RawCallMany<T> {
    /// Decodes the calls, allowing at most `max_calls`, counted like
    /// [`CallManyList::call_count`].
    ///
    /// Raw input is decoded with [`CallManySeed`], which skips excess calls without
    /// deserializing them, and must be a single JSON value with no trailing input. A
    /// [`Parsed`](Self::Parsed) list is checked against the same limit, so returned calls always
    /// satisfy it.
    ///
    /// # Errors
    ///
    /// - [`RawCallManyError::Json`] if the raw input is not a valid list of calls.
    /// - [`RawCallManyError::LimitExceeded`] if there are more than `max_calls` calls.
    pub fn parse(self, max_calls: usize) -> Result<T, RawCallManyError> {
        match self {
            Self::Raw(raw) => {
                let mut deserializer = serde_json::Deserializer::from_str(raw.get());
                let calls = CallManySeed::new(max_calls).deserialize(&mut deserializer)?;
                deserializer.end()?;
                Ok(calls?)
            }
            Self::Parsed(calls) => {
                if calls.call_count() > max_calls {
                    return Err(CallManyLimitExceeded.into());
                }
                Ok(calls)
            }
        }
    }
}

/// Error returned by [`RawCallMany::parse`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RawCallManyError {
    /// The raw input is not a valid list of calls, for example malformed JSON, a field of the
    /// wrong type, or trailing input.
    ///
    /// JSON-RPC servers usually report this as invalid params (`-32602`). Its message can quote
    /// request input.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The request exceeds the call limit passed to [`RawCallMany::parse`].
    #[error(transparent)]
    LimitExceeded(#[from] CallManyLimitExceeded),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bundle, TransactionRequest};
    use alloc::{string::String, vec, vec::Vec};

    type Bundles = Vec<Bundle<TransactionRequest>>;

    #[test]
    fn parses_raw_input_within_limit() {
        let input = r#"[{"transactions":[{"gas":"0x1"}]},{"transactions":[]}]"#;
        let expected = serde_json::from_str::<Bundles>(input).unwrap();
        let calls = serde_json::from_str::<RawCallMany<Bundles>>(input).unwrap();
        assert_eq!(calls.parse(2).unwrap(), expected);
    }

    #[test]
    fn rejects_raw_input_over_limit() {
        let input = r#"[[{},["trace"]],[{},["trace"]]]"#;
        let calls =
            serde_json::from_str::<RawCallMany<Vec<(TransactionRequest, Vec<String>)>>>(input)
                .unwrap();
        assert!(matches!(calls.parse(1), Err(RawCallManyError::LimitExceeded(_))));
    }

    #[test]
    fn rejects_malformed_input() {
        for input in [r#""x""#, r#"{"transactions":[]}"#, r#"[{"transactions":{}}]"#] {
            let calls = serde_json::from_str::<RawCallMany<Bundles>>(input).unwrap();
            assert!(matches!(calls.parse(8), Err(RawCallManyError::Json(_))), "{input}");
        }
    }

    #[test]
    fn checks_parsed_calls() {
        let bundles = vec![Bundle::<TransactionRequest>::default(); 3];
        assert_eq!(RawCallMany::from(bundles.clone()).parse(3).unwrap(), bundles);
        assert!(matches!(
            RawCallMany::from(bundles).parse(2),
            Err(RawCallManyError::LimitExceeded(_))
        ));
    }

    #[test]
    fn serializes_raw_and_parsed_like_calls() {
        let input = r#"[{"transactions":[{}]}]"#;
        let calls = serde_json::from_str::<RawCallMany<Bundles>>(input).unwrap();
        assert_eq!(serde_json::to_string(&calls).unwrap(), input);
        let bundles = vec![Bundle::<TransactionRequest>::default()];
        assert_eq!(
            serde_json::to_value(RawCallMany::from(bundles.clone())).unwrap(),
            serde_json::to_value(bundles).unwrap()
        );
    }
}
