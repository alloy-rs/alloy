//! Deserialization of call-many requests with a caller-supplied call limit.

use super::Bundle;
use crate::BlockOverrides;
use alloc::vec::Vec;
use core::{fmt, marker::PhantomData};
use serde::{
    de::{DeserializeSeed, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

/// The call limit was exceeded while deserializing a call-many request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallManyLimitExceeded;

impl fmt::Display for CallManyLimitExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("too many calls in call-many request")
    }
}

impl core::error::Error for CallManyLimitExceeded {}

/// A list of calls, as taken by `eth_callMany`, `debug_traceCallMany` or `trace_callMany`, that
/// can be deserialized with a limit on its number of calls.
pub trait CallManyList: Sized {
    /// Deserializes the list, decoding at most `max_calls` calls.
    ///
    /// Calls past the limit are consumed without being deserialized, and the result is
    /// [`CallManyLimitExceeded`]. Calls are counted like [`Self::call_count`].
    fn deserialize_limited<'de, D: Deserializer<'de>>(
        deserializer: D,
        max_calls: usize,
    ) -> Result<Result<Self, CallManyLimitExceeded>, D::Error>;

    /// Returns the number of calls in the list, as counted against the limit.
    fn call_count(&self) -> usize;
}

/// The bundles of `eth_callMany` and `debug_traceCallMany`.
///
/// An empty bundle counts as one call, so the limit also bounds the number of bundles.
impl<TxReq: for<'de> Deserialize<'de>> CallManyList for Vec<Bundle<TxReq>> {
    fn deserialize_limited<'de, D: Deserializer<'de>>(
        deserializer: D,
        max_calls: usize,
    ) -> Result<Result<Self, CallManyLimitExceeded>, D::Error> {
        deserializer.deserialize_seq(BundlesVisitor { max_calls, marker: PhantomData })
    }

    fn call_count(&self) -> usize {
        self.iter()
            .fold(0usize, |calls, bundle| calls.saturating_add(bundle.transactions.len().max(1)))
    }
}

/// The calls of `trace_callMany`, each a transaction request with its trace types.
impl<TxReq: for<'de> Deserialize<'de>, T: for<'de> Deserialize<'de>> CallManyList
    for Vec<(TxReq, T)>
{
    fn deserialize_limited<'de, D: Deserializer<'de>>(
        deserializer: D,
        max_calls: usize,
    ) -> Result<Result<Self, CallManyLimitExceeded>, D::Error> {
        deserializer.deserialize_seq(CallsVisitor { max_calls, marker: PhantomData })
    }

    fn call_count(&self) -> usize {
        self.len()
    }
}

/// A deserialization seed that bounds the number of calls in a call-many request.
///
/// The limit is supplied by the caller instead of being imposed on the ordinary deserializer.
/// Excess calls are consumed without deserializing them. The limit does not bound the size of a
/// single transaction request or block override.
///
/// # Example
///
/// ```
/// use alloy_rpc_types_eth::{Bundle, CallManyLimitExceeded, CallManySeed, TransactionRequest};
/// use serde::de::DeserializeSeed;
///
/// let input = r#"[{"transactions":[{},{}]},{"transactions":[]}]"#;
/// let mut deserializer = serde_json::Deserializer::from_str(input);
/// let bundles = CallManySeed::<Vec<Bundle<TransactionRequest>>>::new(3)
///     .deserialize(&mut deserializer)??;
/// deserializer.end()?;
/// assert_eq!(bundles.len(), 2);
///
/// let mut deserializer = serde_json::Deserializer::from_str(input);
/// let err = CallManySeed::<Vec<Bundle<TransactionRequest>>>::new(2)
///     .deserialize(&mut deserializer)?
///     .unwrap_err();
/// assert_eq!(err, CallManyLimitExceeded);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
pub struct CallManySeed<T> {
    max_calls: usize,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for CallManySeed<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for CallManySeed<T> {}

impl<T> fmt::Debug for CallManySeed<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CallManySeed").field("max_calls", &self.max_calls).finish()
    }
}

impl<T> CallManySeed<T> {
    /// Creates a seed that decodes at most `max_calls` calls.
    pub const fn new(max_calls: usize) -> Self {
        Self { max_calls, marker: PhantomData }
    }
}

impl<'de, T: CallManyList> DeserializeSeed<'de> for CallManySeed<T> {
    type Value = Result<T, CallManyLimitExceeded>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        T::deserialize_limited(deserializer, self.max_calls)
    }
}

/// Calls left to decode, and whether a call past the limit was seen.
struct Budget {
    remaining: usize,
    exceeded: bool,
}

impl Budget {
    /// Charges one call, returning `false` if the limit is exhausted.
    const fn charge(&mut self) -> bool {
        if self.exceeded || self.remaining == 0 {
            self.exceeded = true;
            return false;
        }
        self.remaining -= 1;
        true
    }

    fn finish<T>(self, value: T) -> Result<T, CallManyLimitExceeded> {
        if self.exceeded {
            Err(CallManyLimitExceeded)
        } else {
            Ok(value)
        }
    }
}

