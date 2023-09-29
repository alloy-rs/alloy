use alloy_json_rpc::Request;
use serde_json::value::RawValue;
use tokio::sync::broadcast;

#[derive(Clone)]
/// An active subscription.
pub struct ActiveSubscription {
    /// The serialized params for the subscription request
    pub request: Request<Box<RawValue>>,
    /// The channel via which notifications are broadcast
    pub tx: broadcast::Sender<Box<RawValue>>,
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
    pub fn new(request: Request<Box<RawValue>>) -> (Self, broadcast::Receiver<Box<RawValue>>) {
        let (tx, rx) = broadcast::channel(16);

        (Self { request, tx }, rx)
    }

    /// Serialize the request as a boxed [`RawValue`].
    ///
    /// This is used to (re-)send the request over the transport.
    pub fn req_json(&self) -> serde_json::Result<Box<RawValue>> {
        serde_json::to_string(&self.request).and_then(RawValue::from_string)
    }

    /// Notify the subscription channel of a new value, if any receiver exists.
    /// If no receiver exists, the notification is dropped.
    pub fn notify(&mut self, notification: Box<RawValue>) {
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(notification);
        }
    }
}
