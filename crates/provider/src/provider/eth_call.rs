use alloy_eips::BlockId;
use alloy_json_rpc::RpcReturn;
use alloy_network::Network;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_rpc_types::state::StateOverride;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;
use serde::ser::SerializeSeq;
use std::{future::Future, marker::PhantomData, task::Poll};

type RunningFut<'req, 'state, T, N, Resp, Output, Map> =
    RpcCall<T, EthCallParams<'req, 'state, N>, Resp, Output, Map>;

#[derive(Clone, Debug)]
struct EthCallParams<'req, 'state, N: Network> {
    data: &'req N::TransactionRequest,
    block: BlockId,
    overrides: Option<&'state StateOverride>,
}

impl<N: Network> serde::Serialize for EthCallParams<'_, '_, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = if self.overrides.is_some() { 3 } else { 2 };
        let mut seq = serializer.serialize_seq(Some(len))?;
        seq.serialize_element(&self.data)?;
        seq.serialize_element(&self.block)?;
        if let Some(overrides) = self.overrides {
            seq.serialize_element(overrides)?;
        }
        seq.end()
    }
}

/// The [`EthCallFut`] future is the future type for an `eth_call` RPC request.
#[derive(Clone, Debug)]
#[doc(hidden)] // Not public API.
#[pin_project::pin_project]
pub struct EthCallFut<'req, 'state, T, N, Resp, Output, Map>(
    EthCallFutInner<'req, 'state, T, N, Resp, Output, Map>,
)
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output;

#[derive(Clone, Debug)]
enum EthCallFutInner<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    Preparing {
        client: WeakClient<T>,
        data: &'req N::TransactionRequest,
        overrides: Option<&'state StateOverride>,
        block: Option<BlockId>,
        method: &'static str,
        map: Map,
    },
    Running(RunningFut<'req, 'state, T, N, Resp, Output, Map>),
    Polling,
}

impl<'req, 'state, T, N, Resp, Output, Map> EthCallFutInner<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    /// Returns `true` if the future is in the preparing state.
    const fn is_preparing(&self) -> bool {
        matches!(self, Self::Preparing { .. })
    }

    /// Returns `true` if the future is in the running state.
    const fn is_running(&self) -> bool {
        matches!(self, Self::Running(..))
    }

    fn poll_preparing(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let Self::Preparing { client, data, overrides, block, method, map } =
            std::mem::replace(self, Self::Polling)
        else {
            unreachable!("bad state")
        };

        let client = match client.upgrade().ok_or_else(TransportErrorKind::backend_gone) {
            Ok(client) => client,
            Err(e) => return Poll::Ready(Err(e)),
        };

        let params = EthCallParams { data, block: block.unwrap_or_default(), overrides };

        let fut = client.request(method, params).map_resp(map);

        *self = Self::Running(fut);
        self.poll_running(cx)
    }

    fn poll_running(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let Self::Running(ref mut call) = self else { unreachable!("bad state") };

        call.poll_unpin(cx)
    }
}

impl<'req, 'state, T, N, Resp, Output, Map> Future
    for EthCallFut<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut self.get_mut().0;
        if this.is_preparing() {
            this.poll_preparing(cx)
        } else if this.is_running() {
            this.poll_running(cx)
        } else {
            panic!("unexpected state")
        }
    }
}

/// A builder for an `"eth_call"` request. This type is returned by the
/// [`Provider::call`] method.
///
/// [`Provider::call`]: crate::Provider::call
#[must_use = "EthCall must be awaited to execute the call"]
#[derive(Debug, Clone)]
pub struct EthCall<'req, 'state, T, N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    client: WeakClient<T>,

    data: &'req N::TransactionRequest,
    overrides: Option<&'state StateOverride>,
    block: Option<BlockId>,
    method: &'static str,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
}

impl<'req, T, N, Resp> EthCall<'req, 'static, T, N, Resp>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
{
    /// Create a new CallBuilder.
    pub const fn new(client: WeakClient<T>, data: &'req N::TransactionRequest) -> Self {
        Self {
            client,
            data,
            overrides: None,
            block: None,
            method: "eth_call",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }

    /// Create new EthCall for gas estimates.
    pub const fn gas_estimate(client: WeakClient<T>, data: &'req N::TransactionRequest) -> Self {
        Self {
            client,
            data,
            overrides: None,
            block: None,
            method: "eth_estimateGas",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<'req, 'state, T, N, Resp, Output, Map> EthCall<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    /// Map the response to a different type. This is usable for converting
    /// the response to a more usable type, e.g. changing `U64` to `u64`.
    ///
    /// ## Note
    ///
    /// Carefully review the rust documentation on [fn pointers] before passing
    /// them to this function. Unless the pointer is specifically coerced to a
    /// `fn(_) -> _`, the `NewMap` will be inferred as that function's unique
    /// type. This can lead to confusing error messages.
    ///
    /// [fn pointers]: https://doc.rust-lang.org/std/primitive.fn.html#creating-function-pointers
    pub fn map_resp<NewOutput, NewMap>(
        self,
        map: NewMap,
    ) -> EthCall<'req, 'state, T, N, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        EthCall {
            client: self.client,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
            method: self.method,
            map,
            _pd: PhantomData,
        }
    }

    /// Set the state overrides for this call.
    pub const fn overrides(mut self, overrides: &'state StateOverride) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// Set the block to use for this call.
    pub const fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'req, 'state, T, N, Resp, Output, Map> std::future::IntoFuture
    for EthCall<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = EthCallFut<'req, 'state, T, N, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        EthCallFut(EthCallFutInner::Preparing {
            client: self.client,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
            method: self.method,
            map: self.map,
        })
    }
}