struct CallsVisitor<T> {
    max_calls: usize,
    marker: PhantomData<fn() -> T>,
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for CallsVisitor<T> {
    type Value = Result<Vec<T>, CallManyLimitExceeded>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array of calls")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut budget = Budget { remaining: self.max_calls, exceeded: false };
        let mut calls = Vec::new();
        while let Some(call) =
            seq.next_element_seed(CallSeed { budget: &mut budget, marker: self.marker })?
        {
            if let Some(call) = call {
                calls.push(call);
            }
        }
        Ok(budget.finish(calls))
    }
}

/// Decodes one call if the budget allows it, and skips it otherwise.
struct CallSeed<'a, T> {
    budget: &'a mut Budget,
    marker: PhantomData<fn() -> T>,
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for CallSeed<'_, T> {
    type Value = Option<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        if !self.budget.charge() {
            IgnoredAny::deserialize(deserializer)?;
            return Ok(None);
        }
        T::deserialize(deserializer).map(Some)
    }
}

struct BundlesVisitor<TxReq> {
    max_calls: usize,
    marker: PhantomData<fn() -> TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for BundlesVisitor<TxReq> {
    type Value = Result<Vec<Bundle<TxReq>>, CallManyLimitExceeded>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array of bundles")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut budget = Budget { remaining: self.max_calls, exceeded: false };
        let mut bundles = Vec::new();
        while let Some(bundle) =
            seq.next_element_seed(BundleSeed { budget: &mut budget, marker: self.marker })?
        {
            if let Some(bundle) = bundle {
                bundles.push(bundle);
            }
        }
        Ok(budget.finish(bundles))
    }
}

/// Decodes one bundle within the budget, and skips it once the budget is exhausted.
struct BundleSeed<'a, TxReq> {
    budget: &'a mut Budget,
    marker: PhantomData<fn() -> TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for BundleSeed<'_, TxReq> {
    type Value = Option<Bundle<TxReq>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        if self.budget.exceeded {
            IgnoredAny::deserialize(deserializer)?;
            return Ok(None);
        }
        let bundle = deserializer.deserialize_struct(
            "Bundle",
            &["transactions", "blockOverride"],
            BundleVisitor { budget: &mut *self.budget, marker: self.marker },
        )?;
        // An empty bundle counts as one call.
        if bundle.transactions.is_empty() && !self.budget.charge() {
            return Ok(None);
        }
        Ok((!self.budget.exceeded).then_some(bundle))
    }
}

struct BundleVisitor<'a, TxReq> {
    budget: &'a mut Budget,
    marker: PhantomData<fn() -> TxReq>,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum BundleField {
    Transactions,
    BlockOverride,
    #[serde(other)]
    Other,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for BundleVisitor<'_, TxReq> {
    type Value = Bundle<TxReq>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bundle")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut transactions = None;
        let mut block_override = None;
        while let Some(key) = map.next_key::<BundleField>()? {
            match key {
                BundleField::Transactions => {
                    if transactions.is_some() {
                        return Err(A::Error::duplicate_field("transactions"));
                    }
                    transactions = Some(map.next_value_seed(TransactionsSeed {
                        budget: &mut *self.budget,
                        marker: self.marker,
                    })?);
                }
                BundleField::BlockOverride => {
                    if block_override.is_some() {
                        return Err(A::Error::duplicate_field("blockOverride"));
                    }
                    block_override = Some(map.next_value::<Option<BlockOverrides>>()?);
                }
                BundleField::Other => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        let transactions = transactions.ok_or_else(|| A::Error::missing_field("transactions"))?;
        Ok(Bundle { transactions, block_override: block_override.flatten() })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let transactions = seq
            .next_element_seed(TransactionsSeed { budget: &mut *self.budget, marker: self.marker })?
            .ok_or_else(|| A::Error::invalid_length(0, &self))?;
        let block_override = seq.next_element::<Option<BlockOverrides>>()?.flatten();
        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::invalid_length(3, &self));
        }
        Ok(Bundle { transactions, block_override })
    }
}

/// Decodes the transactions of a bundle within the budget.
struct TransactionsSeed<'a, TxReq> {
    budget: &'a mut Budget,
    marker: PhantomData<fn() -> TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> DeserializeSeed<'de> for TransactionsSeed<'_, TxReq> {
    type Value = Vec<TxReq>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer
            .deserialize_seq(TransactionsVisitor { budget: self.budget, marker: self.marker })
    }
}

struct TransactionsVisitor<'a, TxReq> {
    budget: &'a mut Budget,
    marker: PhantomData<fn() -> TxReq>,
}

impl<'de, TxReq: Deserialize<'de>> Visitor<'de> for TransactionsVisitor<'_, TxReq> {
    type Value = Vec<TxReq>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array of transactions")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut transactions = Vec::new();
        while let Some(tx) =
            seq.next_element_seed(CallSeed { budget: &mut *self.budget, marker: self.marker })?
        {
            if let Some(tx) = tx {
                transactions.push(tx);
            }
        }
        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionRequest;
    use alloc::{format, string::String, vec};
    use core::cell::Cell;

