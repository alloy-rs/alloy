use crate::{client::RpcClientInner, ClientRef};
use alloy_json_rpc::{
    transform_response, try_deserialize_ok, Id, Request, RequestPacket, ResponsePacket, RpcParam,
    RpcReturn, SerializedRequest,
};
use alloy_transport::{Transport, TransportError, TransportErrorKind, TransportResult};
use futures::channel::oneshot;
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::HashMap,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    task::{self, ready, Poll},
};

pub(crate) type Channel = oneshot::Sender<TransportResult<Box<RawValue>>>;
pub(crate) type ChannelMap = HashMap<Id, Channel>;

/// A batch JSON-RPC request, used to bundle requests into a single transport
/// call.
#[derive(Debug)]
#[must_use = "A BatchRequest does nothing unless sent via `send_batch` and `.await`"]
pub struct BatchRequest<'a, T> {
    /// The transport via which the batch will be sent.
    transport: ClientRef<'a, T>,

    /// The requests to be sent.
    requests: RequestPacket,

    /// The channels to send the responses through.
    channels: ChannelMap,
}

/// Awaits a single response for a request that has been included in a batch.
#[must_use = "A Waiter does nothing unless the corresponding BatchRequest is sent via `send_batch` and `.await`, AND the Waiter is awaited."]
#[derive(Debug)]
pub struct Waiter<Resp> {
    rx: oneshot::Receiver<TransportResult<Box<RawValue>>>,
    _resp: PhantomData<fn() -> Resp>,
}

impl<Resp> From<oneshot::Receiver<TransportResult<Box<RawValue>>>> for Waiter<Resp> {
    fn from(rx: oneshot::Receiver<TransportResult<Box<RawValue>>>) -> Self {
        Self { rx, _resp: PhantomData }
    }
}

impl<Resp> std::future::Future for Waiter<Resp>
where
    Resp: RpcReturn,
{
    type Output = TransportResult<Resp>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.rx).poll(cx).map(|resp| match resp {
            Ok(resp) => try_deserialize_ok(resp),
            Err(e) => Err(TransportErrorKind::custom(e)),
        })
    }
}

#[pin_project::pin_project(project = CallStateProj)]
#[derive(Debug)]
pub enum BatchFuture<Conn: Transport> {
    Prepared {
        transport: Conn,
        requests: RequestPacket,
        channels: ChannelMap,
    },
    SerError(Option<TransportError>),
    AwaitingResponse {
        channels: ChannelMap,
        #[pin]
        fut: Conn::Future,
    },
    Complete,
}

impl<'a, T> BatchRequest<'a, T> {
    /// Create a new batch request.
    pub fn new(transport: &'a RpcClientInner<T>) -> Self {
        Self {
            transport,
            requests: RequestPacket::Batch(Vec::with_capacity(10)),
            channels: HashMap::with_capacity(10),
        }
    }

    fn push_raw(
        &mut self,
        request: SerializedRequest,
    ) -> oneshot::Receiver<TransportResult<Box<RawValue>>> {
        let (tx, rx) = oneshot::channel();
        self.channels.insert(request.id().clone(), tx);
        self.requests.push(request);
        rx
    }

    fn push<Params: RpcParam, Resp: RpcReturn>(
        &mut self,
        request: Request<Params>,
    ) -> TransportResult<Waiter<Resp>> {
        let ser = request.serialize().map_err(TransportError::ser_err)?;
        Ok(self.push_raw(ser).into())
    }
}

impl<'a, Conn> BatchRequest<'a, Conn>
where
    Conn: Transport + Clone,
{
    /// Add a call to the batch.
    ///
    /// ### Errors
    ///
    /// If the request cannot be serialized, this will return an error.
    pub fn add_call<Params: RpcParam, Resp: RpcReturn>(
        &mut self,
        method: impl Into<Cow<'static, str>>,
        params: &Params,
    ) -> TransportResult<Waiter<Resp>> {
        let request = self.transport.make_request(method, Cow::Borrowed(params));
        self.push(request)
    }

    /// Send the batch future via its connection.
    pub fn send(self) -> BatchFuture<Conn> {
        BatchFuture::Prepared {
            transport: self.transport.transport.clone(),
            requests: self.requests,
            channels: self.channels,
        }
    }
}

impl<'a, T> IntoFuture for BatchRequest<'a, T>
where
    T: Transport + Clone,
{
    type Output = <BatchFuture<T> as Future>::Output;
    type IntoFuture = BatchFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        self.send()
    }
}

impl<T> BatchFuture<T>
where
    T: Transport + Clone,
{
    fn poll_prepared(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<<Self as Future>::Output> {
        let CallStateProj::Prepared { transport, requests, channels } = self.as_mut().project()
        else {
            unreachable!("Called poll_prepared in incorrect state")
        };

        if let Err(e) = task::ready!(transport.poll_ready(cx)) {
            self.set(BatchFuture::Complete);
            return Poll::Ready(Err(e));
        }

        // We only have mut refs, and we want ownership, so we just replace
        // with 0-capacity collections.
        let channels = std::mem::replace(channels, HashMap::with_capacity(0));
        let req = std::mem::replace(requests, RequestPacket::Batch(Vec::with_capacity(0)));

        let fut = transport.call(req);
        self.set(BatchFuture::AwaitingResponse { channels, fut });
        cx.waker().wake_by_ref();
        Poll::Pending
    }

    fn poll_awaiting_response(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<<Self as Future>::Output> {
        let CallStateProj::AwaitingResponse { channels, fut } = self.as_mut().project() else {
            unreachable!("Called poll_awaiting_response in incorrect state")
        };

        // Has the service responded yet?
        let responses = match ready!(fut.poll(cx)) {
            Ok(responses) => responses,
            Err(e) => {
                self.set(BatchFuture::Complete);
                return Poll::Ready(Err(e));
            }
        };

        // Send all responses via channels
        match responses {
            ResponsePacket::Single(single) => {
                if let Some(tx) = channels.remove(&single.id) {
                    let _ = tx.send(transform_response(single));
                }
            }
            ResponsePacket::Batch(responses) => {
                for response in responses.into_iter() {
                    if let Some(tx) = channels.remove(&response.id) {
                        let _ = tx.send(transform_response(response));
                    }
                }
            }
        }

        // Any channels remaining in the map are missing responses.
        // To avoid hanging futures, we send an error.
        for (id, tx) in channels.drain() {
            let _ = tx.send(Err(TransportErrorKind::missing_batch_response(id)));
        }

        self.set(BatchFuture::Complete);
        Poll::Ready(Ok(()))
    }

    fn poll_ser_error(
        mut self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> Poll<<Self as Future>::Output> {
        let e = if let CallStateProj::SerError(e) = self.as_mut().project() {
            e.take().expect("no error")
        } else {
            unreachable!("Called poll_ser_error in incorrect state")
        };

        self.set(BatchFuture::Complete);
        Poll::Ready(Err(e))
    }
}

impl<T> Future for BatchFuture<T>
where
    T: Transport + Clone,
{
    type Output = TransportResult<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        if matches!(*self.as_mut(), BatchFuture::Prepared { .. }) {
            return self.poll_prepared(cx);
        }

        if matches!(*self.as_mut(), BatchFuture::AwaitingResponse { .. }) {
            return self.poll_awaiting_response(cx);
        }

        if matches!(*self.as_mut(), BatchFuture::SerError(_)) {
            return self.poll_ser_error(cx);
        }

        panic!("Called poll on CallState in invalid state")
    }
}
