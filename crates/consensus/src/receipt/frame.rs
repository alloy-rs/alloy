use crate::{Eip658Value, TxReceipt};
use alloc::vec::Vec;
use alloy_eips::eip8141::FrameReceiptPayload;
use alloy_primitives::{logs_bloom, Bloom, Log};
use alloy_rlp::{BufMut, Decodable, Encodable};
use core::fmt;

#[cfg(feature = "serde")]
use alloy_eips::eip8141::{FrameGasUsed, FrameReceipt, FrameStatus};
#[cfg(feature = "serde")]
use alloy_primitives::Address;

/// An [EIP-8141] receipt payload together with the transaction logs flattened across frames.
///
/// The payload is the consensus representation and the only part that is RLP encoded. The
/// flattened logs are derived from it and cached so that [`TxReceipt::logs`] can return a slice
/// without allocating on every access. Both are private so that they cannot diverge.
///
/// With the `serde` feature this serializes as the fields of a JSON-RPC receipt without the
/// `type` tag, which belongs to the enclosing envelope. `status`, `logs`, `logsBloom` and the
/// per-frame `gasUsed` are derived from the payload on serialization. On deserialization the
/// first three are ignored and `gasUsed` must equal the sum of `executionGasUsed` and
/// `stateGasUsed`.
///
/// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameReceiptEnvelope<T = Log> {
    /// The consensus EIP-8141 receipt payload.
    payload: FrameReceiptPayload<T>,
    /// Logs of all frames in frame execution order.
    logs: Vec<T>,
}

impl<T> FrameReceiptEnvelope<T> {
    /// Creates a frame receipt envelope from its consensus payload.
    pub fn new(payload: FrameReceiptPayload<T>) -> Self
    where
        T: Clone,
    {
        payload.into()
    }

    /// Returns the consensus EIP-8141 receipt payload.
    pub const fn payload(&self) -> &FrameReceiptPayload<T> {
        &self.payload
    }

    /// Returns the logs of all frames in frame execution order.
    pub const fn logs(&self) -> &[T] {
        self.logs.as_slice()
    }

    /// Returns true if every frame succeeded.
    ///
    /// The receipt payload only has a status per frame. This aggregate is a convenience and not
    /// part of the consensus encoding.
    pub fn status(&self) -> bool {
        self.payload.frame_receipts.iter().all(|frame| frame.status.is_success())
    }

    /// Splits this envelope into its consensus payload and the flattened logs.
    pub fn into_parts(self) -> (FrameReceiptPayload<T>, Vec<T>) {
        (self.payload, self.logs)
    }

    /// Converts the receipt's log type by applying a function to each log.
    ///
    /// Returns the receipt with the new log type.
    pub fn map_logs<U: Clone>(self, f: impl FnMut(T) -> U) -> FrameReceiptEnvelope<U> {
        self.payload.map_logs(f).into()
    }
}

impl<T: Clone> From<FrameReceiptPayload<T>> for FrameReceiptEnvelope<T> {
    fn from(payload: FrameReceiptPayload<T>) -> Self {
        let logs = payload
            .frame_receipts
            .iter()
            .flat_map(|receipt| receipt.logs.iter().cloned())
            .collect();
        Self { payload, logs }
    }
}

impl<T> TxReceipt for FrameReceiptEnvelope<T>
where
    T: AsRef<Log> + Clone + fmt::Debug + PartialEq + Eq + Send + Sync,
{
    type Log = T;

    fn status_or_post_state(&self) -> Eip658Value {
        self.status().into()
    }

    fn status(&self) -> bool {
        Self::status(self)
    }

    fn bloom(&self) -> Bloom {
        logs_bloom(self.logs.iter().map(AsRef::as_ref))
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.payload.cumulative_gas_used
    }

    fn logs(&self) -> &[Self::Log] {
        &self.logs
    }

    fn into_logs(self) -> Vec<Self::Log>
    where
        Self::Log: Clone,
    {
        self.logs
    }
}

impl<T: Encodable> Encodable for FrameReceiptEnvelope<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.payload.encode(out);
    }

    fn length(&self) -> usize {
        self.payload.length()
    }
}

