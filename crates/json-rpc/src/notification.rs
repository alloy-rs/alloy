use crate::{Response, ResponsePayload};
use alloy_primitives::U256;
use serde::{
    de::{MapAccess, Visitor},
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
                            let _ = map.next_value::<serde_json::Value>()?;
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

#[cfg(test)]
mod test {

    use crate::{EthNotification, PubSubItem, SubId};

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
}
