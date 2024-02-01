use crate::{handle::ConnectionHandle, service::PubSubService, PubSubFrontend};
use alloy_transport::{Pbf, TransportError};

/// Configuration objects that contain connection details for a backend.
///
/// Implementers should contain configuration options for the underlying
/// transport.
pub trait PubSubConnect: Sized + Send + Sync + 'static {
    /// Returns `true` if the transport connects to a local resource.
    fn is_local(&self) -> bool;

    /// Returns the buffer size for subscription channels. This value will
    /// be used to determine the size of the broadcast channels used to send
    /// notifications to subscribers. See [`broadcast::channel`] for more
    /// information about channel behavior.
    ///
    /// [`broadcast::channel`]: tokio::sync::broadcast::channel
    fn subscription_buffer_size(&self) -> usize {
        16
    }

    /// Spawn the backend, returning a handle to it.
    ///
    /// This function MUST create a long-lived task containing a
    /// [`ConnectionInterface`], and return the corresponding handle.
    ///
    /// [`ConnectionInterface`]: crate::ConnectionInterface
    fn connect<'a: 'b, 'b>(&'a self) -> Pbf<'b, ConnectionHandle, TransportError>;

    /// Attempt to reconnect the transport.
    ///
    /// Override this to add custom reconnection logic to your connector. This
    /// will be used by the internal pubsub connection managers in the event the
    /// connection fails.
    fn try_reconnect<'a: 'b, 'b>(&'a self) -> Pbf<'b, ConnectionHandle, TransportError> {
        self.connect()
    }

    /// Convert the configuration object into a service with a running backend.
    fn into_service(self) -> Pbf<'static, PubSubFrontend, TransportError> {
        Box::pin(PubSubService::connect(self))
    }
}