impl<T: Decodable + Clone> Decodable for FrameReceiptEnvelope<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        FrameReceiptPayload::<T>::decode(buf).map(Into::into)
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for FrameReceiptEnvelope<T>
where
    T: serde::Serialize + AsRef<Log>,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct FrameReceiptHelper<'a, T> {
            status: FrameStatus,
            #[serde(with = "alloy_serde::quantity")]
            gas_used: u64,
            #[serde(with = "alloy_serde::quantity")]
            execution_gas_used: u64,
            #[serde(with = "alloy_serde::quantity")]
            state_gas_used: u64,
            logs: &'a [T],
        }

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Helper<'a, T> {
            #[serde(with = "alloy_serde::quantity")]
            status: u8,
            #[serde(with = "alloy_serde::quantity")]
            cumulative_gas_used: u64,
            logs: &'a [T],
            logs_bloom: Bloom,
            payer: Address,
            frame_receipts: Vec<FrameReceiptHelper<'a, T>>,
        }

        Helper {
            status: self.status().into(),
            cumulative_gas_used: self.payload.cumulative_gas_used,
            logs: &self.logs,
            logs_bloom: logs_bloom(self.logs.iter().map(AsRef::as_ref)),
            payer: self.payload.payer,
            frame_receipts: self
                .payload
                .frame_receipts
                .iter()
                .map(|receipt| FrameReceiptHelper {
                    status: receipt.status,
                    gas_used: receipt.gas_used.execution.saturating_add(receipt.gas_used.state),
                    execution_gas_used: receipt.gas_used.execution,
                    state_gas_used: receipt.gas_used.state,
                    logs: &receipt.logs,
                })
                .collect(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T> serde::Deserialize<'de> for FrameReceiptEnvelope<T>
where
    T: serde::Deserialize<'de> + Clone,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FrameReceiptHelper<T> {
            status: FrameStatus,
            #[serde(with = "alloy_serde::quantity")]
            gas_used: u64,
            #[serde(with = "alloy_serde::quantity")]
            execution_gas_used: u64,
            #[serde(with = "alloy_serde::quantity")]
            state_gas_used: u64,
            logs: Vec<T>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Helper<T> {
            #[serde(with = "alloy_serde::quantity")]
            cumulative_gas_used: u64,
            payer: Address,
            frame_receipts: Vec<FrameReceiptHelper<T>>,
        }

        let Helper { cumulative_gas_used, payer, frame_receipts } =
            Helper::<T>::deserialize(deserializer)?;
        let frame_receipts = frame_receipts
            .into_iter()
            .map(|receipt| {
                let FrameReceiptHelper {
                    status,
                    gas_used,
                    execution_gas_used: execution,
                    state_gas_used: state,
                    logs,
                } = receipt;
                if gas_used != execution.saturating_add(state) {
                    return Err(serde::de::Error::custom("inconsistent frame gas used"));
                }
                Ok(FrameReceipt { status, gas_used: FrameGasUsed { execution, state }, logs })
            })
            .collect::<Result<_, D::Error>>()?;

        Ok(FrameReceiptPayload { cumulative_gas_used, payer, frame_receipts }.into())
    }
}

/// Serializes only the payload. The flattened logs are rebuilt on deserialization.
#[cfg(feature = "borsh")]
impl<T: borsh::BorshSerialize> borsh::BorshSerialize for FrameReceiptEnvelope<T> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.payload.serialize(writer)
    }
}

#[cfg(feature = "borsh")]
impl<T: borsh::BorshDeserialize + Clone> borsh::BorshDeserialize for FrameReceiptEnvelope<T> {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        FrameReceiptPayload::<T>::deserialize_reader(reader).map(Into::into)
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T> arbitrary::Arbitrary<'a> for FrameReceiptEnvelope<T>
where
    T: arbitrary::Arbitrary<'a> + Clone,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        FrameReceiptPayload::<T>::arbitrary(u).map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_eips::eip8141::{FrameGasUsed, FrameReceipt, FrameStatus};
    use alloy_primitives::{Address, LogData};

    fn log(byte: u8) -> Log {
        Log { address: Address::repeat_byte(byte), data: LogData::default() }
    }

    fn payload() -> FrameReceiptPayload {
        FrameReceiptPayload {
            cumulative_gas_used: 42,
            payer: Address::repeat_byte(0x11),
            frame_receipts: vec![
                FrameReceipt {
                    status: FrameStatus::Success,
                    gas_used: FrameGasUsed { execution: 21, state: 1 },
                    logs: vec![log(1)],
                },
                FrameReceipt {
                    status: FrameStatus::Success,
                    gas_used: FrameGasUsed { execution: 7, state: 0 },
                    logs: vec![log(2), log(3)],
                },
            ],
        }
    }

