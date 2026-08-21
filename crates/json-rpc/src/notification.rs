use crate::{Response, ResponsePayload};
use alloy_primitives::U256;
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};

/// A subscription ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(untagged)]
pub enum SubId {
    /// A number.
    Number(U256),
    /// A string.
    String(String),
}

impl From<U256> for SubId {
    fn from(value: U256) -> Self {
        Self::Number(value)
    }
}

impl From<String> for SubId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

/// An ethereum-style notification, not to be confused with a JSON-RPC
/// notification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthNotification<T = Box<serde_json::value::RawValue>> {
    /// The subscription ID.
    pub subscription: SubId,
    /// The notification payload.
    pub result: T,
}

/// An item received over an Ethereum pubsub transport.
///
/// Ethereum pubsub uses a non-standard JSON-RPC notification format. An item received over a pubsub
/// transport may be a JSON-RPC response or an Ethereum-style notification.
#[derive(Clone, Debug)]
pub enum PubSubItem {
    /// A [`Response`] to a JSON-RPC request.
    Response(Response),
    /// An Ethereum-style notification.
    Notification(EthNotification),
}

impl From<Response> for PubSubItem {
    fn from(response: Response) -> Self {
        Self::Response(response)
    }
}

impl From<EthNotification> for PubSubItem {
    fn from(notification: EthNotification) -> Self {
        Self::Notification(notification)
    }
}

impl<'de> Deserialize<'de> for PubSubItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PubSubItemVisitor;

        impl<'de> Visitor<'de> for PubSubItemVisitor {
            type Value = PubSubItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON-RPC response or an Ethereum-style notification")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut id = None;
                let mut result = None;
                let mut params = None;
                let mut error = None;

                // Drain the map into the appropriate fields.
                while let Ok(Some(key)) = map.next_key() {
                    match key {
                        "id" => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "result" => {
                            if result.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result = Some(map.next_value()?);
                        }
                        "params" => {
                            if params.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params = Some(map.next_value()?);
                        }
                        "error" => {
                            if error.is_some() {
                                return Err(serde::de::Error::duplicate_field("error"));
                            }
                            error = Some(map.next_value()?);
                        }
                        // Discard unknown fields.
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                // If it has an ID, it is a response.
                if let Some(id) = id {
                    let payload = error
                        .map(ResponsePayload::Failure)
                        .or_else(|| result.map(ResponsePayload::Success))
                        .ok_or_else(|| {
                            serde::de::Error::custom(
                                "missing `result` or `error` field in response",
                            )
                        })?;

                    Ok(Response { id, payload }.into())
                } else {
                    // Notifications cannot have an error.
                    if error.is_some() {
                        return Err(serde::de::Error::custom(
                            "unexpected `error` field in subscription notification",
                        ));
                    }
                    params
                        .map(PubSubItem::Notification)
                        .ok_or_else(|| serde::de::Error::missing_field("params"))
                }
            }
        }

        deserializer.deserialize_any(PubSubItemVisitor)
    }
}

/// One or more [`PubSubItem`]s received in a single pubsub transport message.
///
/// A pubsub server normally sends one JSON object per message. However, a
/// server answering a JSON-RPC batch request may reply with a single JSON array
/// containing several responses. This type deserializes either shape so batch
/// responses received over a pubsub transport are not dropped.
///
/// Each contained [`PubSubItem`] is dispatched and routed to its waiter
/// independently by JSON-RPC ID, so the order of items within a batch does not
/// matter.
#[derive(Clone, Debug)]
pub enum PubSubItems {
    /// A single item, received as a JSON object.
    Single(PubSubItem),
    /// Multiple items, received as one JSON array (a JSON-RPC batch response).
    Batch(Vec<PubSubItem>),
}

impl From<PubSubItem> for PubSubItems {
    fn from(item: PubSubItem) -> Self {
        Self::Single(item)
    }
}

impl IntoIterator for PubSubItems {
    type Item = PubSubItem;
    type IntoIter = PubSubItemsIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Single(item) => PubSubItemsIter::Single(Some(item)),
            Self::Batch(items) => PubSubItemsIter::Batch(items.into_iter()),
        }
    }
}

/// Owning iterator over the [`PubSubItem`]s in a [`PubSubItems`].
///
/// The single case does not allocate, keeping the common (non-batch) path cheap.
#[derive(Debug)]
pub enum PubSubItemsIter {
    /// Yields the single contained item once.
    Single(Option<PubSubItem>),
    /// Yields each item of a batch in order.
    Batch(std::vec::IntoIter<PubSubItem>),
}

impl Iterator for PubSubItemsIter {
    type Item = PubSubItem;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(item) => item.take(),
            Self::Batch(iter) => iter.next(),
        }
    }
}

impl<'de> Deserialize<'de> for PubSubItems {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PubSubItemsVisitor;

