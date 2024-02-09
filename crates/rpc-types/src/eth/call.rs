use crate::{request::TransactionRequest, BlockId, BlockOverrides};
use alloy_primitives::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Bundle of transactions
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Bundle {
    /// All transactions to execute
    pub transactions: Vec<TransactionRequest>,
    /// Block overrides to apply
    pub block_override: Option<BlockOverrides>,
}

/// State context for callMany
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StateContext {
    /// Block Number
    pub block_number: Option<BlockId>,
    /// Inclusive number of tx to replay in block. -1 means replay all
    pub transaction_index: Option<TransactionIndex>,
}

/// CallResponse for eth_callMany
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EthCallResponse {
    /// eth_call output (if no error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Bytes>,
    /// eth_call output (if error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl EthCallResponse {
    /// Returns the value if present, otherwise returns the error.
    pub fn ensure_ok(self) -> Result<Bytes, String> {
        match self.value {
            Some(output) => Ok(output),
            None => Err(self.error.unwrap_or_else(|| "Unknown error".to_string())),
        }
    }
}

/// Represents a transaction index where -1 means all transactions
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum TransactionIndex {
    /// -1 means all transactions
    #[default]
    All,
    /// Transaction index
    Index(usize),
}

impl TransactionIndex {
    /// Returns true if this is the all variant
    pub const fn is_all(&self) -> bool {
        matches!(self, TransactionIndex::All)
    }

    /// Returns the index if this is the index variant
    pub const fn index(&self) -> Option<usize> {
        match self {
            TransactionIndex::All => None,
            TransactionIndex::Index(idx) => Some(*idx),
        }
    }
}

impl Serialize for TransactionIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TransactionIndex::All => serializer.serialize_i8(-1),
            TransactionIndex::Index(idx) => idx.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for TransactionIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match isize::deserialize(deserializer)? {
            -1 => Ok(TransactionIndex::All),
            idx if idx < -1 => Err(serde::de::Error::custom(format!(
                "Invalid transaction index, expected -1 or positive integer, got {}",
                idx
            ))),
            idx => Ok(TransactionIndex::Index(idx as usize)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_index() {
        let s = "-1";
        let idx = serde_json::from_str::<TransactionIndex>(s).unwrap();
        assert_eq!(idx, TransactionIndex::All);

        let s = "5";
        let idx = serde_json::from_str::<TransactionIndex>(s).unwrap();
        assert_eq!(idx, TransactionIndex::Index(5));

        let s = "-2";
        let res = serde_json::from_str::<TransactionIndex>(s);
        assert!(res.is_err());
    }
}
