//! Strict payload field decoding, with the Engine API's exception for null fields.

use alloc::borrow::Cow;
use core::fmt;
use serde::{
    de::{DeserializeSeed, Error, IgnoredAny, IntoDeserializer, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

/// Rejects unknown non-null fields of a struct without buffering its values.
pub(super) struct StrictFields<D>(pub(super) D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for StrictFields<D> {
    type Error = D::Error;

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        // Preserve deserialize_struct and its field list: when flattened, Serde only supplies
        // this struct's fields and leaves sibling fields for the enclosing deserializer.
        self.0.deserialize_struct(name, fields, StructFields { inner: visitor, fields })
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.0.deserialize_any(visitor)
    }

    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
    }
}

struct StructFields<T> {
    inner: T,
    fields: &'static [&'static str],
}

impl<'de, V: Visitor<'de>> Visitor<'de> for StructFields<V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(formatter)
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        self.inner.visit_map(StructFields { inner: map, fields: self.fields })
    }

    fn visit_seq<S: SeqAccess<'de>>(self, seq: S) -> Result<Self::Value, S::Error> {
        self.inner.visit_seq(seq)
    }
}

impl<'de, M: MapAccess<'de>> MapAccess<'de> for StructFields<M> {
    type Error = M::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        #[derive(Deserialize)]
        struct FieldName<'a>(#[serde(borrow)] Cow<'a, str>);

        while let Some(FieldName(field)) = self.inner.next_key::<FieldName<'de>>()? {
            if self.fields.contains(&field.as_ref()) {
                return seed.deserialize(field.into_deserializer()).map(Some);
            }
            // Cancun treats null as absent, including for fields from later forks.
            if self.inner.next_value::<Option<IgnoredAny>>()?.is_some() {
                return Err(M::Error::unknown_field(&field, self.fields));
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.next_value_seed(seed)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadEnvelopeV5,
        ExecutionPayloadEnvelopeV6, ExecutionPayloadV3, ExecutionPayloadV4,
    };
    use alloc::{
        collections::BTreeMap,
        string::{String, ToString},
        vec::Vec,
    };
    use alloy_consensus::{Block, TxEnvelope};
    use alloy_primitives::{Bytes, B256};
    use core::fmt::Debug;
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use serde_json::{json, Value};

    fn payload() -> ExecutionPayloadV4 {
        let mut payload_inner =
            ExecutionPayloadV3::from_block_unchecked(B256::ZERO, &Block::<TxEnvelope>::default());
        payload_inner.payload_inner.payload_inner.transactions =
            vec![Bytes::from_static(&[0x02, 0x01])];
        payload_inner.payload_inner.withdrawals = vec![alloy_eips::eip4895::Withdrawal {
            index: 1,
            validator_index: 2,
            address: alloy_primitives::Address::with_last_byte(3),
            amount: 4,
        }];
        ExecutionPayloadV4 {
            payload_inner,
            block_access_list: Bytes::from_static(&[0xc0]),
            slot_number: 7,
        }
    }

    fn assert_strict<T: Serialize + DeserializeOwned + Debug + PartialEq>(
        payload: &T,
        unknown_fields: &[&str],
    ) {
        let original = serde_json::to_value(payload).unwrap();
        assert_eq!(&serde_json::from_value::<T>(original.clone()).unwrap(), payload);

        for &field in unknown_fields {
            for value in [
                json!("0x"),
                json!("0xc0"),
                json!("null"),
                json!(0),
                json!(-1),
                json!(1.5),
                json!(false),
                json!(true),
                json!([]),
                json!([null]),
                json!({}),
                json!({"nested": [null, {"value": true}]}),
            ] {
                let mut input = original.clone();
                input[field] = value;
                let error = serde_json::from_value::<T>(input.clone()).unwrap_err();
                assert!(error.to_string().contains(field), "{error}");
                let error = serde_json::from_str::<T>(&input.to_string()).unwrap_err();
                assert!(error.to_string().contains(field), "{error}");
            }
        }

        let mut input = original.clone();
        for &field in unknown_fields {
            input[field] = Value::Null;
        }
        assert_eq!(&serde_json::from_value::<T>(input.clone()).unwrap(), payload);
        assert_eq!(&serde_json::from_str::<T>(&input.to_string()).unwrap(), payload);

        for field in original.as_object().unwrap().keys() {
            let mut input = original.clone();
            input.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<T>(input.clone()).is_err(), "missing {field}");
            assert!(serde_json::from_str::<T>(&input.to_string()).is_err(), "missing {field}");
            input[field] = Value::Null;
            assert!(serde_json::from_str::<T>(&input.to_string()).is_err(), "null {field}");
            assert!(serde_json::from_value::<T>(input).is_err(), "null {field}");
        }
    }

    #[test]
    fn strict_payload_v3_fields() {
        assert_strict(
            &payload().payload_inner,
            &["blockAccessList", "slotNumber", "unknown", "blob_gas_used"],
        );
    }

    #[test]
    fn strict_payload_v4_fields() {
        assert_strict(&payload(), &["unknown", "block_access_list", "slot_number"]);
    }

    #[test]
    fn new_payload_v3_v4_params() {
        let payload = serde_json::to_value(payload().payload_inner).unwrap();
        let v3 = json!([payload, [], B256::ZERO]);
        let v4 = json!([payload, [], B256::ZERO, []]);
        type V3Params = (ExecutionPayloadV3, Vec<B256>, B256);
        type V4Params = (ExecutionPayloadV3, Vec<B256>, B256, Vec<Bytes>);
        serde_json::from_value::<V3Params>(v3.clone()).unwrap();
        serde_json::from_value::<V4Params>(v4.clone()).unwrap();

        for field in ["blockAccessList", "slotNumber", "unknown"] {
            let mut v3 = v3.clone();
            let mut v4 = v4.clone();
            v3[0][field] = json!("0x0");
            v4[0][field] = json!("0x0");
            assert!(serde_json::from_value::<V3Params>(v3.clone()).is_err());
            assert!(serde_json::from_value::<V4Params>(v4.clone()).is_err());
            v3[0][field] = Value::Null;
            v4[0][field] = Value::Null;
            serde_json::from_value::<V3Params>(v3).unwrap();
            serde_json::from_value::<V4Params>(v4).unwrap();
        }
    }

    fn assert_nested<T: Serialize + DeserializeOwned + Debug + PartialEq>(payload: T) {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Envelope<T> {
            execution_payload: T,
            metadata: bool,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Flattened<T> {
            #[serde(flatten)]
            payload: T,
            #[serde(flatten)]
            extra: BTreeMap<String, Value>,
        }

        let envelope = Envelope { execution_payload: payload, metadata: true };
        let input = serde_json::to_value(&envelope).unwrap();
        assert_eq!(serde_json::from_value::<Envelope<T>>(input.clone()).unwrap(), envelope);
        let mut invalid = input;
        invalid["executionPayload"]["unknown"] = json!(true);
        assert!(serde_json::from_value::<Envelope<T>>(invalid).is_err());

        let flattened = Flattened {
            payload: envelope.execution_payload,
            extra: BTreeMap::from([(String::from("extension"), json!({"nested": true}))]),
        };
        let input = serde_json::to_string(&flattened).unwrap();
        assert_eq!(serde_json::from_str::<Flattened<T>>(&input).unwrap(), flattened);
    }

    #[test]
    fn nested_and_flattened_payloads() {
        assert_nested(payload().payload_inner);
        assert_nested(payload());
    }

    fn assert_json<T: DeserializeOwned + Debug + PartialEq>(input: &str, expected: &T) {
        assert_eq!(&serde_json::from_str::<T>(input).unwrap(), expected);
        assert_eq!(&serde_json::from_slice::<T>(input.as_bytes()).unwrap(), expected);
        #[cfg(feature = "std")]
        assert_eq!(&serde_json::from_reader::<_, T>(input.as_bytes()).unwrap(), expected);
    }

    fn assert_field_order<T: Serialize + DeserializeOwned + Debug + PartialEq>(payload: T) {
        let original = serde_json::to_value(&payload).unwrap();
        let fields: Vec<_> = original
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| format!("{}:{value}", serde_json::to_string(key).unwrap()))
            .collect();

        for index in 0..=fields.len() {
            let mut fields = fields.clone();
            fields.insert(index, r#""unknown":null,"another":null"#.into());
            let input = format!("{{{}}}", fields.join(","));
            assert_json(&input, &payload);

            for extra in [
                r#""unknown":null,"another":false"#,
                r#""unknown":false,"another":null"#,
                r#""unknown":null,"unknown":{}"#,
                r#""unknown":[],"unknown":null"#,
            ] {
                fields[index] = extra.into();
                let input = format!("{{{}}}", fields.join(","));
                let error = serde_json::from_str::<T>(&input).unwrap_err();
                assert!(error.to_string().contains("unknown field"), "{error}");
            }
        }
    }

    #[test]
    fn unknown_fields_at_any_position() {
        assert_field_order(payload().payload_inner);
        assert_field_order(payload());
    }

    fn assert_duplicate_fields<T: Serialize + DeserializeOwned + Debug>(payload: T) {
        let original = serde_json::to_value(payload).unwrap();
        let serialized = original.to_string();
        let fields = &serialized[1..serialized.len() - 1];
        // Construct JSON directly because Value would discard duplicate keys.
        for (field, value) in original.as_object().unwrap() {
            let input = format!(r#"{{{fields},"{field}":{value},"unknown":null}}"#);
            let error = serde_json::from_str::<T>(&input).unwrap_err();
            assert!(error.to_string().contains(&format!("duplicate field `{field}`")), "{error}");
        }
    }

    #[test]
    fn duplicate_known_fields_are_rejected() {
        assert_duplicate_fields(payload().payload_inner);
        assert_duplicate_fields(payload());
    }

    fn assert_escaped_fields<T: Serialize + DeserializeOwned + Debug + PartialEq>(payload: T) {
        let original = serde_json::to_string(&payload).unwrap();
        let escaped = original.replace("parentHash", r"parent\u0048ash");
        assert_json(&escaped, &payload);

        let fields = &escaped[1..escaped.len() - 1];
        assert_json(&format!(r#"{{{fields},"un\u006bnown":null}}"#), &payload);
        let error =
            serde_json::from_str::<T>(&format!(r#"{{{fields},"un\u006bnown":true}}"#)).unwrap_err();
        assert!(error.to_string().contains("unknown field `unknown`"), "{error}");
        let error = serde_json::from_str::<T>(&format!(
            r#"{{{fields},"parentHash":{}}}"#,
            serde_json::to_value(&payload).unwrap()["parentHash"]
        ))
        .unwrap_err();
        assert!(error.to_string().contains("duplicate field `parentHash`"), "{error}");
    }

    #[test]
    fn escaped_field_names() {
        assert_escaped_fields(payload().payload_inner);
        assert_escaped_fields(payload());
    }

    fn assert_envelope<T: Serialize + DeserializeOwned + Debug + PartialEq>(envelope: T) {
        let mut input = serde_json::to_value(&envelope).unwrap();
        // The payload's strictness must not apply to the enclosing response's fields.
        input["unknown"] = json!({"metadata": true});
        input["executionPayload"]["unknown"] = Value::Null;
        assert_json(&input.to_string(), &envelope);
        input["executionPayload"]["unknown"] = json!(true);
        assert!(serde_json::from_value::<T>(input.clone()).is_err());
        assert!(serde_json::from_str::<T>(&input.to_string()).is_err());
    }

    #[test]
    fn execution_payload_envelopes_v3_through_v6() {
        let v3 = ExecutionPayloadEnvelopeV3 {
            execution_payload: payload().payload_inner,
            block_value: alloy_primitives::U256::from(42),
            blobs_bundle: Default::default(),
            should_override_builder: true,
        };
        let v4 = ExecutionPayloadEnvelopeV4 {
            envelope_inner: v3.clone(),
            execution_requests: Default::default(),
        };
        let v5 = ExecutionPayloadEnvelopeV5 {
            execution_payload: v3.execution_payload.clone(),
            block_value: v3.block_value,
            blobs_bundle: Default::default(),
            should_override_builder: true,
            execution_requests: Default::default(),
        };
        let v6 = ExecutionPayloadEnvelopeV6 {
            execution_payload: payload(),
            block_value: v3.block_value,
            blobs_bundle: Default::default(),
            should_override_builder: true,
            execution_requests: Default::default(),
        };
        assert_envelope(v3);
        assert_envelope(v4);
        assert_envelope(v5);
        assert_envelope(v6);
    }

    fn assert_flattened_siblings<T: Serialize + DeserializeOwned + Debug + PartialEq>(payload: T) {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Extension {
            metadata: Value,
        }

        #[derive(Debug, Deserialize)]
        struct PayloadFirst<T> {
            #[serde(flatten)]
            payload: T,
            #[serde(flatten)]
            extension: Extension,
        }

        #[derive(Debug, Deserialize)]
        struct PayloadLast<T> {
            #[serde(flatten)]
            extension: Extension,
            #[serde(flatten)]
            payload: T,
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct StrictOuter<T> {
            #[serde(flatten)]
            payload: T,
            metadata: Value,
        }

        let original = serde_json::to_string(&payload).unwrap();
        let fields = &original[1..original.len() - 1];
        for metadata in [json!(null), json!(true), json!({"nested": [1, 2]})] {
            for input in [
                format!(r#"{{"metadata":{metadata},{fields}}}"#),
                format!(r#"{{{fields},"metadata":{metadata}}}"#),
            ] {
                let first = serde_json::from_str::<PayloadFirst<T>>(&input).unwrap();
                let last = serde_json::from_str::<PayloadLast<T>>(&input).unwrap();
                assert_eq!(first.payload, payload);
                assert_eq!(last.payload, payload);
                assert_eq!(first.extension, last.extension);
                assert_eq!(first.extension.metadata, metadata);
                let outer = serde_json::from_str::<StrictOuter<T>>(&input).unwrap();
                assert_eq!(outer.payload, payload);
                assert_eq!(outer.metadata, metadata);
                let mut input: Value = serde_json::from_str(&input).unwrap();
                input["unknown"] = json!(true);
                let error = serde_json::from_value::<StrictOuter<T>>(input).unwrap_err();
                assert!(error.to_string().contains("unknown field"), "{error}");
            }
        }
    }

    #[test]
    fn flattened_siblings_and_strict_outer() {
        assert_flattened_siblings(payload().payload_inner);
        assert_flattened_siblings(payload());
    }
}
