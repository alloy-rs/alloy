use crate::{SubscriptionOptions, SubscriptionReceiverTicket, SubscriptionRetentionPolicy};
use alloy_json_rpc::{Response, ResponsePayload, SerializedRequest, SubId};
use alloy_transport::{TransportError, TransportResult};
use std::{borrow::Cow, fmt};
use tokio::sync::oneshot;

/// An in-flight JSON-RPC request.
///
/// This struct contains the request that was sent, as well as a channel to
/// receive the response on.
pub struct InFlight {
    /// The request
    pub request: SerializedRequest,

    /// The number of items to buffer in the subscription channel.
    pub channel_size: usize,

    /// The method used to remove the server-side subscription.
    pub(crate) unsubscribe_method: Option<Cow<'static, str>>,

    /// One-shot receiver delivery for the typed provider path.
    pub(crate) receiver_ticket: Option<SubscriptionReceiverTicket>,

    /// The retention requested by this waiter.
    pub(crate) retention_policy: SubscriptionRetentionPolicy,

    /// The channel to send the response on.
    pub tx: oneshot::Sender<TransportResult<Response>>,
}

impl fmt::Debug for InFlight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InFlight")
            .field("request", &self.request)
            .field("channel_size", &self.channel_size)
            .field("unsubscribe_method", &self.unsubscribe_method)
            .field("typed", &self.receiver_ticket.is_some())
            .field("retention_policy", &self.retention_policy)
            .field("tx_is_closed", &self.tx.is_closed())
            .finish()
    }
}

impl InFlight {
    /// Create a new in-flight request.
    pub fn new(
        mut request: SerializedRequest,
        default_channel_size: usize,
    ) -> (Self, oneshot::Receiver<TransportResult<Response>>) {
        let (tx, rx) = oneshot::channel();
        let receiver_ticket =
            request.meta_mut().extensions_mut().remove::<SubscriptionReceiverTicket>();
        let options = request.meta().extensions().get::<SubscriptionOptions>();
        let channel_size =
            options.and_then(SubscriptionOptions::channel_size).unwrap_or(default_channel_size);
        let unsubscribe_method = options.and_then(SubscriptionOptions::unsubscribe_method_owned);
        let retention_policy = options.and_then(SubscriptionOptions::retention_policy).unwrap_or(
            if receiver_ticket.is_some() {
                SubscriptionRetentionPolicy::WhileReceivers
            } else {
                SubscriptionRetentionPolicy::UntilExplicitUnsubscribe
            },
        );

        (
            Self {
                request,
                channel_size,
                unsubscribe_method,
                receiver_ticket,
                retention_policy,
                tx,
            },
            rx,
        )
    }

    /// Check if the request is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.request.is_subscription()
    }

    /// Returns whether the caller can still receive both subscription ownership and its response.
    pub(crate) fn is_live_subscription_waiter(&self) -> bool {
        !self.tx.is_closed()
            && self.receiver_ticket.as_ref().is_none_or(|ticket| !ticket.is_closed())
    }

    /// Get a reference to the serialized request.
    ///
    /// This is used to (re-)send the request over the transport.
    pub const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Fulfill the request with a response. This consumes the in-flight
    /// request. If the request is a subscription and the response is not an
    /// error, the subscription ID and the in-flight request are returned.
    pub fn fulfill(self, resp: Response) -> Option<(SubId, Self)> {
        if self.is_subscription() {
            if let ResponsePayload::Success(val) = resp.payload {
                let sub_id: serde_json::Result<SubId> = serde_json::from_str(val.get());
                return match sub_id {
                    Ok(alias) => Some((alias, self)),
                    Err(e) => {
                        let _ = self.tx.send(Err(TransportError::deser_err(e, val.get())));
                        None
                    }
                };
            }
        }

        let _ = self.tx.send(Ok(resp));
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request};

    #[test]
    fn typed_and_manual_requests_have_distinct_retention_defaults() {
        let raw = Request::new("eth_subscribe", Id::Number(1), ("newHeads",)).serialize().unwrap();
        let (raw, _rx) = InFlight::new(raw, 16);
        assert_eq!(raw.retention_policy, SubscriptionRetentionPolicy::UntilExplicitUnsubscribe);

        let mut typed =
            Request::new("eth_subscribe", Id::Number(2), ("newHeads",)).serialize().unwrap();
        let (ticket, _subscription_rx) = SubscriptionReceiverTicket::channel();
        typed.meta_mut().extensions_mut().insert(ticket);
        let (typed, _rx) = InFlight::new(typed, 16);
        assert_eq!(typed.retention_policy, SubscriptionRetentionPolicy::WhileReceivers);
        assert!(typed.request.meta().extensions().get::<SubscriptionReceiverTicket>().is_none());
    }
}
