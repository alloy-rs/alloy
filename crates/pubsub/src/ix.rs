use crate::{managers::InFlight, RawSubscription};
use alloy_json_rpc::Id;
use alloy_primitives::B256;
use std::{fmt, sync::Arc};
use tokio::sync::oneshot;

/// Instructions for the pubsub service.
pub enum PubSubInstruction {
    /// Send a request.
    Request(InFlight),
    /// Cancel this exact request attempt.
    Cancel(Id, Arc<()>),
    /// Get the subscription ID for a local ID.
    GetSub(B256, oneshot::Sender<Option<RawSubscription>>),
    /// Unsubscribe from a subscription.
    Unsubscribe(B256),
}

impl fmt::Debug for PubSubInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(arg0) => f.debug_tuple("Request").field(arg0).finish(),
            Self::Cancel(id, _) => f.debug_tuple("Cancel").field(id).finish(),
            Self::GetSub(arg0, _) => f.debug_tuple("GetSub").field(arg0).finish(),
            Self::Unsubscribe(arg0) => f.debug_tuple("Unsubscribe").field(arg0).finish(),
        }
    }
}
