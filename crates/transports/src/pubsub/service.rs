use alloy_json_rpc::{PubSubItem, Request, ResponsePayload};
use alloy_primitives::U256;
use tokio::task::JoinHandle;

use crate::{
    pubsub::{
        managers::{RequestManager, SubscriptionManager},
        ConnectionHandle, InFlight, PubSubConnect,
    },
    utils::to_json_raw_value,
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
        let brv = in_flight.req_json().map_err(TransportError::ser_err)?;

        self.in_flights.insert(in_flight);
        if self.handle.to_socket.send(brv).is_err() {
            self.reconnect().await.map_err(TransportError::custom)?;
        }

        Ok(())
    }

    /// Handle an item from the backend.
    async fn handle_item(&mut self, item: PubSubItem) -> Result<(), TransportError> {
        match item {
            PubSubItem::Response(resp) => match self.in_flights.handle_response(resp) {
                Some((server_id, in_flight)) => self.handle_sub_response(in_flight, server_id),
                None => Ok(()),
            },
            PubSubItem::Notification(notification) => {
                let server_id = notification.subscription;
                // disconnect on err
                if self.subs.forward_notification(notification).is_err() {
                    self.unsubscribe(server_id)?;
                }
                Ok(())
            }
        }
    }

    /// Rewrite the subscription id and insert into the subscriptions manager
    fn handle_sub_response(
        &mut self,
        in_flight: InFlight,
        server_id: alloy_primitives::Uint<256, 4>,
    ) -> Result<(), TransportError> {
        let request = in_flight.request;

        self.subs.insert(request, server_id);
        let alias = self.subs.alias(server_id).unwrap();

        // lie to the client about the sub id
        let ser_alias = to_json_raw_value(&alias)?;
        let _ = in_flight.tx.send(Ok(ResponsePayload::Success(ser_alias)));

        Ok(())
    }

    /// Unsubscribe from a subscription.
    fn unsubscribe(&mut self, server_id: U256) -> Result<(), TransportError> {
        let req = Request {
            method: "eth_unsubscribe",
            params: to_json_raw_value(&server_id)?,
            id: alloy_json_rpc::Id::None,
        };
        let (in_flight, _) = InFlight::new(req);
        self.service_request(in_flight);
        Ok(())
    }

    /// Spawn the service.
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
                        } else if let Err(e) = self.reconnect().await {
                            break Err(TransportError::Custom(Box::new(e)))
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
