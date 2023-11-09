use alloy_primitives::U256;
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Serialize,
};

use crate::{Response, ResponsePayload};

/// An ethereum-style notification, not to be confused with a JSON-RPC
/// notification.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EthNotification<T = Box<serde_json::value::RawValue>> {
    /// The subscription ID.
    pub subscription: U256,
    /// The notification payload.
    pub result: T,
}

/// An item received over an Ethereum pubsub transport. Ethereum pubsub uses a
/// non-standard JSON-RPC notification format. An item received over a pubsub
/// transport may be a JSON-RPC response or an Ethereum-style notification.
#[derive(Debug, Clone)]
pub enum PubSubItem {
    /// A [`Response`] to a JSON-RPC request.
    Response(Response),
    /// An Ethereum-style notification.
    Notification(EthNotification),
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
                let mut subscription = None;
                let mut result = None;
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
                        "subscription" => {
                            if subscription.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription = Some(map.next_value()?);
                        }
                        "result" => {
                            if result.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result = Some(map.next_value()?);
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
                    if subscription.is_some() {
                        return Err(serde::de::Error::custom(
                            "unexpected subscription in pubsub item",
                        ));
                    }
                    // We need to differentiate error vs result here.
                    let payload = if let Some(error) = error {
                        ResponsePayload::Failure(error)
                    } else if let Some(result) = result {
                        ResponsePayload::Success(result)
                    } else {
                        return Err(serde::de::Error::custom(
                            "missing `result` or `error` field in response",
                        ));
                    };
                    Ok(PubSubItem::Response(Response { id, payload }))
                } else {
                    // Notifications cannot have an error.
                    if error.is_some() {
                        return Err(serde::de::Error::custom(
                            "unexpected `error` field in subscription notification",
                        ));
                    }
                    // Notifications must have a subscription and a result.
                    if subscription.is_none() {
                        return Err(serde::de::Error::missing_field("subscription"));
                    }
                    if result.is_none() {
                        return Err(serde::de::Error::missing_field("result"));
                    }

                    Ok(PubSubItem::Notification(EthNotification {
                        subscription: subscription.unwrap(),
                        result: result.unwrap(),
                    }))
                }
            }
        }

        deserializer.deserialize_any(PubSubItemVisitor)
    }
}
