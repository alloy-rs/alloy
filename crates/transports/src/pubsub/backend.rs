use std::{future::Future, pin::Pin};

use alloy_json_rpc::{EthNotification, Response};
use tokio::sync::{mpsc, oneshot};

use super::SubscriptionManager;

pub enum PubSubItem {
    Response { json_rpc_response: Response },
    Notification { params: EthNotification },
}

/// A handle to a backend.
///
/// The backend SHOULD shut down when the handle is dropped (as indicated by
/// the shutdown channel).
pub struct ConnectionHandle {
    /// Inbound channel from server.
    from_server: mpsc::UnboundedReceiver<PubSubItem>,

    /// Outbound channel to server.
    to_server: mpsc::UnboundedSender<Box<serde_json::value::RawValue>>,

    /// Notification from the backend of a terminal error.
    error: oneshot::Receiver<()>,

    /// Notify the backend of intentional shutdown.
    shutdown: oneshot::Sender<()>,
}

pub trait PubSubConnect: Sized + 'static {
    type Error;

    /// Spawn the backend, returning a handle to it.
    fn connect(&self) -> Pin<Box<dyn Future<Output = Result<ConnectionHandle, Self::Error>>>>;

    fn into_service(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<PubSubService<Self>, Self::Error>>>> {
        Box::pin(async move {
            let handle = self.connect().await?;
            Ok(PubSubService {
                inner: handle,
                connector: self,
                subs: SubscriptionManager::default(),
            })
        })
    }
}

pub struct PubSubService<T> {
    inner: ConnectionHandle,

    connector: T,

    subs: SubscriptionManager,
}

impl<T> PubSubService<T>
where
    T: PubSubConnect,
{
    pub async fn reconnect(&mut self) -> Result<(), T::Error> {
        let handle = self.connector.connect().await?;
        self.inner = handle;
        Ok(())
    }
}
