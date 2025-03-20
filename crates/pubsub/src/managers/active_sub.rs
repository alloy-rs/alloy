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
    /// The channel via which notifications are received.
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
        let new_rx = self.tx.subscribe();

        // Check if any events are in queue
        let mut pending = self.tx.len();
        if pending == 0 {
            return RawSubscription { rx: new_rx, local_id: self.local_id };
        }

        let mut lock = self.rx.lock();
        if let Some(mut rx) = lock.deref_mut().take() {
            // Attempt to drain the queue by forwarding all pending notifications to the new
            // receiver.

            while let Ok(notification) = rx.try_recv() {
                self.notify(notification);
                pending -= 1;
                if pending == 0 {
                    break;
                }
            }
        }

        RawSubscription { rx: new_rx, local_id: self.local_id }
    }

    /// Notify the subscription channel of a new value, if any receiver exists.
    /// If no receiver exists, the notification is dropped.
    pub(crate) fn notify(&self, notification: Box<RawValue>) {
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(notification);
        }
    }
}
