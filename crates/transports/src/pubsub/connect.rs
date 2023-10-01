use std::{future::Future, pin::Pin};

use tokio::sync::mpsc;

use crate::pubsub::{handle::ConnectionHandle, service::PubSubService, PubSubFrontend};

/// Configuration objects that contain connection details for a backend.
///
/// Implementers should contain configuration options for the underlying
/// transport.
pub trait PubSubConnect: Sized + Send + Sync + 'static {
    /// Returned by the `connect` and `into_service` methods if connection
    /// fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Spawn the backend, returning a handle to it.
    ///
    /// This function MUST create a long-lived task containing a
    /// [`ConnectionInterface`], and return the corresponding handle.
    ///
    /// [`ConnectionInterface`]: crate::pubsub::ConnectionInterface
    fn connect<'a: 'b, 'b>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<ConnectionHandle, Self::Error>> + Send + 'b>>;

    /// Convert the configuration object into a service with a running backend.
    fn into_service(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<PubSubFrontend, Self::Error>> + Send>> {
        Box::pin(async move {
            let handle = self.connect().await?;
            let (tx, reqs) = mpsc::unbounded_channel();

            let service_handle = PubSubFrontend::new(tx);
            let service = PubSubService {
                handle,
                connector: self,
                reqs,
                subs: Default::default(),
                in_flights: Default::default(),
            };

            service.spawn();

            Ok(service_handle)
        })
    }
}
