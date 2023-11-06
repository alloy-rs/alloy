use alloy_primitives::U256;
use serde_json::value::RawValue;
use tokio::sync::{broadcast, oneshot};

use crate::pubsub::managers::InFlight;

/// Instructions for the pubsub service.
pub enum PubSubInstruction {
    /// Send a request.
    Request(InFlight),
    /// Get the subscription ID for a local ID.
    GetSub(U256, oneshot::Sender<broadcast::Receiver<Box<RawValue>>>),
    /// Unsubscribe from a subscription.
    Unsubscribe(U256),
}

impl std::fmt::Debug for PubSubInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(arg0) => f.debug_tuple("Request").field(arg0).finish(),
            Self::GetSub(arg0, _) => f.debug_tuple("GetSub").field(arg0).finish(),
            Self::Unsubscribe(arg0) => f.debug_tuple("Unsubscribe").field(arg0).finish(),
        }
    }
}
