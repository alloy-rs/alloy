use crate::RawSubscription;
use alloy_json_rpc::SerializedRequest;
use alloy_primitives::B256;
use parking_lot::Mutex;
use serde_json::value::RawValue;
use std::{fmt, hash::Hash, ops::DerefMut};
use tokio::sync::broadcast;

/// An active subscription.
pub(crate) struct ActiveSubscription {
    /// Cached hash of the request, used for sorting and equality.
    pub(crate) local_id: B256,
    /// The serialized subscription request.
    pub(crate) request: SerializedRequest,
    /// The channel via which notifications are broadcast.
    pub(crate) tx: broadcast::Sender<Box<RawValue>>,
    /// The initial channel via which notifications are received.
    ///
    /// This is stored so that we don't drop any notifications between initializing
    /// using [`ActiveSubscription::new`] and [`ActiveSubscription::subscribe`]. Ref: <https://github.com/alloy-rs/alloy/issues/2187>
    ///
    /// This is wrapped in a [`Mutex`] to allow for mutable access to the receiver without making
    /// [`ActiveSubscription::subscribe`] require mutable self.
    pub(crate) rx: Mutex<Option<broadcast::Receiver<Box<RawValue>>>>,
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

impl fmt::Debug for ActiveSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveSubscription")
            .field("local_id", &self.local_id)
            .field("request", &self.request)
            .field("subscribers", &self.tx.receiver_count())
            .finish()
    }
}

impl ActiveSubscription {
    /// Create a new active subscription.
    pub(crate) fn new(request: SerializedRequest, channel_size: usize) -> Self {
        let local_id = request.params_hash();
        let (tx, rx) = broadcast::channel(channel_size);
        Self { request, local_id, tx, rx: Mutex::new(Some(rx)) }
    }

    /// Serialize the request as a boxed [`RawValue`].
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Get a subscription.
    pub(crate) fn subscribe(&self) -> RawSubscription {
        if self.tx.is_empty() {
            // If there are no pending notifications, we can subscribe directly and return a new
            // subscriber.
            return RawSubscription { rx: self.tx.subscribe(), local_id: self.local_id };
        }

        // If there are pending notifications, we need to ensure that they are not dropped.
        // Hence, we first try to return the initial receiver (if it exists), which will receive
        // those pending notifications.
        // Ref: <https://github.com/alloy-rs/alloy/issues/2187>
        RawSubscription {
            rx: self.rx.lock().deref_mut().take().unwrap_or_else(|| self.tx.subscribe()),
            local_id: self.local_id,
        }
    }

    /// Notify the subscription channel of a new value, if any receiver exists.
    /// If no receiver exists, the notification is dropped.
    pub(crate) fn notify(&self, notification: Box<RawValue>) {
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(notification);
        }
    }
}
