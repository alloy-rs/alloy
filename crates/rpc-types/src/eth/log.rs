#![allow(unknown_lints, non_local_definitions)]

use alloy_primitives::{LogData, B256};
use serde::{Deserialize, Serialize};

/// Ethereum Log emitted by a transaction
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    any(test, feature = "arbitrary"),
    derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
)]
#[serde(rename_all = "camelCase")]
pub struct Log<T = LogData> {
    #[serde(flatten)]
    /// Consensus log object
    pub inner: alloy_primitives::Log<T>,
    /// Hash of the block the transaction that emitted this log was mined in
    pub block_hash: Option<B256>,
    /// Number of the block the transaction that emitted this log was mined in
    #[serde(with = "alloy_serde::u64_opt_via_ruint")]
    pub block_number: Option<u64>,
    /// The timestamp of the block as proposed in:
    /// <https://ethereum-magicians.org/t/proposal-for-adding-blocktimestamp-to-logs-object-returned-by-eth-getlogs-and-related-requests>
    /// <https://github.com/ethereum/execution-apis/issues/295>
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::u64_opt_via_ruint",
        default
    )]
    pub block_timestamp: Option<u64>,
    /// Transaction Hash
    pub transaction_hash: Option<B256>,
    /// Index of the Transaction in the block
    #[serde(with = "alloy_serde::u64_opt_via_ruint")]
    pub transaction_index: Option<u64>,
    /// Log Index in Block
    #[serde(with = "alloy_serde::u64_opt_via_ruint")]
    pub log_index: Option<u64>,
    /// Geth Compatibility Field: whether this log was removed
    #[serde(default)]
    pub removed: bool,
}

impl<T> Log<T> {
    /// Getter for the address field. Shortcut for `log.inner.address`.
    pub const fn address(&self) -> alloy_primitives::Address {
        self.inner.address
    }

    /// Getter for the data field. Shortcut for `log.inner.data`.
    pub const fn data(&self) -> &T {
        &self.inner.data
    }
}

impl Log<LogData> {
    /// Getter for the topics field. Shortcut for `log.inner.topics()`.
    pub fn topics(&self) -> &[B256] {
        self.inner.topics()
    }

    /// Get the topic list, mutably. This gives access to the internal
    /// array, without allowing extension of that array. Shortcut for
    /// [`LogData::topics_mut`]
    pub fn topics_mut(&mut self) -> &mut [B256] {
        self.inner.data.topics_mut()
    }

    /// Decode the log data into a typed log.
    pub fn log_decode<T: alloy_sol_types::SolEvent>(&self) -> alloy_sol_types::Result<Log<T>> {
        let decoded = T::decode_log(&self.inner, false)?;
        Ok(Log {
            inner: decoded,
            block_hash: self.block_hash,
            block_number: self.block_number,
            block_timestamp: self.block_timestamp,
            transaction_hash: self.transaction_hash,
            transaction_index: self.transaction_index,
            log_index: self.log_index,
            removed: self.removed,
        })
    }
}

impl<T> alloy_rlp::Encodable for Log<T>
where
    for<'a> &'a T: Into<LogData>,
{
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.reserialize_inner().encode(out)
    }

    fn length(&self) -> usize {
        self.reserialize_inner().length()
    }
}

impl<T> Log<T>
where
    for<'a> &'a T: Into<LogData>,
{
    /// Reserialize the inner data, returning an [`alloy_primitives::Log`].
    pub fn reserialize_inner(&self) -> alloy_primitives::Log {
        alloy_primitives::Log { address: self.inner.address, data: (&self.inner.data).into() }
    }

    /// Reserialize the data, returning a new `Log` object wrapping an
    /// [`alloy_primitives::Log`]. this copies the log metadata, preserving
    /// the original object.
    pub fn reserialize(&self) -> Log<LogData> {
        Log {
            inner: self.reserialize_inner(),
            block_hash: self.block_hash,
            block_number: self.block_number,
            block_timestamp: self.block_timestamp,
            transaction_hash: self.transaction_hash,
            transaction_index: self.transaction_index,
            log_index: self.log_index,
            removed: self.removed,
        }
    }
}

impl<T> AsRef<alloy_primitives::Log<T>> for Log<T> {
    fn as_ref(&self) -> &alloy_primitives::Log<T> {
        &self.inner
    }
}

impl<T> AsMut<alloy_primitives::Log<T>> for Log<T> {
    fn as_mut(&mut self) -> &mut alloy_primitives::Log<T> {
        &mut self.inner
    }
}

impl<T> AsRef<T> for Log<T> {
    fn as_ref(&self) -> &T {
        &self.inner.data
    }
}

impl<T> AsMut<T> for Log<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner.data
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, Bytes};

    use super::*;
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn log_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _: Log = Log::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    fn serde_log() {
        let mut log = Log {
            inner: alloy_primitives::Log {
                address: Address::with_last_byte(0x69),
                data: alloy_primitives::LogData::new_unchecked(
                    vec![B256::with_last_byte(0x69)],
                    Bytes::from_static(&[0x69]),
                ),
            },
            block_hash: Some(B256::with_last_byte(0x69)),
            block_number: Some(0x69),
            block_timestamp: None,
            transaction_hash: Some(B256::with_last_byte(0x69)),
            transaction_index: Some(0x69),
            log_index: Some(0x69),
            removed: false,
        };
        let serialized = serde_json::to_string(&log).unwrap();
        assert_eq!(
            serialized,
            r#"{"address":"0x0000000000000000000000000000000000000069","topics":["0x0000000000000000000000000000000000000000000000000000000000000069"],"data":"0x69","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000069","blockNumber":"0x69","transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000069","transactionIndex":"0x69","logIndex":"0x69","removed":false}"#
        );

        let deserialized: Log = serde_json::from_str(&serialized).unwrap();
        assert_eq!(log, deserialized);

        log.block_timestamp = Some(0x69);
        let serialized = serde_json::to_string(&log).unwrap();
        assert_eq!(
            serialized,
            r#"{"address":"0x0000000000000000000000000000000000000069","topics":["0x0000000000000000000000000000000000000000000000000000000000000069"],"data":"0x69","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000069","blockNumber":"0x69","blockTimestamp":"0x69","transactionHash":"0x0000000000000000000000000000000000000000000000000000000000000069","transactionIndex":"0x69","logIndex":"0x69","removed":false}"#
        );

        let deserialized: Log = serde_json::from_str(&serialized).unwrap();
        assert_eq!(log, deserialized);
    }
}
