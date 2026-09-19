use crate::{managers::InFlight, RawSubscription, UnsubscribeOutcome};
use alloy_primitives::B256;
use alloy_transport::TransportResult;
use std::fmt;
use tokio::sync::oneshot;

/// Instructions for the pubsub service.
pub enum PubSubInstruction {
    /// Send a request.
    Request(InFlight),
    /// Get the subscription ID for a local ID.
    GetSub(B256, oneshot::Sender<Option<RawSubscription>>),
    /// Unsubscribe from a subscription.
    Unsubscribe(B256),
    /// Unsubscribe and wait for the server-side cleanup to reach a terminal state.
    UnsubscribeAndWait(B256, oneshot::Sender<TransportResult<UnsubscribeOutcome>>),
}

impl fmt::Debug for PubSubInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(arg0) => f.debug_tuple("Request").field(arg0).finish(),
            Self::GetSub(arg0, _) => f.debug_tuple("GetSub").field(arg0).finish(),
            Self::Unsubscribe(arg0) => f.debug_tuple("Unsubscribe").field(arg0).finish(),
            Self::UnsubscribeAndWait(arg0, _) => {
                f.debug_tuple("UnsubscribeAndWait").field(arg0).finish()
            }
        }
    }
}
