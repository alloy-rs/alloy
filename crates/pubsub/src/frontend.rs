use crate::{ix::PubSubInstruction, managers::InFlight};
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, SerializedRequest};
use alloy_primitives::U256;
use alloy_transport::{TransportError, TransportErrorKind, TransportFut};
use futures::{future::try_join_all, FutureExt, TryFutureExt};
use serde_json::value::RawValue;
use std::{
    future::Future,
    task::{Context, Poll},
};
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
    pub fn get_subscription(
        &self,
        id: U256,
    ) -> impl Future<Output = Result<broadcast::Receiver<Box<RawValue>>, TransportError>> + Send + 'static
    {
        let backend_tx = self.tx.clone();
        async move {
            let (tx, rx) = oneshot::channel();
            backend_tx
                .send(PubSubInstruction::GetSub(id, tx))
                .map_err(|_| TransportErrorKind::backend_gone())?;
            rx.await.map_err(|_| TransportErrorKind::backend_gone())
        }
    }

    /// Unsubscribe from a subscription.
    pub fn unsubscribe(&self, id: U256) -> Result<(), TransportError> {
        self.tx
            .send(PubSubInstruction::Unsubscribe(id))
            .map_err(|_| TransportErrorKind::backend_gone())
    }

    /// Send a request.
    pub fn send(
        &self,
        req: SerializedRequest,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send + 'static {
        let tx = self.tx.clone();
        async move {
            let (in_flight, rx) = InFlight::new(req);
            tx.send(PubSubInstruction::Request(in_flight))
                .map_err(|_| TransportErrorKind::backend_gone())?;
            rx.await.map_err(|_| TransportErrorKind::backend_gone())?
        }
    }

    /// Send a packet of requests, by breaking it up into individual requests.
    ///
    /// Once all responses are received, we return a single response packet.
    pub fn send_packet(&self, req: RequestPacket) -> TransportFut<'static> {
        match req {
            RequestPacket::Single(req) => self.send(req).map_ok(ResponsePacket::Single).boxed(),
            RequestPacket::Batch(reqs) => try_join_all(reqs.into_iter().map(|req| self.send(req)))
                .map_ok(ResponsePacket::Batch)
                .boxed(),
        }
    }
}

impl tower::Service<RequestPacket> for PubSubFrontend {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        (&*self).poll_ready(cx)
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        (&*self).call(req)
    }
}

impl tower::Service<RequestPacket> for &PubSubFrontend {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let result =
            if self.tx.is_closed() { Err(TransportErrorKind::backend_gone()) } else { Ok(()) };
        Poll::Ready(result)
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.send_packet(req)
    }
}