        impl<'de> Visitor<'de> for PubSubItemsVisitor {
            type Value = PubSubItems;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(
                    "a JSON-RPC response, an Ethereum-style notification, or a batch array of them",
                )
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(PubSubItems::Batch(items))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // Reuse the single-item map deserialization.
                PubSubItem::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(PubSubItems::Single)
            }
        }

        deserializer.deserialize_any(PubSubItemsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EthNotification, PubSubItem, SubId};
    use serde_json::json;

    #[test]
    fn deserializer_test() {
        // https://geth.ethereum.org/docs/interacting-with-geth/rpc/pubsub
        let notification = r#"{ "jsonrpc": "2.0", "method": "eth_subscription", "params": {"subscription": "0xcd0c3e8af590364c09d0fa6a1210faf5", "result": {"difficulty": "0xd9263f42a87", "uncles": []}} }
        "#;

        let deser = serde_json::from_str::<PubSubItem>(notification).unwrap();

        match deser {
            PubSubItem::Notification(EthNotification { subscription, result }) => {
                assert_eq!(
                    subscription,
                    SubId::Number("0xcd0c3e8af590364c09d0fa6a1210faf5".parse().unwrap())
                );
                assert_eq!(result.get(), r#"{"difficulty": "0xd9263f42a87", "uncles": []}"#);
            }
            _ => panic!("unexpected deserialization result"),
        }
    }

    #[test]
    fn subid_number() {
        let number = U256::from(123456u64);
        let subid: SubId = number.into();
        assert_eq!(subid, SubId::Number(number));
    }

    #[test]
    fn subid_string() {
        let string = "subscription_id".to_string();
        let subid: SubId = string.clone().into();
        assert_eq!(subid, SubId::String(string));
    }

    #[test]
    fn eth_notification_header() {
        let header = json!({
            "subscription": "0x123",
            "result": {
                "difficulty": "0xabc",
                "uncles": []
            }
        });

        let notification: EthNotification = serde_json::from_value(header).unwrap();
        assert_eq!(notification.subscription, SubId::Number(U256::from(0x123)));
        assert_eq!(notification.result.get(), r#"{"difficulty":"0xabc","uncles":[]}"#);
    }

    #[test]
    fn deserializer_test_valid_response() {
        // A valid JSON-RPC response with a result
        let response = r#"
            {
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x123456"
            }"#;

        let deser = serde_json::from_str::<PubSubItem>(response).unwrap();

        match deser {
            PubSubItem::Response(Response { id, payload }) => {
                assert_eq!(id, 1.into());
                match payload {
                    ResponsePayload::Success(result) => assert_eq!(result.get(), r#""0x123456""#),
                    _ => panic!("unexpected payload"),
                }
            }
            _ => panic!("unexpected deserialization result"),
        }
    }

    #[test]
    fn deserializer_test_error_response() {
        // A JSON-RPC response with an error
        let response = r#"
            {
                "jsonrpc": "2.0",
                "id": 2,
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            }"#;

        let deser = serde_json::from_str::<PubSubItem>(response).unwrap();

        match deser {
            PubSubItem::Response(Response { id, payload }) => {
                assert_eq!(id, 2.into());
                match payload {
                    ResponsePayload::Failure(error) => {
                        assert_eq!(error.code, -32601);
                        assert_eq!(error.message, "Method not found");
                    }
                    _ => panic!("unexpected payload"),
                }
            }
            _ => panic!("unexpected deserialization result"),
        }
    }

    #[test]
    fn deserializer_test_empty_notification() {
        // An empty notification to test deserialization handling
        let notification = r#"
            {
                "jsonrpc": "2.0",
                "method": "eth_subscription",
                "params": {
                    "subscription": "0x0",
                    "result": {}
                }
            }"#;

        let deser = serde_json::from_str::<PubSubItem>(notification).unwrap();

        match deser {
            PubSubItem::Notification(EthNotification { subscription, result }) => {
                assert_eq!(subscription, SubId::Number(U256::from(0u64)));
                assert_eq!(result.get(), r#"{}"#);
            }
            _ => panic!("unexpected deserialization result"),
        }
    }

    #[test]
    fn deserializer_test_invalid_structure() {
        // An invalid structure should fail deserialization
        let invalid_notification = r#"
           {
               "jsonrpc": "2.0",
               "method": "eth_subscription"
           }"#;

        let deser = serde_json::from_str::<PubSubItem>(invalid_notification);
        assert!(deser.is_err());
    }

    #[test]
    fn pubsub_items_single_object() {
        // A single JSON object deserializes into `Single`.
        let single = r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#;
        let items = serde_json::from_str::<PubSubItems>(single).unwrap();
        match items {
            PubSubItems::Single(PubSubItem::Response(resp)) => assert_eq!(resp.id, 1.into()),
            _ => panic!("expected a single response"),
        }
    }

    #[test]
    fn pubsub_items_batch_array() {
        // A JSON array (a batch response) deserializes into `Batch`, preserving
        // every element so each can be routed by its own id.
        let batch = r#"[
            {"jsonrpc":"2.0","id":1,"result":22},
            {"jsonrpc":"2.0","id":0,"result":11}
        ]"#;
        let items = serde_json::from_str::<PubSubItems>(batch).unwrap();
        let collected: Vec<_> = items.into_iter().collect();
        assert_eq!(collected.len(), 2);
        let ids: Vec<_> = collected
            .iter()
            .map(|item| match item {
                PubSubItem::Response(resp) => resp.id.clone(),
                _ => panic!("expected responses"),
            })
            .collect();
        assert_eq!(ids, vec![1.into(), 0.into()]);
    }

    #[test]
    fn pubsub_items_single_notification() {
        // Subscription notifications still parse via the single path.
        let notification = r#"{ "jsonrpc": "2.0", "method": "eth_subscription", "params": {"subscription": "0x1", "result": {}} }"#;
        let items = serde_json::from_str::<PubSubItems>(notification).unwrap();
        assert!(matches!(items, PubSubItems::Single(PubSubItem::Notification(_))));
    }

    #[test]
    fn deserializer_test_missing_fields() {
        // A notification missing essential fields should fail
        let missing_fields = r#"
           {
               "jsonrpc": "2.0",
               "method": "eth_subscription",
               "params": {}
           }"#;

        let deser = serde_json::from_str::<PubSubItem>(missing_fields);
        assert!(deser.is_err());
    }
}
