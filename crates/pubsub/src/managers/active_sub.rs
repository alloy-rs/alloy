use crate::RawSubscription;
use alloy_json_rpc::SerializedRequest;
use alloy_primitives::B256;
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::VecDeque, fmt};
use tokio::sync::broadcast;

/// An active subscription.
pub(crate) struct ActiveSubscription {
    /// Subscription identity and public alias.
    pub(crate) local_id: B256,
    /// The serialized subscription request.
    pub(crate) request: SerializedRequest,
    /// The channel via which notifications are broadcast.
    pub(crate) tx: broadcast::Sender<Box<RawValue>>,
    /// The configured channel capacity.
    pub(crate) channel_size: usize,
    /// The server-side cleanup method.
    pub(crate) unsubscribe_method: Option<Cow<'static, str>>,
    /// Receivers awaiting a legacy two-phase `get_subscription` claim.
    manual_claims: VecDeque<RawSubscription>,
    /// Whether explicit unsubscribe is required even when no receiver remains.
    persistent_hold: bool,
}

impl fmt::Debug for ActiveSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveSubscription")
            .field("local_id", &self.local_id)
            .field("request", &self.request)
            .field("channel_size", &self.channel_size)
            .field("unsubscribe_method", &self.unsubscribe_method)
            .field("subscribers", &self.tx.receiver_count())
            .field("manual_claims", &self.manual_claims.len())
            .field("persistent_hold", &self.persistent_hold)
            .finish()
    }
}

impl ActiveSubscription {
    /// Create a new active subscription.
    pub(crate) fn new(
        local_id: B256,
        request: SerializedRequest,
        channel_size: usize,
        unsubscribe_method: Option<Cow<'static, str>>,
    ) -> (Self, RawSubscription) {
        let (tx, rx) = broadcast::channel(channel_size);
        let initial = RawSubscription { rx, local_id };
        (
            Self {
                request,
                local_id,
                tx,
                channel_size,
                unsubscribe_method,
                manual_claims: VecDeque::new(),
                persistent_hold: false,
            },
            initial,
        )
    }

    /// Serialize the request as a boxed [`RawValue`].
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Get a subscription.
    pub(crate) fn subscribe(&self) -> RawSubscription {
        RawSubscription { rx: self.tx.subscribe(), local_id: self.local_id }
    }

    pub(crate) fn push_manual_claim(&mut self, subscription: RawSubscription) {
        self.manual_claims.push_back(subscription);
    }

    pub(crate) fn pop_manual_claim(&mut self) -> Option<RawSubscription> {
        self.manual_claims.pop_front()
    }

    pub(crate) fn restore_manual_claim(&mut self, subscription: RawSubscription) {
        self.manual_claims.push_front(subscription);
    }

    pub(crate) const fn commit_persistent_hold(&mut self) {
        self.persistent_hold = true;
    }

    pub(crate) const fn has_persistent_hold(&self) -> bool {
        self.persistent_hold
    }

    pub(crate) fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    pub(crate) fn should_auto_cleanup(&self) -> bool {
        !self.has_persistent_hold() && self.receiver_count() == 0
    }

    #[cfg(test)]
    pub(crate) fn manual_claim_count(&self) -> usize {
        self.manual_claims.len()
    }

    /// Notify the subscription channel of a new value, if any receiver exists.
    /// If no receiver exists, the notification is dropped.
    pub(crate) fn notify(&self, notification: Box<RawValue>) {
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(notification);
        }
    }
}
