//! Ethereum types for pub-sub

use crate::{Filter, Header, Log, Transaction, TransactionReceipt};
use alloc::{boxed::Box, format, vec::Vec};
use alloy_primitives::B256;
use alloy_serde::WithOtherFields;

/// Subscription result.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum SubscriptionResult<T = Transaction, R = TransactionReceipt> {
    /// New block header.
    Header(Box<WithOtherFields<Header>>),
    /// Log
    Log(Box<Log>),
    /// Transaction hash
    TransactionHash(B256),
    /// Full Transaction
    FullTransaction(Box<T>),
    /// SyncStatus
    SyncState(PubSubSyncStatus),
    /// Transaction Receipts
    TransactionReceipts(Vec<R>),
}

/// Response type for a SyncStatus subscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum PubSubSyncStatus {
    /// If not currently syncing, this should always be `false`.
    Simple(bool),
    /// Syncing metadata.
    Detailed(SyncStatusMetadata),
}

/// Sync status metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SyncStatusMetadata {
    /// Whether the node is currently syncing.
    pub syncing: bool,
    /// The starting block.
    pub starting_block: u64,
    /// The current block.
    pub current_block: u64,
    /// The highest block.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub highest_block: Option<u64>,
}

#[cfg(feature = "serde")]
impl<T, R> serde::Serialize for SubscriptionResult<T, R>
where
    T: serde::Serialize,
    R: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Self::Header(ref header) => header.serialize(serializer),
            Self::Log(ref log) => log.serialize(serializer),
            Self::TransactionHash(ref hash) => hash.serialize(serializer),
            Self::FullTransaction(ref tx) => tx.serialize(serializer),
            Self::SyncState(ref sync) => sync.serialize(serializer),
            Self::TransactionReceipts(ref receipts) => receipts.serialize(serializer),
        }
    }
}

/// Subscription kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum SubscriptionKind {
    /// New block headers subscription.
    ///
    /// Fires a notification each time a new header is appended to the chain, including chain
    /// reorganizations. In case of a chain reorganization the subscription will emit all new
    /// headers for the new chain. Therefore the subscription can emit multiple headers on the same
    /// height.
    NewHeads,
    /// Logs subscription.
    ///
    /// Returns logs that are included in new imported blocks and match the given filter criteria.
    /// In case of a chain reorganization previous sent logs that are on the old chain will be
    /// resent with the removed property set to true. Logs from transactions that ended up in the
    /// new chain are emitted. Therefore, a subscription can emit logs for the same transaction
    /// multiple times.
    Logs,
    /// New Pending Transactions subscription.
    ///
    /// Returns the hash or full tx for all transactions that are added to the pending state and
    /// are signed with a key that is available in the node. When a transaction that was
    /// previously part of the canonical chain isn't part of the new canonical chain after a
    /// reorganization its again emitted.
    NewPendingTransactions,
    /// Node syncing status subscription.
    ///
    /// Indicates when the node starts or stops synchronizing. The result can either be a boolean
    /// indicating that the synchronization has started (true), finished (false) or an object with
    /// various progress indicators.
    Syncing,
    /// New transaction receipts subscription.
    ///
    /// Returns transaction receipts that are included in new imported blocks and match the given
    /// filter criteria. In case of a chain reorganization the subscription will emit transaction
    /// receipts for the new chain if they match the filter criteria. Therefore, the subscription
    /// can emit same transaction receipts multiple times.
    TransactionReceipts,
}

/// The maximum number of transaction hash criteria allowed in a single subscription.
pub const MAX_TX_HASHES: usize = 200;

/// Parameters for transaction receipts subscription.
///
/// # Example
///
/// ```json
/// // Subscribe to specific transaction receipts
/// {
///   "transactionHashes": [
///     "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"
///   ]
/// }
/// ```
///
/// ```json
/// // Subscribe to all transaction receipts (no filter)
/// {
///   "transactionHashes": null
/// }
/// ```json
/// // Subscribe to all transaction receipts (no filter)
/// {
///   "transactionHashes": []
/// }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TransactionReceiptsParams {
    /// Optional list of transaction hashes to filter by.
    /// If not provided or empty, all transaction receipts will be returned.
    /// Limited to MAX_TX_HASHES items.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub transaction_hashes: Option<Vec<B256>>,
}

