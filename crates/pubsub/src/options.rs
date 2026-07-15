use std::borrow::Cow;

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
