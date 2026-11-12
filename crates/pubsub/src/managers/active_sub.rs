use alloy_json_rpc::SerializedRequest;
use alloy_primitives::{map::B256Map, Keccak256, B256};
use serde_json::value::RawValue;
use std::{fmt, hash::Hash};
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
    /// The shares of the callers that were handed a local id for this subscription, by that id.
    pub(crate) shares: B256Map<Share>,
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
            .field("shares", &self.shares.len())
            .finish()
    }
}

impl ActiveSubscription {
    /// Create a new active subscription.
    pub(crate) fn new(request: SerializedRequest) -> Self {
        let local_id = subscription_local_id(&request);
        Self { request, local_id, shares: B256Map::default(), unacked_resubscribes: 0 }
    }

    /// Returns `true` if anyone holds a receiver for this subscription, or is about to fetch one.
    pub(crate) fn has_consumers(&self) -> bool {
        self.shares.values().any(Share::is_live)
    }

    /// Serialize the request as a boxed [`RawValue`].
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Notify the channel of every share that has a receiver of a new value. If no receiver
    /// exists, the notification is dropped.
    pub(crate) fn notify(&self, notification: Box<RawValue>) {
        let mut shares = self.shares.values().filter(|share| share.tx.receiver_count() > 0);
        let Some(mut current) = shares.next() else { return };
        for next in shares {
            let _ = current.tx.send(notification.clone());
            current = next;
        }
        let _ = current.tx.send(notification);
    }
}

/// The share of a subscription held by a caller that was handed a local id for it.
///
/// Each share has its own channel, so that unsubscribing one local id closes only the receivers
/// fetched with it.
pub(crate) struct Share {
    /// The channel via which notifications are broadcast to the receivers of this share.
    tx: broadcast::Sender<Box<RawValue>>,
    /// The initial receiver, until the caller fetches it.
    ///
    /// This is stored so that we don't drop any notifications between handing out the local id
    /// and the caller fetching its receiver. Ref: <https://github.com/alloy-rs/alloy/issues/2187>
    rx: Option<broadcast::Receiver<Box<RawValue>>>,
}

impl Share {
    /// Create a new share whose channel buffers `channel_size` notifications.
    pub(crate) fn new(channel_size: usize) -> Self {
        let (tx, rx) = broadcast::channel(channel_size);
        Self { tx, rx: Some(rx) }
    }

    /// Returns `true` if the caller holds a receiver, or has yet to fetch one.
    pub(crate) fn is_live(&self) -> bool {
        self.rx.is_some() || self.tx.receiver_count() > 0
    }

    /// Get a receiver.
    pub(crate) fn subscribe(&mut self) -> broadcast::Receiver<Box<RawValue>> {
        // The first receiver is the initial one, so that notifications received before it was
        // requested are not dropped. Later receivers start at the tail: the initial receiver may
        // still hold notifications that other receivers already consumed.
        // Ref: <https://github.com/alloy-rs/alloy/issues/2187>
        self.rx.take().unwrap_or_else(|| self.tx.subscribe())
    }
}