/// Any additional parameters for a subscription.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Params {
    /// No parameters passed.
    #[default]
    None,
    /// Log parameters.
    Logs(Box<Filter>),
    /// Boolean parameter for new pending transactions.
    Bool(bool),
    /// Transaction receipts parameters.
    TransactionReceipts(TransactionReceiptsParams),
}

impl Params {
    /// Returns true if it's a bool parameter.
    #[inline]
    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// Returns true if it's a log parameter.
    #[inline]
    pub const fn is_logs(&self) -> bool {
        matches!(self, Self::Logs(_))
    }
}

impl From<Filter> for Params {
    fn from(filter: Filter) -> Self {
        Self::Logs(Box::new(filter))
    }
}

impl From<bool> for Params {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<TransactionReceiptsParams> for Params {
    fn from(params: TransactionReceiptsParams) -> Self {
        Self::TransactionReceipts(params)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TransactionReceiptsParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use core::fmt;
        use serde::de::{Error, MapAccess, Visitor};

        struct TransactionReceiptsParamsVisitor;

        impl<'de> Visitor<'de> for TransactionReceiptsParamsVisitor {
            type Value = TransactionReceiptsParams;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a transaction receipts parameters object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut transaction_hashes: Option<Option<Vec<B256>>> = None;

                while let Some(key) = map.next_key::<alloc::string::String>()? {
                    match key.as_str() {
                        "transactionHashes" => {
                            if transaction_hashes.is_some() {
                                return Err(A::Error::duplicate_field("transactionHashes"));
                            }
                            let hashes: Option<Vec<B256>> = map.next_value()?;

                            if let Some(ref hash_vec) = hashes {
                                if hash_vec.len() > MAX_TX_HASHES {
                                    return Err(A::Error::custom(format!(
                                        "exceed max number of transaction hashes allowed per transactionReceipts subscription: {} items (max: {})",
                                        hash_vec.len(),
                                        MAX_TX_HASHES
                                    )));
                                }
                            }

                            transaction_hashes = Some(hashes);
                        }
                        key => {
                            return Err(serde::de::Error::unknown_field(
                                key,
                                &["transactionHashes"],
                            ))
                        }
                    }
                }

                Ok(TransactionReceiptsParams {
                    transaction_hashes: transaction_hashes.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(TransactionReceiptsParamsVisitor)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Params {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::None => (&[] as &[serde_json::Value]).serialize(serializer),
            Self::Logs(logs) => logs.serialize(serializer),
            Self::Bool(full) => full.serialize(serializer),
            Self::TransactionReceipts(params) => params.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Deserialize<'a> for Params {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        use serde::de::Error;

        let v = serde_json::Value::deserialize(deserializer)?;

        if v.is_null() {
            return Ok(Self::None);
        }

        if let Some(val) = v.as_bool() {
            return Ok(val.into());
        }

        // Check if it's a transaction receipts parameter by looking for transactionHashes field
        if let Some(obj) = v.as_object() {
            if obj.contains_key("transactionHashes") {
                return serde_json::from_value::<TransactionReceiptsParams>(v)
                    .map(Into::into)
                    .map_err(|e| {
                        D::Error::custom(format!("Invalid transaction receipts parameters: {e}"))
                    });
            }
        }

        serde_json::from_value::<Filter>(v)
            .map(Into::into)
            .map_err(|e| D::Error::custom(format!("Invalid Pub-Sub parameters: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use similar_asserts::assert_eq;

    #[test]
    #[cfg(feature = "serde")]
    fn params_serde() {
        // Test deserialization of boolean parameter
        let s: Params = serde_json::from_str("true").unwrap();
        assert_eq!(s, Params::Bool(true));

        // Test deserialization of null (None) parameter
        let s: Params = serde_json::from_str("null").unwrap();
        assert_eq!(s, Params::None);

        // Test deserialization of log parameters
        let filter = Filter::default();
        let s: Params = serde_json::from_str(&serde_json::to_string(&filter).unwrap()).unwrap();
        assert_eq!(s, Params::Logs(Box::new(filter)));

        // Test deserialization of transaction receipts parameters
        let json = r#"{"transactionHashes":["0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"]}"#;
        let param: Params = serde_json::from_str(json).unwrap();
        match param {
            Params::TransactionReceipts(params) => {
                assert_eq!(
                    params.transaction_hashes,
                    Some(vec![B256::from(hex!(
                        "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"
                    ))])
                );
            }
            _ => panic!("Expected TransactionReceipts variant"),
        }

        // Test deserialization of transaction receipts parameters, with null transactionHashes
        let json = r#"{"transactionHashes":null}"#;
        let param: Params = serde_json::from_str(json).unwrap();
        match param {
            Params::TransactionReceipts(params) => {
                assert_eq!(params.transaction_hashes, None);
            }
            _ => panic!("Expected TransactionReceipts variant"),
        }

        // Test deserialization of transaction receipts parameters, with empty transactionHashes
        let json = r#"{"transactionHashes":[]}"#;
        let param: Params = serde_json::from_str(json).unwrap();
        match param {
            Params::TransactionReceipts(params) => {
                assert_eq!(params.transaction_hashes, Some(vec![]));
            }
            _ => panic!("Expected TransactionReceipts variant"),
        }
    }

    #[test]
    fn params_is_bool() {
        // Check if the `is_bool` method correctly identifies boolean parameters
        let param = Params::Bool(true);
        assert!(param.is_bool());

        let param = Params::None;
        assert!(!param.is_bool());

        let param = Params::Logs(Box::default());
        assert!(!param.is_bool());
    }

    #[test]
    fn params_is_logs() {
        // Check if the `is_logs` method correctly identifies log parameters
        let param = Params::Logs(Box::default());
        assert!(param.is_logs());

        let param = Params::None;
        assert!(!param.is_logs());

        let param = Params::Bool(true);
        assert!(!param.is_logs());
    }

    #[test]
    fn params_from_filter() {
        let filter = Filter::default();
        let param: Params = filter.clone().into();
        assert_eq!(param, Params::Logs(Box::new(filter)));
    }

    #[test]
    fn params_from_bool() {
        let param: Params = true.into();
        assert_eq!(param, Params::Bool(true));

        let param: Params = false.into();
        assert_eq!(param, Params::Bool(false));
    }

    #[test]
    fn params_from_transaction_receipts() {
        let params = TransactionReceiptsParams { transaction_hashes: Some(vec![B256::random()]) };
        let param: Params = params.clone().into();
        assert_eq!(param, Params::TransactionReceipts(params));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn params_serialize_none() {
        let param = Params::None;
        let serialized = serde_json::to_string(&param).unwrap();
        assert_eq!(serialized, "[]");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn params_serialize_bool() {
        let param = Params::Bool(true);
        let serialized = serde_json::to_string(&param).unwrap();
        assert_eq!(serialized, "true");

        let param = Params::Bool(false);
        let serialized = serde_json::to_string(&param).unwrap();
        assert_eq!(serialized, "false");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn params_serialize_logs() {
        let filter = Filter::default();
        let param = Params::Logs(Box::new(filter.clone()));
        let serialized = serde_json::to_string(&param).unwrap();
        let expected = serde_json::to_string(&filter).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn params_serialize_transaction_receipts() {
        let params = TransactionReceiptsParams {
            transaction_hashes: Some(vec![B256::from(hex!(
                "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"
            ))]),
        };
        let param = Params::TransactionReceipts(params);
        let serialized = serde_json::to_string(&param).unwrap();
        let expected = r#"{"transactionHashes":["0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"]}"#;
        assert_eq!(serialized, expected);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn params_transaction_hashes_limit() {
        // Test rejection of arrays exceeding the limit
        let large_array: Vec<_> = (0..=MAX_TX_HASHES).map(|i| format!("0x{:064x}", i)).collect();
        let json_payload = serde_json::json!({ "transactionHashes": large_array });
        let result: Result<TransactionReceiptsParams, _> = serde_json::from_value(json_payload);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceed max number of transaction hashes"));

        // Test acceptance of arrays at the limit
        let valid_array: Vec<_> = (0..MAX_TX_HASHES).map(|i| format!("0x{:064x}", i)).collect();
        let json_payload = serde_json::json!({ "transactionHashes": valid_array });
        let result: Result<TransactionReceiptsParams, _> = serde_json::from_value(json_payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().transaction_hashes.unwrap().len(), MAX_TX_HASHES);
    }
}