    std::thread_local! {
        static DECODED: Cell<usize> = const { Cell::new(0) };
    }

    /// A transaction request that counts how often it is decoded.
    #[derive(Debug)]
    struct CountedRequest;

    impl<'de> Deserialize<'de> for CountedRequest {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            IgnoredAny::deserialize(deserializer)?;
            DECODED.with(|decoded| decoded.set(decoded.get() + 1));
            Ok(Self)
        }
    }

    fn bundles(counts: &[usize]) -> String {
        let bundles = counts
            .iter()
            .map(|&count| format!(r#"{{"transactions":[{}]}}"#, vec!["{}"; count].join(",")))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{bundles}]")
    }

    fn parse<T: CallManyList>(
        input: &str,
        max_calls: usize,
    ) -> Result<Result<T, CallManyLimitExceeded>, serde_json::Error> {
        let mut deserializer = serde_json::Deserializer::from_str(input);
        let value = CallManySeed::<T>::new(max_calls).deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(value)
    }

    #[test]
    fn decodes_bundles_within_limit() {
        let input = r#"[{"transactions":[{"gas":"0x1"}],"blockOverride":{"number":"0x2"}},{"transactions":[]}]"#;
        let expected = serde_json::from_str::<Vec<Bundle<TransactionRequest>>>(input).unwrap();
        let parsed = parse::<Vec<Bundle<TransactionRequest>>>(input, 2).unwrap().unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.call_count(), 2);
    }

    #[test]
    fn bounds_calls_across_bundles() {
        let input = bundles(&[3, 2]);
        assert_eq!(parse::<Vec<Bundle<TransactionRequest>>>(&input, 5).unwrap().unwrap().len(), 2);
        assert_eq!(
            parse::<Vec<Bundle<TransactionRequest>>>(&input, 4).unwrap(),
            Err(CallManyLimitExceeded)
        );
    }

    #[test]
    fn counts_empty_bundles() {
        let input = bundles(&[0, 0, 0]);
        assert_eq!(parse::<Vec<Bundle<TransactionRequest>>>(&input, 3).unwrap().unwrap().len(), 3);
        assert_eq!(
            parse::<Vec<Bundle<TransactionRequest>>>(&input, 2).unwrap(),
            Err(CallManyLimitExceeded)
        );
    }

    #[test]
    fn decodes_array_encoded_bundles() {
        let input = r#"[[[{}, {}], null], [[{}]]]"#;
        let expected = serde_json::from_str::<Vec<Bundle<TransactionRequest>>>(input).unwrap();
        assert_eq!(parse::<Vec<Bundle<TransactionRequest>>>(input, 3).unwrap().unwrap(), expected);
        assert_eq!(
            parse::<Vec<Bundle<TransactionRequest>>>(input, 2).unwrap(),
            Err(CallManyLimitExceeded)
        );
    }

    #[test]
    fn rejects_duplicate_transactions() {
        let input = r#"[{"transactions":[{}],"transactions":[]}]"#;
        let err = parse::<Vec<Bundle<TransactionRequest>>>(input, 8).unwrap_err();
        assert_eq!(err.to_string(), "duplicate field `transactions` at line 1 column 36");
    }

    #[test]
    fn rejects_missing_transactions() {
        let err = parse::<Vec<Bundle<TransactionRequest>>>("[{}]", 8).unwrap_err();
        assert_eq!(err.to_string(), "missing field `transactions` at line 1 column 3");
    }

    #[test]
    fn bounds_trace_calls() {
        let input = r#"[[{},["trace"]],[{},["vmTrace"]]]"#;
        let parsed = parse::<Vec<(TransactionRequest, Vec<String>)>>(input, 2).unwrap().unwrap();
        assert_eq!(parsed.call_count(), 2);
        assert_eq!(
            parse::<Vec<(TransactionRequest, Vec<String>)>>(input, 1).unwrap(),
            Err(CallManyLimitExceeded)
        );
    }

    #[test]
    fn skips_calls_past_the_limit_without_decoding() {
        DECODED.with(|decoded| decoded.set(0));
        let input = bundles(&[2, 100_000, 3]);
        assert_eq!(
            parse::<Vec<Bundle<CountedRequest>>>(&input, 4).unwrap().unwrap_err(),
            CallManyLimitExceeded
        );
        assert_eq!(DECODED.with(Cell::get), 4);

        DECODED.with(|decoded| decoded.set(0));
        let input = format!("[{}]", vec!["[{},[]]"; 100_000].join(","));
        assert_eq!(
            parse::<Vec<(CountedRequest, Vec<String>)>>(&input, 4).unwrap().unwrap_err(),
            CallManyLimitExceeded
        );
        assert_eq!(DECODED.with(Cell::get), 4);
    }
}
