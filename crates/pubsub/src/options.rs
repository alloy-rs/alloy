use crate::RawSubscription;
use parking_lot::Mutex;
use std::{borrow::Cow, fmt, sync::Arc};
use tokio::sync::oneshot;

/// Controls when a server-side subscription is eligible for automatic cleanup.
///
/// This enum defaults to [`Self::WhileReceivers`], matching typed provider builders.
/// Low-level/manual subscription requests apply [`Self::UntilExplicitUnsubscribe`] as a separate
/// compatibility default.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubscriptionRetentionPolicy {
    /// Keep the server-side subscription until it is explicitly unsubscribed.
    UntilExplicitUnsubscribe,
    /// Clean up the server-side subscription after its last local receiver is dropped.
    #[default]
    WhileReceivers,
}

/// A clone-safe, one-shot delivery ticket for a typed subscription receiver.
///
/// This is public only so the provider and RPC client crates can carry it through request
/// metadata. It is not part of the user-facing subscription API.
#[doc(hidden)]
#[derive(Clone)]
pub struct SubscriptionReceiverTicket(Arc<Mutex<Option<oneshot::Sender<RawSubscription>>>>);

impl fmt::Debug for SubscriptionReceiverTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubscriptionReceiverTicket").field("closed", &self.is_closed()).finish()
    }
}

impl SubscriptionReceiverTicket {
    /// Creates a ticket and its receiving half.
    #[doc(hidden)]
    pub fn channel() -> (Self, oneshot::Receiver<RawSubscription>) {
        let (tx, rx) = oneshot::channel();
        (Self(Arc::new(Mutex::new(Some(tx)))), rx)
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.0.lock().as_ref().is_none_or(oneshot::Sender::is_closed)
    }

    pub(crate) fn send(&self, subscription: RawSubscription) -> Result<(), RawSubscription> {
        let Some(tx) = self.0.lock().take() else { return Err(subscription) };
        tx.send(subscription)
    }
}

/// Per-request configuration for a pubsub subscription.
///
/// This value is carried in the JSON-RPC request extensions and consumed by the pubsub service.
/// It is never serialized onto the wire.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubscriptionOptions {
    /// The requested local broadcast channel capacity.
    channel_size: Option<usize>,
    /// The RPC method used to clean up the server-side subscription.
    unsubscribe_method: Option<Cow<'static, str>>,
    /// The requested retention behavior.
    retention_policy: Option<SubscriptionRetentionPolicy>,
}

impl SubscriptionOptions {
    /// Returns the requested local broadcast channel capacity.
    pub const fn channel_size(&self) -> Option<usize> {
        self.channel_size
    }

    /// Sets the requested local broadcast channel capacity.
    ///
    /// A zero capacity is invalid and is rejected before wire dispatch.
    pub const fn set_channel_size(&mut self, channel_size: usize) {
        self.channel_size = Some(channel_size);
    }

    /// Returns the configured server-side unsubscribe method.
    ///
    /// `None` means the protocol provides no known cleanup RPC and the subscription can only be
    /// reclaimed when its connection closes.
    pub fn unsubscribe_method(&self) -> Option<&str> {
        self.unsubscribe_method.as_deref()
    }

    /// Sets the server-side unsubscribe method.
    pub fn set_unsubscribe_method(&mut self, method: impl Into<Cow<'static, str>>) {
        self.unsubscribe_method = Some(method.into());
    }

    /// Returns the configured retention policy.
    pub const fn retention_policy(&self) -> Option<SubscriptionRetentionPolicy> {
        self.retention_policy
    }

    /// Sets the retention policy for this subscription request.
    pub const fn set_retention_policy(&mut self, policy: SubscriptionRetentionPolicy) {
        self.retention_policy = Some(policy);
    }

    pub(crate) fn unsubscribe_method_owned(&self) -> Option<Cow<'static, str>> {
        self.unsubscribe_method.clone()
    }
}

/// The terminal result of a tracked server-side unsubscribe request.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsubscribeOutcome {
    /// The server confirmed that the subscription was removed.
    ServerConfirmed,
    /// The server or local service reported that the subscription was already absent.
    AlreadyAbsent,
    /// The connection closed, which releases all subscriptions owned by that connection.
    TransportClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_policy_default_matches_typed_option_a() {
        assert_eq!(
            SubscriptionRetentionPolicy::default(),
            SubscriptionRetentionPolicy::WhileReceivers
        );
    }
}
