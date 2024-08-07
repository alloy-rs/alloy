use alloy_eips::BlockId;
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_primitives::B256;
use alloy_transport::{Transport, TransportResult};
use futures::FutureExt;

use crate::{Caller, ProviderCall};
use std::{
    borrow::Cow,
    future::{Future, IntoFuture},
    marker::PhantomData,
    sync::{Arc, OnceLock},
    task::Poll,
};
/// States of the
#[derive(Clone)]
enum States<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    Invalid,
    Preparing {
        caller: Arc<dyn Caller<T, Params, Resp>>,
        method: Cow<'static, str>,
        params: Params,
        block_id: BlockId,
        map: Map,
    },
    Running {
        map: Map,
    },
}

impl<T, Params, Resp, Output, Map> core::fmt::Debug for States<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.debug_tuple("Invalid").finish(),
            Self::Preparing { caller: _, method, params, block_id, .. } => f
                .debug_struct("Preparing")
                .field("method", method)
                .field("params", params)
                .field("block_id", block_id)
                .finish(),
            Self::Running { map: _ } => f.debug_tuple("Running").finish(),
        }
    }
}

/// A struct that takes an optional [`BlockId`] parameter.
///
/// This resolves to a [`ProviderCall`] that will execute the call on the specified block.
///
/// By default this will use "latest".
#[pin_project::pin_project]
#[derive(Clone)]
pub struct RpcWithBlock<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
{
    caller: Arc<dyn Caller<T, Params, Resp>>,
    method: Cow<'static, str>,
    params: Params,
    block_id: BlockId,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
}

impl<T, Params, Resp> core::fmt::Debug for RpcWithBlock<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcWithBlock")
            .field("method", &self.method)
            .field("params", &self.params)
            .field("block_id", &self.block_id)
            .finish()
    }
}

impl<T, Params, Resp> RpcWithBlock<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    /// Create a new [`RpcWithBlock`] instance.
    pub fn new(
        caller: impl Caller<T, Params, Resp> + 'static,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> Self {
        Self {
            caller: Arc::new(caller),
            method: method.into(),
            params,
            block_id: Default::default(),
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<T, Params, Resp, Output, Map> RpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
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
    ) -> RpcWithBlock<T, Params, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput + Clone,
    {
        RpcWithBlock {
            caller: self.caller,
            method: self.method,
            params: self.params,
            block_id: self.block_id,
            map,
            _pd: PhantomData,
        }
    }

    /// Set the block id.
    pub const fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = block_id;
        self
    }

    /// Set the block id to "pending".
    pub const fn pending(self) -> Self {
        self.block_id(BlockId::pending())
    }

    /// Set the block id to "latest".
    pub const fn latest(self) -> Self {
        self.block_id(BlockId::latest())
    }

    /// Set the block id to "earliest".
    pub const fn earliest(self) -> Self {
        self.block_id(BlockId::earliest())
    }

    /// Set the block id to "finalized".
    pub const fn finalized(self) -> Self {
        self.block_id(BlockId::finalized())
    }

    /// Set the block id to "safe".
    pub const fn safe(self) -> Self {
        self.block_id(BlockId::safe())
    }

    /// Set the block id to a specific height.
    pub const fn number(self, number: u64) -> Self {
        self.block_id(BlockId::number(number))
    }

    /// Set the block id to a specific hash, without requiring the hash be part
    /// of the canonical chain.
    pub const fn hash(self, hash: B256) -> Self {
        self.block_id(BlockId::hash(hash))
    }

    /// Set the block id to a specific hash and require the hash be part of the
    /// canonical chain.
    pub const fn hash_canonical(self, hash: B256) -> Self {
        self.block_id(BlockId::hash_canonical(hash))
    }
}

impl<T, Params, Resp, Output, Map> IntoFuture for RpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone,
{
    type Output = TransportResult<Output>;

    type IntoFuture = WithBlockFut<T, Params, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        WithBlockFut {
            fut: OnceLock::new(),
            state: States::Preparing {
                caller: self.caller,
                method: self.method,
                params: self.params,
                block_id: self.block_id,
                map: self.map,
            },
        }
    }
}

/// Intermediate `Future` type between `RpcWithBlock` and `ProviderCall`, that helps poll
/// the `ProviderCall` and map the response.
#[derive(Debug)]
#[pin_project::pin_project]
pub struct WithBlockFut<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Output: 'static,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
{
    fut: OnceLock<ProviderCall<T, serde_json::Value, Resp>>,
    state: States<T, Params, Resp, Output, Map>,
}

impl<T, Params, Resp, Output, Map> WithBlockFut<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone,
{
    fn poll_preparing(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let this = self.project();
        let States::Preparing { caller, params, method, block_id, map } =
            std::mem::replace(this.state, States::Invalid)
        else {
            unreachable!("bad state")
        };

        let mut fut = caller.call(method, params, block_id)?;

        match fut.poll_unpin(cx) {
            Poll::Ready(value) => Poll::Ready(value.map(map)),
            Poll::Pending => {
                let _ = this.fut.set(fut);
                *this.state = States::Running { map };
                Poll::Pending
            }
        }
    }

    fn poll_running(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let this = self.project();
        let States::Running { map } = this.state else { unreachable!("bad state") };
        this.fut.get_mut().map_or_else(
            || {
                unreachable!("ProviderCall not set");
            },
            |fut| fut.poll_unpin(cx).map(|value| value.map(map)),
        )
    }
}

impl<T, Params, Resp, Output, Map> Future for WithBlockFut<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Output: 'static,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
{
    type Output = TransportResult<Output>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if matches!(self.state, States::Preparing { .. }) {
            self.poll_preparing(cx)
        } else if matches!(self.state, States::Running { .. }) {
            self.poll_running(cx)
        } else {
            panic!("bad state")
        }
    }
}
