use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PubSubItem {
    Response(Response),
    Notification(EthNotification),
}
