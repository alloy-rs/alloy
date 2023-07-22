use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{self, ready},
};

use futures_channel::oneshot;
use serde_json::value::RawValue;
use tower::Service;

use crate::{error::TransportError, transports::BatchTransportFuture};
use alloy_json_rpc::{Id, JsonRpcRequest, JsonRpcResponse, RpcResult, RpcReturn};

type Channel = oneshot::Sender<RpcResult<Box<RawValue>, TransportError>>;
type ChannelMap = HashMap<Id, Channel>;

/// A Batch JSON-RPC request, awaiting dispatch.
#[derive(Debug, Default)]
pub struct BatchRequest<T> {
    transport: T,

    requests: Vec<JsonRpcRequest>,

    channels: ChannelMap,
}

/// Awaits a single response for a request that has been included in a batch.
pub struct Waiter<Resp> {
    rx: oneshot::Receiver<RpcResult<Box<RawValue>, TransportError>>,
    _resp: PhantomData<Resp>,
}

impl<Resp> std::future::Future for Waiter<Resp>
where
    Resp: RpcReturn,
{
    type Output = RpcResult<Resp, TransportError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let resp = ready!(Pin::new(&mut self.rx).poll(cx));

        task::Poll::Ready(match resp {
            Ok(resp) => resp.deser_ok_or_else(|e, text| TransportError::deser_err(e, text)),
            Err(e) => RpcResult::Err(TransportError::Custom(Box::new(e))),
        })
    }
}

#[pin_project::pin_project(project = CallStateProj)]
pub enum BatchFuture<T>
where
    T: Service<
        Vec<JsonRpcRequest>,
        Response = Vec<JsonRpcResponse>,
        Error = TransportError,
        Future = BatchTransportFuture,
    >,
{
    Prepared(BatchRequest<T>),
    SerError(Option<TransportError>),
    AwaitingResponse {
        channels: ChannelMap,
        #[pin]
        fut: <T as Service<Vec<JsonRpcRequest>>>::Future,
    },
    Complete,
}

impl<T> BatchRequest<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            requests: Vec::with_capacity(10),
            channels: HashMap::with_capacity(10),
        }
    }

    pub fn push_req(
        &mut self,
        request: JsonRpcRequest,
    ) -> oneshot::Receiver<RpcResult<Box<RawValue>, TransportError>> {
        let (tx, rx) = oneshot::channel();
        self.channels.insert(request.id.clone(), tx);
        self.requests.push(request);
        rx
    }
}

impl<T> BatchRequest<T>
where
    T: Service<
        Vec<JsonRpcRequest>,
        Response = Vec<JsonRpcResponse>,
        Error = TransportError,
        Future = BatchTransportFuture,
    >,
{
    /// Send the batch future via its connection.
    pub fn send(self) -> BatchFuture<T> {
        BatchFuture::Prepared(self)
    }
}

impl<T> BatchFuture<T>
where
    T: Service<
        Vec<JsonRpcRequest>,
        Response = Vec<JsonRpcResponse>,
        Error = TransportError,
        Future = BatchTransportFuture,
    >,
{
    fn poll_prepared(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let CallStateProj::Prepared(BatchRequest {
            transport,
            requests,
            channels,
        }) = self.as_mut().project()
        else {
            unreachable!("Called poll_prepared in incorrect state")
        };

        if let Err(e) = task::ready!(transport.poll_ready(cx)) {
            self.set(BatchFuture::Complete);
            return task::Poll::Ready(Err(e));
        }

        // We only have mut refs, and we want ownership, so we just replace
        // with 0-capacity collections.
        let channels = std::mem::replace(channels, HashMap::with_capacity(0));
        let requests = std::mem::replace(requests, Vec::with_capacity(0));

        let fut = transport.call(requests);

        self.set(BatchFuture::AwaitingResponse { channels, fut });
        cx.waker().wake_by_ref();

        task::Poll::Pending
    }

    fn poll_awaiting_response(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let CallStateProj::AwaitingResponse { channels, fut } = self.as_mut().project() else {
            unreachable!("Called poll_awaiting_response in incorrect state")
        };

        // Has the service responded yet?
        let responses = match ready!(fut.poll(cx)) {
            Ok(responses) => responses,
            Err(e) => {
                self.set(BatchFuture::Complete);
                return task::Poll::Ready(Err(e));
            }
        };

        // Drain the responses into the channels.
        for response in responses {
            if let Some(tx) = channels.remove(&response.id) {
                let _ = tx.send(RpcResult::from(response));
            }
        }

        // Any remaining channels are missing responses.
        channels.drain().for_each(|(_, tx)| {
            let _ = tx.send(RpcResult::Err(TransportError::MissingBatchResponse));
        });

        self.set(BatchFuture::Complete);
        task::Poll::Ready(Ok(()))
    }

    fn poll_ser_error(
        mut self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let e = if let CallStateProj::SerError(e) = self.as_mut().project() {
            e.take().expect("No error. This is a bug.")
        } else {
            unreachable!("Called poll_ser_error in incorrect state")
        };

        self.set(BatchFuture::Complete);
        task::Poll::Ready(Err(e))
    }
}

impl<T> Future for BatchFuture<T>
where
    T: Service<
        Vec<JsonRpcRequest>,
        Response = Vec<JsonRpcResponse>,
        Error = TransportError,
        Future = BatchTransportFuture,
    >,
{
    type Output = Result<(), TransportError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if matches!(*self.as_mut(), BatchFuture::Prepared(_)) {
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
