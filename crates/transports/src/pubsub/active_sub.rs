use serde_json::value::RawValue;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
/// An active subscription.
pub struct ActiveSubscription {
    /// The serialized params for the subscription request
    params: Box<RawValue>,
    /// The channel via which notifications are broadcast
    pub channel: broadcast::Sender<Box<RawValue>>,
}

impl ActiveSubscription {
    /// Get the params
    pub fn params(&self) -> &RawValue {
        &self.params
    }

    /// Notify the subscription channel of a new value.
    pub fn notify(
        &mut self,
        notification: Box<RawValue>,
    ) -> Result<usize, broadcast::error::SendError<Box<RawValue>>> {
        self.channel.send(notification)
    }
}
