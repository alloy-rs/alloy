use alloy_json_rpc::SerializedRequest;
use alloy_primitives::B256;
use serde_json::value::RawValue;
use std::hash::Hash;
use tokio::sync::broadcast;

#[derive(Clone)]
/// An active subscription.
pub(crate) struct ActiveSubscription {
    /// Cached hash of the request, used for sorting and equality.
    pub(crate) local_id: B256,
    /// The serialized subscription request.
    pub(crate) request: SerializedRequest,
    /// The channel via which notifications are broadcast.
    pub(crate) tx: broadcast::Sender<Box<RawValue>>,
}

// NB: We implement this to prevent any incorrect future implementations.
// See: https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq
impl Hash for ActiveSubscription {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.local_id.hash(state);
    }
}

impl PartialEq for ActiveSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.local_id == other.local_id
    }
}

impl Eq for ActiveSubscription {}

impl PartialOrd for ActiveSubscription {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActiveSubscription {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.local_id.cmp(&other.local_id)
    }
}

impl std::fmt::Debug for ActiveSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let channel_desc = format!("Channel status: {} subscribers", self.tx.receiver_count());

        f.debug_struct("ActiveSubscription")
            .field("req", &self.request)
            .field("tx", &channel_desc)
            .finish()
    }
}

impl ActiveSubscription {
    /// Create a new active subscription.
    pub(crate) fn new(request: SerializedRequest) -> (Self, broadcast::Receiver<Box<RawValue>>) {
        let local_id = request.params_hash();
        let (tx, rx) = broadcast::channel(16);
        (Self { request, local_id, tx }, rx)
    }

    /// Serialize the request as a boxed [`RawValue`].
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Notify the subscription channel of a new value, if any receiver exists.
    /// If no receiver exists, the notification is dropped.
    pub(crate) fn notify(&mut self, notification: Box<RawValue>) {
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(notification);
        }
    }
}
