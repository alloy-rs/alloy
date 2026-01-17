//! MEV-share event type bindings

use alloy_primitives::{hex, Address, Bytes, TxHash, B256, U256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{array::TryFromSliceError, fmt::LowerHex, ops::Deref};

/// SSE event from the MEV-share endpoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Event {
    /// Transaction or bundle hash.
    pub hash: TxHash,
    /// Transactions from the event. If the event itself is a transaction, txs will only have one
    /// entry. Bundle events may have more.
    #[serde(rename = "txs", with = "null_sequence")]
    pub transactions: Vec<EventTransaction>,
    /// Event logs emitted by executing the transaction.
    #[serde(with = "null_sequence")]
    pub logs: Vec<EventTransactionLog>,
}

/// Transaction from the event
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventTransaction {
    /// Transaction recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// 4-byte-function selector
    #[serde(rename = "functionSelector")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_selector: Option<FunctionSelector>,
    /// Calldata of the transaction
    #[serde(rename = "callData")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calldata: Option<Bytes>,
}

/// A log produced by a transaction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventTransactionLog {
    /// The address of the contract that emitted the log
    pub address: Address,
    /// Topics of the log
    ///
    /// (In solidity: The first topic is the hash of the signature of the event
    /// (e.g. `Deposit(address,bytes32,uint256)`), except you declared the event
    /// with the anonymous specifier.)
    pub topics: Vec<B256>,
    /// The data of the log
    pub data: Bytes,
}

/// SSE event type of the event history endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)]
pub struct EventHistoryInfo {
    pub count: u64,
    pub min_block: u64,
    pub max_block: u64,
    pub min_timestamp: u64,
    pub max_timestamp: u64,
    pub max_limit: u64,
}

/// SSE event of the `history` endpoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventHistory {
    /// The block number of the event's block.
    pub block: u64,
    /// The timestamp when the event was emitted.
    pub timestamp: u64,
    /// Hint for the historic block.
    pub hint: Hint,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)]
pub struct Hint {
    #[serde(with = "null_sequence")]
    pub txs: Vec<EventTransaction>,
    pub hash: B256,
    #[serde(with = "null_sequence")]
    pub logs: Vec<EventTransactionLog>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mev_gas_price: Option<U256>,
}

/// Query params for the `history` endpoint
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[expect(missing_docs)]
pub struct EventHistoryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_end: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

#[expect(missing_docs)]
impl EventHistoryParams {
    pub const fn with_block_start(mut self, block_start: u64) -> Self {
        self.block_start = Some(block_start);
        self
    }

    pub const fn with_block_end(mut self, block_end: u64) -> Self {
        self.block_end = Some(block_end);
        self
    }

    pub const fn with_block_range(mut self, block_start: u64, block_end: u64) -> Self {
        self.block_start = Some(block_start);
        self.block_end = Some(block_end);
        self
    }

    pub const fn with_timestamp_start(mut self, timestamp_start: u64) -> Self {
        self.timestamp_start = Some(timestamp_start);
        self
    }

    pub const fn with_timestamp_end(mut self, timestamp_end: u64) -> Self {
        self.timestamp_end = Some(timestamp_end);
        self
    }

    pub const fn with_timestamp_range(mut self, timestamp_start: u64, timestamp_end: u64) -> Self {
        self.timestamp_start = Some(timestamp_start);
        self.timestamp_end = Some(timestamp_end);
        self
    }

    pub const fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub const fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// 4-byte-function selector
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunctionSelector(pub [u8; 4]);

// === impl FunctionSelector ===

impl FunctionSelector {
    fn hex_encode(&self) -> String {
        hex::encode(self.0.as_ref())
    }
}

impl Serialize for FunctionSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for FunctionSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let s = hex_str.strip_prefix("0x").unwrap_or(&hex_str);
        if s.len() != 8 {
            return Err(serde::de::Error::custom(format!(
                "Expected 4 byte function selector: {hex_str}"
            )));
        }

        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
        let selector = Self::try_from(bytes.as_slice()).map_err(serde::de::Error::custom)?;
        Ok(selector)
    }
}

impl AsRef<[u8]> for FunctionSelector {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl std::fmt::Debug for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FunctionSelector").field(&self.hex_encode()).finish()
    }
}

impl std::fmt::Display for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.hex_encode())
    }
}

impl LowerHex for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl Deref for FunctionSelector {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

impl From<[u8; 4]> for FunctionSelector {
    fn from(src: [u8; 4]) -> Self {
        Self(src)
    }
}

impl<'a> TryFrom<&'a [u8]> for FunctionSelector {
    type Error = TryFromSliceError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let sel: [u8; 4] = value.try_into()?;
        Ok(Self(sel))
    }
}

impl PartialEq<[u8; 4]> for FunctionSelector {
    fn eq(&self, other: &[u8; 4]) -> bool {
        other == &self.0
    }
}

/// Deserializes missing or null sequences as empty vectors.
mod null_sequence {
    use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

    pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: DeserializeOwned,
    {
        let s = Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default();
        Ok(s)
    }

    pub(crate) fn serialize<T, S>(val: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        if val.is_empty() {
            serializer.serialize_none()
        } else {
            val.serialize(serializer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_txs() {
        let s = "{\"hash\":\"0x9d525cbf4ed0cd367df93a685da93da036bf5c6d0d6e9e31945779ddbca31d3b\",\"logs\":[{\"address\":\"0x074201cb10b1efedbd8dec271c37687e1ab5be4e\",\"data\":\"0x\",\"topics\":[\"0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822\",\"0x0000000000000000000000000000000000000000000000000000000000000000\",\"0x0000000000000000000000000000000000000000000000000000000000000000\"]},{\"address\":\"0x0d4a11d5eeaac28ec3f61d100daf4d40471f1852\",\"data\":\"0x\",\"topics\":[\"0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822\",\"0x0000000000000000000000000000000000000000000000000000000000000000\",\"0x0000000000000000000000000000000000000000000000000000000000000000\"]}],\"txs\":null}";
        let _ev: Event = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_null_logs() {
        let s = "{\"hash\":\"0x8e32bfed609925168302ea538acd839ad34fdcd5d89dff67b19f9717d39abee6\",\"logs\":null,\"txs\":null}";
        let _ev: Event = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_sample() {
        let s = r#"{"hash":"0xc7dc06c994400830054ab815732d91275bc1241f9be62b62b687b7882f19b8d4","logs":null,"txs":[{"to":"0x0000c335bc9d5d1af0402cad63fa7f258363d71a","functionSelector":"0x696d2073","callData":"0x696d20736861726969696969696e67"}]}"#;
        let _ev: Event = serde_json::from_str(s).unwrap();
    }
}
