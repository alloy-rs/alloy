use crate::{
    pubsub::{handle::ConnectionHandle, service::PubSubService, PubSubFrontend},
    Pbf, TransportConnect, TransportError,
};

/// Configuration objects that contain connection details for a backend.
///
/// Implementers should contain configuration options for the underlying
/// transport.
pub trait PubSubConnect: Sized + Send + Sync + 'static {
    /// Returns `true`` if the transport connects to a local resource.
    fn is_local(&self) -> bool;

    /// Spawn the backend, returning a handle to it.
    ///
    /// This function MUST create a long-lived task containing a
    /// [`ConnectionInterface`], and return the corresponding handle.
    ///
    /// [`ConnectionInterface`]: crate::pubsub::ConnectionInterface
    fn connect<'a: 'b, 'b>(&'a self) -> Pbf<'b, ConnectionHandle, TransportError>;

    /// Attempt to reconnect the transport.
    ///
    /// Override this to add custom reconnection logic to your connector. This
    /// will be used by [`PubSubService`] connection managers in the event the
    /// connection fails.
    ///
    /// [`PubSubService`]: crate::pubsub::service::PubSubService
    fn try_reconnect<'a: 'b, 'b>(&'a self) -> Pbf<'b, ConnectionHandle, TransportError> {
        self.connect()
    }

    /// Convert the configuration object into a service with a running backend.
    fn into_service(self) -> Pbf<'static, PubSubFrontend, TransportError> {
        Box::pin(PubSubService::connect(self))
    }
}

impl<T> TransportConnect for T
where
    T: PubSubConnect + Clone,
{
    type Transport = PubSubFrontend;

    fn is_local(&self) -> bool {
        PubSubConnect::is_local(self)
    }

    fn get_transport<'a: 'b, 'b>(&self) -> Pbf<'b, Self::Transport, TransportError> {
        let this = self.clone();
        Box::pin(async move { this.into_service().await })
    }
}
