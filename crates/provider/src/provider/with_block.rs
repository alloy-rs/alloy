use alloy_eips::BlockId;
use alloy_json_rpc::{RpcError, RpcParam, RpcReturn};
use alloy_primitives::B256;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;

use std::{borrow::Cow, future::Future, marker::PhantomData, sync::Arc, task::Poll};

use crate::{provider::caller::WithBlockCall, Caller};
/// States of the
// #[derive(Clone)]
// enum States<T, Resp, Output = Resp, Map = fn(Resp) -> Output>
// where
//     T: Transport + Clone,
//     Resp: RpcReturn,
//     Map: Fn(Resp) -> Output,
// {
//     Invalid,
//     Preparing,
//     Running(RpcCall<T, serde_json::Value, Resp, Output, Map>),
// }
#[derive(Clone)]
enum States {
    Invalid,
    Preparing,
    Running,
}

// impl<T, Resp, Output, Map> core::fmt::Debug for States<T, Resp, Output, Map>
// where
//     T: Transport + Clone,
//     Resp: RpcReturn,
//     Map: Fn(Resp) -> Output,
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Invalid => f.debug_tuple("Invalid").finish(),
//             Self::Preparing => f.debug_struct("Preparing").finish(),
//             Self::Running(arg0) => f.debug_tuple("Running").field(arg0).finish(),
//         }
//     }
// }

impl core::fmt::Debug for States {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.debug_tuple("Invalid").finish(),
            Self::Preparing => f.debug_struct("Preparing").finish(),
            Self::Running => f.debug_struct("Running").finish(),
        }
    }
}

/// An [`RpcCall`] that takes an optional [`BlockId`] parameter. By default
/// this will use "latest".
#[pin_project::pin_project]
pub struct RpcWithBlock<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
{
    caller: Box<dyn Caller<T, Params, Resp>>,
    method: Cow<'static, str>,
    params: Params,
    block_id: BlockId,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
    state: States,
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
            caller: Box::new(caller),
            method: method.into(),
            params,
            block_id: Default::default(),
            map: std::convert::identity,
            _pd: PhantomData,
            state: States::Preparing,
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
            state: match self.state {
                States::Invalid => States::Invalid,
                States::Preparing => States::Preparing,
                // TODO: Had to add the Clone bound on Map due to this. Can we find a way to remove
                // this? .
                States::Running => States::Running,
            },
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

impl<T, Params, Resp, Output, Map> RpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone,
{
    fn poll_caller(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let this = self.project();
        let States::Preparing { .. } = std::mem::replace(this.state, States::Invalid) else {
            unreachable!("bad state")
        };

        let mut fut = this.caller.call(this.method.clone(), this.params.clone(), *this.block_id)?;

        match fut.poll_unpin(cx) {
            Poll::Ready(value) => Poll::Ready(value.map(this.map.clone())),
            Poll::Pending => {
                *this.state = States::Running;
                Poll::Pending
            }
        }
    }
}

impl<T, Params, Resp, Output, Map> Future for RpcWithBlock<T, Params, Resp, Output, Map>
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
            self.poll_caller(cx)
        } else if matches!(self.state, States::Running { .. }) {
            self.poll_caller(cx)
        } else {
            panic!("bad state")
        }
    }
}
