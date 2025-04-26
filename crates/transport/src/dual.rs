use crate::{TransportError, TransportFut};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use tower::Service;

/// Trait for converting two transports into a dual transport.
pub trait DualTransportHandler<L, R> {
    /// The type of the future returned by the transport.
    fn call(&self, request: RequestPacket, left: L, right: R) -> TransportFut<'static>;
}

/// A transport that dispatches requests to one of two inner transports based on a handler.
///
/// This type allows RPC clients to dynamically select between two different transports
/// at runtime depending on the request. It is [Send] + [`Sync` ] and implements Transport
/// via the [`Service`] trait.
///
/// This is useful for building clients that abstract over multiple backends or protocols,
/// routing requests flexibly without having to commit to a single transport implementation.
///
/// All higher-level types can use  [`DualTransport`] internally to support multiple transport
/// strategies.

#[derive(Debug)]
pub struct DualTransport<L, R, H> {
    /// The left transport.
    left: L,
    /// The right transport.
    right: R,
    /// The handler that decides which transport to use.
    handler: H,
}

impl<L, R, H> DualTransport<L, R, H> {
    /// Instantiate a new dual transport from a suitable transport.
    pub fn new(left: L, right: R, handler: H) -> Self {
        Self { left, right, handler }
    }
}

impl<L, R, H> Service<RequestPacket> for DualTransport<L, R, H>
where
    L: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Sync
        + Clone
        + 'static,
    R: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Sync
        + Clone
        + 'static,
    H: DualTransportHandler<L, R> + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        match (self.left.poll_ready(cx), self.right.poll_ready(cx)) {
            (std::task::Poll::Ready(Ok(())), std::task::Poll::Ready(Ok(()))) => {
                std::task::Poll::Ready(Ok(()))
            }
            (std::task::Poll::Ready(Err(e)), _) => std::task::Poll::Ready(Err(e)),
            (_, std::task::Poll::Ready(Err(e))) => std::task::Poll::Ready(Err(e)),
            _ => std::task::Poll::Pending,
        }
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.handler.call(req, self.left.clone(), self.right.clone())
    }
}
