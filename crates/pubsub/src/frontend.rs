use crate::{ix::PubSubInstruction, managers::InFlight, RawSubscription};
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, SerializedRequest};
use alloy_primitives::B256;
use alloy_transport::{TransportError, TransportErrorKind, TransportFut, TransportResult};
use futures::{FutureExt, TryFutureExt};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, debug_span, Instrument};

/// A `PubSubFrontend` is [`Transport`] composed of a channel to a running
/// PubSub service.
///
/// [`Transport`]: alloy_transport::Transport
#[derive(Debug, Clone)]
pub struct PubSubFrontend {
    tx: mpsc::UnboundedSender<PubSubInstruction>,
    /// The number of items to buffer in new subscription channels. Defaults to
    /// 16. See [`tokio::sync::broadcast::channel`] for a description.
    channel_size: Arc<AtomicUsize>,
}

impl PubSubFrontend {
    /// Create a new frontend.
    pub fn new(tx: mpsc::UnboundedSender<PubSubInstruction>) -> Self {
        Self { tx, channel_size: Arc::new(AtomicUsize::new(16)) }
    }

    /// Get the subscription ID for a local ID.
    pub fn get_subscription(
        &self,
        id: B256,
    ) -> impl Future<Output = TransportResult<RawSubscription>> + Send + 'static {
        let backend_tx = self.tx.clone();
        async move {
            let (tx, rx) = oneshot::channel();
            backend_tx
                .send(PubSubInstruction::GetSub(id, tx))
                .map_err(|_| TransportErrorKind::backend_gone())?;
            rx.await
                .map_err(|_| TransportErrorKind::backend_gone())?
                .map_or_else(|| Err(TransportErrorKind::custom_str("subscription not found")), Ok)
        }
    }

    /// Queue an unsubscribe instruction for a local subscription ID.
    ///
    /// A successful return means the instruction was sent to the pubsub
    /// service. It does not wait for the server's `eth_unsubscribe` response.
    /// Dropping a [`RawSubscription`] does not call this automatically. This is
    /// best effort: during reconnection it can race with a replacement
    /// subscription response, so success does not confirm server-side teardown.
    ///
    /// [`RawSubscription`]: crate::RawSubscription
    pub fn unsubscribe(&self, id: B256) -> TransportResult<()> {
        self.tx
            .send(PubSubInstruction::Unsubscribe(id))
            .map_err(|_| TransportErrorKind::backend_gone())
    }

    /// Send a request.
    pub fn send(
        &self,
        req: SerializedRequest,
    ) -> impl Future<Output = TransportResult<Response>> + Send + 'static {
        let tx = self.tx.clone();
        let channel_size = self.channel_size.load(Ordering::Relaxed);
        let method_name = req.method_clone();

        async move {
            debug!("sending request to backend");
            let (in_flight, rx) = InFlight::new(req, channel_size);
            tx.send(PubSubInstruction::Request(in_flight))
                .map_err(|_| TransportErrorKind::backend_gone())?;
            let resp = rx.await.map_err(|_| TransportErrorKind::backend_gone())?;
            if tracing::enabled!(tracing::Level::TRACE) {
                trace!(?resp, "retrieved response");
            } else {
                debug!(resp=?resp.as_ref().map(|_| ()), "retrieved response");
            };
            resp
        }
        .instrument(debug_span!("request", %method_name))
    }

    /// Send a batch of requests as a single JSON-RPC batch.
    ///
    /// The requests are serialized into a single JSON array and sent over the
    /// backend in one message. Responses are returned in the same order as the
    /// requests.
    pub fn send_batch(
        &self,
        reqs: Vec<SerializedRequest>,
    ) -> impl Future<Output = TransportResult<Vec<Response>>> + Send + 'static {
        let tx = self.tx.clone();
        let channel_size = self.channel_size.load(Ordering::Relaxed);

        async move {
            if reqs.is_empty() {
                return Ok(vec![]);
            }

            debug!(len = reqs.len(), "sending batch request to backend");
            let (in_flights, rxs): (Vec<_>, Vec<_>) =
                reqs.into_iter().map(|req| InFlight::new(req, channel_size)).unzip();
            tx.send(PubSubInstruction::BatchRequest(in_flights))
                .map_err(|_| TransportErrorKind::backend_gone())?;

            let mut responses = Vec::with_capacity(rxs.len());
            for rx in rxs {
                responses.push(rx.await.map_err(|_| TransportErrorKind::backend_gone())??);
            }
            Ok(responses)
        }
        .instrument(debug_span!("batch_request"))
    }

    /// Send a packet of requests.
    ///
    /// A single request is sent as-is. A batch is serialized into a single
    /// JSON-RPC batch message, preserving batch semantics on the wire. Once
    /// all responses are received, we return a single response packet.
    pub fn send_packet(&self, req: RequestPacket) -> TransportFut<'static> {
        match req {
            RequestPacket::Single(req) => self.send(req).map_ok(ResponsePacket::Single).boxed(),
            RequestPacket::Batch(reqs) => {
                self.send_batch(reqs).map_ok(ResponsePacket::Batch).boxed()
            }
        }
    }

    /// Get the currently configured channel size. This is the number of items
    /// to buffer in new subscription channels. Defaults to 16. See
    /// [`tokio::sync::broadcast`] for a description of relevant
    /// behavior.
    pub fn channel_size(&self) -> usize {
        self.channel_size.load(Ordering::Relaxed)
    }

    /// Set the channel size. This is the number of items to buffer in new
    /// subscription channels. Defaults to 16. See
    /// [`tokio::sync::broadcast`] for a description of relevant
    /// behavior.
    pub fn set_channel_size(&self, channel_size: usize) {
        debug_assert_ne!(channel_size, 0, "channel size must be non-zero");
        self.channel_size.store(channel_size, Ordering::Relaxed);
    }
}

impl tower::Service<RequestPacket> for PubSubFrontend {
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
