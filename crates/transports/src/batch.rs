use std::{
    borrow::Cow,
    collections::HashMap,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    task::{self, ready, Poll},
};

use futures_channel::oneshot;
use serde_json::value::RawValue;

use crate::{
    error::TransportError,
    transports::{SingleRequest, Transport},
    RpcClient,
};
use alloy_json_rpc::{Id, Request, Response, RpcParam, RpcResult, RpcReturn};

type Channel = oneshot::Sender<RpcResult<Box<RawValue>, TransportError>>;
type ChannelMap = HashMap<Id, Channel>;

#[must_use = "A BatchRequest does nothing unless sent via `send_batch` and `.await`"]
/// A batch JSON-RPC request, used to bundle requests into a single transport
/// call.
#[derive(Debug)]
pub struct BatchRequest<'a, T> {
    /// The transport via which the batch will be sent.
    transport: &'a RpcClient<T>,

    /// The requests to be sent.
    requests: Vec<SingleRequest>,

    /// The channels to send the responses through.
    channels: ChannelMap,
}

/// Awaits a single response for a request that has been included in a batch.
#[must_use = "A Waiter does nothing unless the corresponding BatchRequest is sent via `send_batch` and `.await`, AND the Waiter is awaited."]
pub struct Waiter<Resp> {
    rx: oneshot::Receiver<RpcResult<Box<RawValue>, TransportError>>,
    _resp: PhantomData<Resp>,
}

impl<Resp> From<oneshot::Receiver<RpcResult<Box<RawValue>, TransportError>>> for Waiter<Resp> {
    fn from(rx: oneshot::Receiver<RpcResult<Box<RawValue>, TransportError>>) -> Self {
        Self {
            rx,
            _resp: PhantomData,
        }
    }
}

impl<Resp> std::future::Future for Waiter<Resp>
where
    Resp: RpcReturn,
{
    type Output = RpcResult<Resp, TransportError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let resp = ready!(Pin::new(&mut self.rx).poll(cx));

        Poll::Ready(match resp {
            Ok(resp) => resp.deser_ok_or_else(|e, text| TransportError::deser_err(e, text)),
            Err(e) => RpcResult::Err(TransportError::Custom(Box::new(e))),
        })
    }
}

#[pin_project::pin_project(project = CallStateProj)]
pub enum BatchFuture<Conn>
where
    Conn: Transport,
{
    Prepared {
        transport: Conn,
        requests: Vec<SingleRequest>,
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
    pub fn new(transport: &'a RpcClient<T>) -> Self {
        Self {
            transport,
            requests: Vec::with_capacity(10),
            channels: HashMap::with_capacity(10),
        }
    }

    fn push_raw(
        &mut self,
        request: SingleRequest,
    ) -> oneshot::Receiver<RpcResult<Box<RawValue>, TransportError>> {
        let (tx, rx) = oneshot::channel();
        self.channels.insert(request.id.clone(), tx);
        self.requests.push(request);
        rx
    }

    fn push<Params: RpcParam, Resp: RpcReturn>(
        &mut self,
        request: Request<Params>,
    ) -> Result<Waiter<Resp>, TransportError> {
        let req = request.try_into()?;
        Ok(self.push_raw(req).into())
    }
}

impl<'a, Conn> BatchRequest<'a, Conn>
where
    Conn: Transport + Clone,
{
    #[must_use = "Waiters do nothing unless polled. A Waiter will never resolve unless its batch is sent."]
    /// Add a call to the batch.
    ///
    /// ### Errors
    ///
    /// If the request cannot be serialized, this will return an error.
    pub fn add_call<Params: RpcParam, Resp: RpcReturn>(
        &mut self,
        method: &'static str,
        params: &Params,
    ) -> Result<Waiter<Resp>, TransportError> {
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
        let CallStateProj::Prepared {
            transport,
            requests,
            channels,
        } = self.as_mut().project()
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

        let req = std::mem::replace(requests, Vec::with_capacity(0));

        let fut = transport.call(req.into());
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

        let responses: Vec<Response> = match serde_json::from_str(responses.get()) {
            Ok(responses) => responses,
            Err(err) => {
                self.set(BatchFuture::Complete);
                return Poll::Ready(Err(TransportError::deser_err(err, responses.get())));
            }
        };

        // Send the responses via the channels by removing the channels from
        // the map.
        for response in responses {
            if let Some(tx) = channels.remove(&response.id) {
                let _ = tx.send(RpcResult::from(response));
            }
        }

        // Any channels remaining in the map are missing responses. To avoid
        // hanging futures, we send an error.
        channels.drain().for_each(|(_, tx)| {
            let _ = tx.send(RpcResult::Err(TransportError::MissingBatchResponse));
        });

        self.set(BatchFuture::Complete);
        Poll::Ready(Ok(()))
    }

    fn poll_ser_error(
        mut self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> Poll<<Self as Future>::Output> {
        let e = if let CallStateProj::SerError(e) = self.as_mut().project() {
            e.take().expect("No error. This is a bug.")
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
    type Output = Result<(), TransportError>;

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
