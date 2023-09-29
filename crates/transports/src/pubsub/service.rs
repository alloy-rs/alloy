use std::{future::Future, pin::Pin};

use alloy_json_rpc::PubSubItem;
use serde_json::value::RawValue;
use tokio::sync::{mpsc, oneshot};

use super::SubscriptionManager;

#[derive(Debug)]
/// A handle to a backend.
///
/// The backend SHOULD shut down when the handle is dropped (as indicated by
/// the shutdown channel).
pub struct ConnectionHandle {
    /// Outbound channel to server.
    to_socket: mpsc::UnboundedSender<Box<RawValue>>,

    /// Inbound channel from remote server via WS.
    from_socket: mpsc::UnboundedReceiver<PubSubItem>,

    /// Notification from the backend of a terminal error.
    error: oneshot::Receiver<()>,

    /// Notify the backend of intentional shutdown.
    shutdown: oneshot::Sender<()>,
}

impl ConnectionHandle {
    pub fn new(
        to_socket: mpsc::UnboundedSender<Box<RawValue>>,
        from_socket: mpsc::UnboundedReceiver<PubSubItem>,
        error: oneshot::Receiver<()>,
        shutdown: oneshot::Sender<()>,
    ) -> Self {
        Self {
            to_socket,
            from_socket,
            error,
            shutdown,
        }
    }
}

/// Configuration objects that contain connection details for a backend.
///
/// Implementers should contain configuration options for the underlying
/// transport.
pub trait PubSubConnect: Sized + Send + 'static {
    type Error;

    /// Spawn the backend, returning a handle to it.
    fn connect<'a: 'b, 'b>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<ConnectionHandle, Self::Error>> + 'b>>;

    fn into_service(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<PubSubService<Self>, Self::Error>>>> {
        Box::pin(async move {
            let handle = self.connect().await?;
            Ok(PubSubService {
                handle,
                connector: self,
                subs: SubscriptionManager::default(),
            })
        })
    }
}

#[derive(Debug)]
/// The service contains the backend handle, a subscription manager, and the
/// configuration details required to reconnect.
pub struct PubSubService<T> {
    /// The backend handle.
    handle: ConnectionHandle,

    /// The configuration details required to reconnect.
    connector: T,

    /// The subscription manager.
    subs: SubscriptionManager,
}

impl<T> PubSubService<T>
where
    T: PubSubConnect,
{
    pub async fn reconnect(&mut self) -> Result<(), T::Error> {
        let handle = self.connector.connect().await?;
        self.handle = handle;
        Ok(())
    }
}
