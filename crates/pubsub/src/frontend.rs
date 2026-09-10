use crate::{ix::PubSubInstruction, managers::InFlight, RawSubscription};
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, SerializedRequest};
use alloy_primitives::B256;
use alloy_transport::{TransportError, TransportErrorKind, TransportFut, TransportResult};
use futures::{future::try_join_all, FutureExt, TryFutureExt};
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

    /// Send a packet of JSON-RPC requests.
    ///
    /// Single requests continue to use the existing request path.
    ///
    /// Batch requests are kept grouped when they are sent to the pubsub
    /// service. Each request still gets its own [`InFlight`] entry and response
    /// receiver so responses can be routed independently by JSON-RPC ID.
    ///
    /// The pubsub service is responsible for serializing the batch into one
    /// JSON array and dispatching it as one backend message.
    pub fn send_packet(&self, req: RequestPacket) -> TransportFut<'static> {
        match req {
            RequestPacket::Single(req) => self.send(req).map_ok(ResponsePacket::Single).boxed(),

            RequestPacket::Batch(reqs) => {
                let tx = self.tx.clone();
                let channel_size = self.channel_size.load(Ordering::Relaxed);

                async move {
                    // Preserve the previous behavior for an empty batch:
                    // there is nothing to send to the backend and there are no
                    // responses to wait for.
                    if reqs.is_empty() {
                        return Ok(ResponsePacket::Batch(Vec::new()));
                    }

                    debug!(request_count = reqs.len(), "sending request batch to backend");

                    // We need two collections:
                    //
                    // 1. `in_flights` These are sent together to the pubsub service so the service
                    //    can serialize them as one JSON-RPC batch.
                    //
                    // 2. `receivers` Each request still has an independent oneshot receiver because
                    //    JSON-RPC responses are matched by request ID.
                    let mut in_flights = Vec::with_capacity(reqs.len());
                    let mut receivers = Vec::with_capacity(reqs.len());

                    for req in reqs {
                        let (in_flight, rx) = InFlight::new(req, channel_size);

                        in_flights.push(in_flight);
                        receivers.push(rx);
                    }

                    // IMPORTANT:
                    //
                    // Previously this function called `self.send(req)` for
                    // every element, producing N `Request` instructions.
                    //
                    // Sending one `Batch` instruction preserves the grouping
                    // until the pubsub service, where the requests can be
                    // serialized into one JSON array / one WebSocket message.
                    tx.send(PubSubInstruction::Batch(in_flights))
                        .map_err(|_| TransportErrorKind::backend_gone())?;

                    // The server is allowed to respond to the requests
                    // independently and even in a different order.
                    //
                    // RequestManager routes each response to the correct
                    // oneshot sender using its JSON-RPC ID. Here we only wait
                    // until all those individual responses have arrived.
                    let responses = try_join_all(receivers.into_iter().map(|rx| async move {
                        rx.await.map_err(|_| TransportErrorKind::backend_gone())?
                    }))
                    .await?;

                    // Keep the Transport API unchanged: callers of
                    // `send_packet()` still receive one ResponsePacket::Batch.
                    Ok(ResponsePacket::Batch(responses))
                }
                .boxed()
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
