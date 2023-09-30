use std::{future::Future, pin::Pin};

use alloy_json_rpc::{Request, ResponsePayload, RpcParam};
use tokio::sync::mpsc;

use crate::{
    pubsub::{
        handle::ConnectionHandle, managers::InFlight, service::PubSubInstruction,
        service::PubSubService,
    },
    TransportError,
};

#[derive(Debug, Clone)]
pub struct ServiceFrontend {
    pub tx: mpsc::UnboundedSender<PubSubInstruction>,
}

impl ServiceFrontend {
    /// Send a request.
    pub fn send<T>(
        &self,
        req: Request<T>,
    ) -> Pin<Box<dyn Future<Output = Result<ResponsePayload, TransportError>> + Send>>
    where
        T: RpcParam,
    {
        let (in_flight, rx) = InFlight::new(req.box_params());
        let ix = PubSubInstruction::Request(in_flight);
        let tx = self.tx.clone();

        Box::pin(async move {
            tx.send(ix).map_err(|_| TransportError::BackendGone)?;
            rx.await.map_err(|_| TransportError::BackendGone)?
        })
    }
}

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
    ) -> Pin<Box<dyn Future<Output = Result<ServiceFrontend, Self::Error>> + Send>> {
        Box::pin(async move {
            let handle = self.connect().await?;
            let (tx, reqs) = mpsc::unbounded_channel();

            let service_handle = ServiceFrontend { tx };
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
