use crate::RawSubscription;
use alloy_json_rpc::SerializedRequest;
use alloy_primitives::{Keccak256, B256};
use parking_lot::Mutex;
use serde_json::value::RawValue;
use std::{fmt, hash::Hash, ops::DerefMut};
use tokio::sync::broadcast;

/// Returns the local id of a subscription request.
///
/// The id commits to the method as well as the params, so that subscriptions with identical params
/// on different methods (e.g. `admin_peerEvents` and `reth_subscribePersistedBlock`) stay distinct.
pub(crate) fn subscription_local_id(request: &SerializedRequest) -> B256 {
    let method = request.method();
    let mut hasher = Keccak256::new();
    hasher.update((method.len() as u64).to_be_bytes());
    hasher.update(method);
    if let Some(params) = request.params() {
        hasher.update(params.get());
    }
    hasher.finalize()
}

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
    /// The number of callers that were handed the local id but have not fetched a receiver yet.
    pub(crate) pending_claims: usize,
    /// The number of callers that were handed the local id and have not unsubscribed yet.
    pub(crate) shares: usize,
    /// The number of consecutive backends that died before acknowledging the resubscription.
    pub(crate) unacked_resubscribes: u32,
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
            .field("pending_claims", &self.pending_claims)
            .field("shares", &self.shares)
            .finish()
    }
}

impl ActiveSubscription {
    /// Create a new active subscription.
    pub(crate) fn new(request: SerializedRequest, channel_size: usize) -> Self {
        let local_id = subscription_local_id(&request);
        let (tx, rx) = broadcast::channel(channel_size);
        Self {
            request,
            local_id,
            tx,
            rx: Mutex::new(Some(rx)),
            pending_claims: 0,
            shares: 0,
            unacked_resubscribes: 0,
        }
    }

    /// Returns `true` if anyone holds a receiver for this subscription, or is about to fetch one.
    ///
    /// The stored initial receiver does not count.
    pub(crate) fn has_consumers(&self) -> bool {
        let initial = usize::from(self.rx.lock().is_some());
        self.pending_claims > 0 || self.tx.receiver_count() > initial
    }

    /// Serialize the request as a boxed [`RawValue`].
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Get a subscription.
    pub(crate) fn subscribe(&self) -> RawSubscription {
        // The first receiver is the initial one, so that notifications received before it was
        // requested are not dropped. Later receivers start at the tail: the initial receiver may
        // still hold notifications that other receivers already consumed.
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
