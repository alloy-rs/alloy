use alloy_json_rpc::PubSubItem;
use tokio::task::JoinHandle;

use crate::{
    pubsub::{
        managers::{RequestManager, SubscriptionManager},
        ConnectionHandle, InFlight, PubSubConnect,
    },
    TransportError,
};

#[derive(Debug)]
/// The service contains the backend handle, a subscription manager, and the
/// configuration details required to reconnect.
pub struct PubSubService<T> {
    /// The backend handle.
    pub(crate) handle: ConnectionHandle,

    /// The configuration details required to reconnect.
    pub(crate) connector: T,

    /// The inbound requests.
    pub(crate) reqs: tokio::sync::mpsc::UnboundedReceiver<InFlight>,

    /// The subscription manager.
    pub(crate) subs: SubscriptionManager,

    /// The request manager.
    pub(crate) in_flights: RequestManager,
}

impl<T> PubSubService<T>
where
    T: PubSubConnect,
{
    /// Reconnect by dropping the backend and creating a new one.
    pub async fn reconnect(&mut self) -> Result<(), T::Error> {
        let handle = self.connector.connect().await?;
        self.handle = handle;
        Ok(())
    }

    /// Service a request.
    async fn service_request(&mut self, in_flight: InFlight) -> Result<(), TransportError> {
        todo!()
    }

    async fn handle_item(&mut self, item: PubSubItem) -> Result<(), TransportError> {
        todo!()
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let result: Result<(), TransportError> = loop {
                // We bias the loop so that we always handle new messages before
                // reconnecting, and always reconnect before dispatching new
                // requests.
                tokio::select! {
                    biased;

                    item_opt = self.handle.from_socket.recv() => {
                        if let Some(item) = item_opt {
                            if let Err(e) = self.handle_item(item).await {
                                break Err(e)
                            }
                        } else {
                            if let Err(e) = self.reconnect().await {
                                break Err(TransportError::Custom(Box::new(e)))
                            }
                        }
                    }

                    _ = &mut self.handle.error => {
                        if let Err(e) = self.reconnect().await {
                            break Err(TransportError::Custom(Box::new(e)))
                        }
                    }

                    req_opt = self.reqs.recv() => {
                        if let Some(req) = req_opt {
                            if let Err(e) = self.service_request(req).await {
                                break Err(e)
                            }
                        } else {
                            tracing::info!("Pubsub service request channel closed. Shutting down.");
                           break Ok(())
                        }
                    }
                }
            };

            if let Err(err) = result {
                tracing::error!(%err, "pubsub service reconnection error");
            }
        })
    }
}
