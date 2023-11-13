use crate::{ix::PubSubInstruction, managers::InFlight};
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, SerializedRequest};
use alloy_primitives::U256;
use alloy_transport::{TransportError, TransportFut};
use futures::future::try_join_all;
use serde_json::value::RawValue;
use std::{future::Future, pin::Pin};
use tokio::sync::{broadcast, mpsc, oneshot};

/// A `PubSubFrontend` is [`Transport`] composed of a channel to a running
/// PubSub service.
///
/// [`Transport`]: alloy_transport::Transport
#[derive(Debug, Clone)]
pub struct PubSubFrontend {
    tx: mpsc::UnboundedSender<PubSubInstruction>,
}

impl PubSubFrontend {
    /// Create a new frontend.
    pub(crate) const fn new(tx: mpsc::UnboundedSender<PubSubInstruction>) -> Self {
        Self { tx }
    }

    /// Get the subscription ID for a local ID.
    pub async fn get_subscription(
        &self,
        id: U256,
    ) -> Result<broadcast::Receiver<Box<RawValue>>, TransportError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(PubSubInstruction::GetSub(id, tx)).map_err(|_| TransportError::BackendGone)?;
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
    pub fn send(
        &self,
        req: SerializedRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Response, TransportError>> + Send>> {
        let (in_flight, rx) = InFlight::new(req);
        let ix = PubSubInstruction::Request(in_flight);
        let tx = self.tx.clone();

        Box::pin(async move {
            tx.send(ix).map_err(|_| TransportError::BackendGone)?;
            rx.await.map_err(|_| TransportError::BackendGone)?
        })
    }

    /// Send a packet of requests, by breaking it up into individual requests.
    ///
    /// Once all responses are received, we return a single response packet.
    /// This is a bit annoying
    pub fn send_packet(
        &self,
        req: RequestPacket,
    ) -> Pin<Box<dyn Future<Output = Result<ResponsePacket, TransportError>> + Send>> {
        match req {
            RequestPacket::Single(req) => {
                let fut = self.send(req);
                Box::pin(async move { Ok(ResponsePacket::Single(fut.await?)) })
            }
            RequestPacket::Batch(reqs) => {
                let futs = try_join_all(reqs.into_iter().map(|req| self.send(req)));
                Box::pin(async move { Ok(futs.await?.into()) })
            }
        }
    }
}

impl tower::Service<RequestPacket> for PubSubFrontend {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        if self.tx.is_closed() {
            return std::task::Poll::Ready(Err(TransportError::BackendGone));
        }
        std::task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.send_packet(req)
    }
}

impl tower::Service<RequestPacket> for &PubSubFrontend {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        if self.tx.is_closed() {
            return std::task::Poll::Ready(Err(TransportError::BackendGone));
        }
        std::task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.send_packet(req)
    }
}
