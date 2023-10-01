use std::{future::Future, pin::Pin};

use alloy_json_rpc::{Request, ResponsePayload, RpcParam};
use alloy_primitives::U256;
use serde_json::value::RawValue;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    pubsub::{ix::PubSubInstruction, managers::InFlight},
    TransportError,
};

#[derive(Debug, Clone)]
pub struct PubSubFrontend {
    tx: mpsc::UnboundedSender<PubSubInstruction>,
}

impl PubSubFrontend {
    /// Create a new frontend.
    pub(crate) fn new(tx: mpsc::UnboundedSender<PubSubInstruction>) -> Self {
        Self { tx }
    }

    /// Get the subscription ID for a local ID.
    pub async fn get_subscription(
        &self,
        id: U256,
    ) -> Result<broadcast::Receiver<Box<RawValue>>, TransportError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(PubSubInstruction::GetSub(id, tx))
            .map_err(|_| TransportError::BackendGone)?;
        rx.await.map_err(|_| TransportError::BackendGone)
    }

    /// Unsubscribe from a subscription.
    pub async fn unsubscribe(&self, id: U256) -> Result<(), TransportError> {
        self.tx
            .send(PubSubInstruction::Unsubscribe(id))
            .map_err(|_| TransportError::BackendGone)?;
        Ok(())
    }

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
