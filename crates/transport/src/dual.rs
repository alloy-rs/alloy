use crate::{TransportError, TransportFut};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use tower::Service;

/// Trait that determines how to dispatch a request given two transports.
pub trait DualTransportHandler<L, R> {
    /// The type of the future returned by the transport.
    fn call(&self, request: RequestPacket, left: L, right: R) -> TransportFut<'static>;
}

impl<F, L, R> DualTransportHandler<L, R> for F
where
    F: Fn(RequestPacket, L, R) -> TransportFut<'static> + Send + Sync,
{
    fn call(&self, request: RequestPacket, left: L, right: R) -> TransportFut<'static> {
        (self)(request, left, right)
    }
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
#[derive(Debug, Clone)]
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
    pub const fn new(left: L, right: R, handler: H) -> Self {
        Self { left, right, handler }
    }

    /// Create a new dual transport with a function handler.
    pub const fn new_handler<F>(left: L, right: R, f: F) -> DualTransport<L, R, F>
    where
        F: Fn(RequestPacket, L, R) -> TransportFut<'static> + Send + Sync,
    {
        DualTransport { left, right, handler: f }
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request, Response, ResponsePayload};
    use alloy_primitives::B256;
    use serde_json::value::RawValue;
    use std::task::{Context, Poll};

    /// Helper function that transforms a closure to a alloy transport service
    fn request_fn<T>(f: T) -> RequestFn<T>
    where
        T: FnMut(RequestPacket) -> TransportFut<'static>,
    {
        RequestFn { f }
    }

    #[derive(Copy, Clone)]
    struct RequestFn<T> {
        f: T,
    }

    impl<T> Service<RequestPacket> for RequestFn<T>
    where
        T: FnMut(RequestPacket) -> TransportFut<'static>,
    {
        type Response = ResponsePacket;
        type Error = TransportError;
        type Future = TransportFut<'static>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), TransportError>> {
            Ok(()).into()
        }

        fn call(&mut self, req: RequestPacket) -> Self::Future {
            (self.f)(req)
        }
    }

    fn make_hash_response() -> ResponsePacket {
        ResponsePacket::Single(Response {
            id: Id::Number(0),
            payload: ResponsePayload::Success(
                RawValue::from_string(serde_json::to_string(&B256::ZERO).unwrap()).unwrap(),
            ),
        })
    }

    #[tokio::test]
    async fn test_dual_transport() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let left = request_fn(move |request: RequestPacket| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(request).unwrap();
                Ok::<_, TransportError>(make_hash_response())
            })
        });

        let right = request_fn(|_request: RequestPacket| {
            Box::pin(async move { Ok::<_, TransportError>(make_hash_response()) })
        });

        let handler = |req: RequestPacket, mut left: RequestFn<_>, mut right: RequestFn<_>| {
            let id = match &req {
                RequestPacket::Single(req) => req.id().as_number().unwrap_or(0),
                RequestPacket::Batch(reqs) => {
                    reqs.first().map(|r| r.id().as_number().unwrap_or(0)).unwrap_or(0)
                }
            };

            if id % 2 == 0 {
                left.call(req)
            } else {
                right.call(req)
            }
        };

        let mut dual_transport = DualTransport::new(left, right, handler);

        let req_even = RequestPacket::Single(
            Request::new("test", Id::Number(2), None::<&'static RawValue>).try_into().unwrap(),
        );
        let _ = dual_transport.call(req_even).await.unwrap();

        let received = rx.try_recv().unwrap();

        match &received {
            RequestPacket::Single(req) => assert_eq!(*req.id(), Id::Number(2)),
            _ => panic!("Expected Single RequestPacket with id 2, but got something else"),
        }

        let req_odd = RequestPacket::Single(
            Request::new("test", Id::Number(1), None::<&'static RawValue>)
                .try_into()
                .expect("Failed to serialize request"),
        );
        let _ = dual_transport.call(req_odd).await.unwrap();

        assert!(rx.try_recv().is_err(), "Received unexpected request for odd ID");
    }
}
