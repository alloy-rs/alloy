use crate::SubscriptionOptions;
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

    /// The channel to send the response on.
    pub tx: oneshot::Sender<TransportResult<Response>>,
}

impl fmt::Debug for InFlight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InFlight")
            .field("request", &self.request)
            .field("channel_size", &self.channel_size)
            .field("unsubscribe_method", &self.unsubscribe_method)
            .field("tx_is_closed", &self.tx.is_closed())
            .finish()
    }
}

impl InFlight {
    /// Create a new in-flight request.
    pub fn new(
        request: SerializedRequest,
        default_channel_size: usize,
    ) -> (Self, oneshot::Receiver<TransportResult<Response>>) {
        let (tx, rx) = oneshot::channel();
        let options = request.meta().extensions().get::<SubscriptionOptions>();
        let channel_size =
            options.and_then(SubscriptionOptions::channel_size).unwrap_or(default_channel_size);
        let unsubscribe_method = options.and_then(SubscriptionOptions::unsubscribe_method_owned);

        (Self { request, channel_size, unsubscribe_method, tx }, rx)
    }

    /// Check if the request is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.request.is_subscription()
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
