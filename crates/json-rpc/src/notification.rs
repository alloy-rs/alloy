use alloy_primitives::U256;
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Serialize,
};

use crate::Response;

/// An ethereum-style notification, not to be confused with a JSON-RPC
/// notification.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EthNotification<T = Box<serde_json::value::RawValue>> {
    pub subscription: U256,
    pub result: T,
}

/// An item received over an Ethereum pubsub transport. Ethereum pubsub uses a
/// non-standard JSON-RPC notification format. An item received over a pubsub
/// transport may be a JSON-RPC response or an Ethereum-style notification.
#[derive(Debug, Clone)]
pub enum PubSubItem {
    Response(Response),
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

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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
                        _ => {
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                if let Some(id) = id {
                    if subscription.is_some() {
                        return Err(serde::de::Error::custom(
                            "unexpected subscription in pubsub item",
                        ));
                    }

                    let payload = if error.is_some() {
                        crate::ResponsePayload::Failure(error.unwrap())
                    } else {
                        if result.is_none() {
                            return Err(serde::de::Error::missing_field("result"));
                        }
                        crate::ResponsePayload::Success(result.unwrap())
                    };
                    Ok(PubSubItem::Response(Response { id, payload }))
                } else {
                    if error.is_some() {
                        return Err(serde::de::Error::custom(
                            "unexpected `error` field in subscription notification",
                        ));
                    }
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