    #[test]
    fn rlp_roundtrip_encodes_only_the_payload() {
        let payload = payload();
        let envelope = FrameReceiptEnvelope::new(payload.clone());
        assert_eq!(envelope.logs(), [log(1), log(2), log(3)]);

        let encoded = alloy_rlp::encode(&envelope);
        assert_eq!(encoded, alloy_rlp::encode(&payload));
        assert_eq!(encoded.len(), envelope.length());

        let decoded = FrameReceiptEnvelope::<Log>::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn tx_receipt_is_derived_from_frames() {
        let mut payload = payload();
        let envelope = FrameReceiptEnvelope::new(payload.clone());
        assert!(TxReceipt::status(&envelope));
        assert_eq!(envelope.status_or_post_state(), Eip658Value::Eip658(true));
        assert_eq!(envelope.cumulative_gas_used(), 42);
        assert_eq!(envelope.bloom(), logs_bloom(&[log(1), log(2), log(3)]));
        assert_eq!(envelope.bloom_cheap(), None);
        assert_eq!(envelope.into_logs(), [log(1), log(2), log(3)]);

        payload.frame_receipts[1].status = FrameStatus::SkippedAtomicBatch;
        assert!(!TxReceipt::status(&FrameReceiptEnvelope::new(payload)));
    }

    #[test]
    fn map_logs_keeps_payload_and_flattened_logs_in_sync() {
        let envelope = FrameReceiptEnvelope::new(payload());

        let mut calls = 0;
        let mapped = envelope.map_logs(|_| {
            calls += 1;
            calls
        });

        assert_eq!(calls, 3);
        assert_eq!(mapped.logs(), [1, 2, 3]);
        assert_eq!(mapped.payload().frame_receipts[0].logs, [1]);
        assert_eq!(mapped.payload().frame_receipts[1].logs, [2, 3]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_roundtrip() {
        let mut payload = payload();
        payload.frame_receipts[1].status = FrameStatus::Failure;
        let envelope = FrameReceiptEnvelope::new(payload);

        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "status": "0x0",
                "cumulativeGasUsed": "0x2a",
                "logs": [log(1), log(2), log(3)],
                "logsBloom": logs_bloom(&[log(1), log(2), log(3)]),
                "payer": "0x1111111111111111111111111111111111111111",
                "frameReceipts": [
                    {
                        "status": "0x1",
                        "gasUsed": "0x16",
                        "executionGasUsed": "0x15",
                        "stateGasUsed": "0x1",
                        "logs": [log(1)],
                    },
                    {
                        "status": "0x0",
                        "gasUsed": "0x7",
                        "executionGasUsed": "0x7",
                        "stateGasUsed": "0x0",
                        "logs": [log(2), log(3)],
                    },
                ],
            })
        );

        assert_eq!(serde_json::from_value::<FrameReceiptEnvelope>(json).unwrap(), envelope);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_roundtrip_as_internally_tagged_variant() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(tag = "type")]
        enum Tagged {
            #[serde(rename = "0x6")]
            Frame(FrameReceiptEnvelope),
        }

        let envelope = FrameReceiptEnvelope::new(payload());
        let mut expected = serde_json::to_value(&envelope).unwrap();
        expected.as_object_mut().unwrap().insert("type".into(), "0x6".into());

        let tagged = Tagged::Frame(envelope);
        let json = serde_json::to_value(&tagged).unwrap();
        assert_eq!(json, expected);
        assert_eq!(serde_json::from_value::<Tagged>(json).unwrap(), tagged);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_deserialize_validates_frames_and_ignores_derived_fields() {
        let receipt = |status: &str, gas_used: &str| {
            serde_json::json!({
                "cumulativeGasUsed": "0x2a",
                "payer": "0x1111111111111111111111111111111111111111",
                "frameReceipts": [
                    {
                        "status": status,
                        "gasUsed": gas_used,
                        "executionGasUsed": "0x1",
                        "stateGasUsed": "0x2",
                        "logs": [log(1)],
                    },
                ],
            })
        };

        let envelope =
            serde_json::from_value::<FrameReceiptEnvelope>(receipt("0x2", "0x3")).unwrap();
        assert_eq!(
            envelope,
            FrameReceiptEnvelope::new(FrameReceiptPayload {
                cumulative_gas_used: 42,
                payer: Address::repeat_byte(0x11),
                frame_receipts: vec![FrameReceipt {
                    status: FrameStatus::SkippedAtomicBatch,
                    gas_used: FrameGasUsed { execution: 1, state: 2 },
                    logs: vec![log(1)],
                }],
            })
        );

        let err =
            serde_json::from_value::<FrameReceiptEnvelope>(receipt("0x2", "0x4")).unwrap_err();
        assert_eq!(err.to_string(), "inconsistent frame gas used");

        serde_json::from_value::<FrameReceiptEnvelope>(receipt("0x3", "0x3")).unwrap_err();
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn borsh_roundtrip_encodes_only_the_payload() {
        let payload = payload().map_logs(|log| log.address[0]);
        let envelope = FrameReceiptEnvelope::new(payload.clone());
        assert_eq!(envelope.logs(), [1, 2, 3]);

        let encoded = borsh::to_vec(&envelope).unwrap();
        assert_eq!(encoded, borsh::to_vec(&payload).unwrap());
        assert_eq!(borsh::from_slice::<FrameReceiptEnvelope<u8>>(&encoded).unwrap(), envelope);
    }
}
