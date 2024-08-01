use crate::ProviderCall;
use alloy_eips::BlockId;
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_primitives::B256;
use alloy_rpc_client::WeakClient;
use alloy_transport::{Transport, TransportResult};
use futures::FutureExt;
use std::{borrow::Cow, future::Future, marker::PhantomData, task::Poll};
/// States of the
// #[derive(Clone)]
// enum States<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
// where
//     T: Transport + Clone,
//     Params: RpcParam,
//     Resp: RpcReturn,
//     Map: Fn(Resp) -> Output + Clone,
// {
//     Invalid,
//     Preparing,
//     Running(ProviderCall<T, Params, Resp, Output, Map>),
// }

#[derive(Clone)]
enum States {
    Invalid,
    Preparing,
    Running, /* Removed the encapsulation of ProviderCall because it would require impl Clone
              * on ProviderCall. Clone cannot be implemented on ProviderCall as it has a
              * BoxedFuture variant */
}

impl core::fmt::Debug for States {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.debug_tuple("Invalid").finish(),
            Self::Preparing => f.debug_struct("Preparing").finish(),
            Self::Running => f.debug_tuple("Running").finish(),
        }
    }
}

/// An [`RpcCall`] that takes an optional [`BlockId`] parameter. By default
/// this will use "latest".
#[derive(Clone)]
#[pin_project::pin_project]
pub struct RpcWithBlock<T, Params, Resp, F, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
    F: FnOnce(
        Cow<'static, str>,
        Params,
        BlockId,
        Option<WeakClient<T>>,
    ) -> TransportResult<ProviderCall<T, Params, Resp>>,
{
    client: WeakClient<T>, // TODO: Remove this.
    method: Cow<'static, str>,
    params: Params,
    into_prov_call: Option<F>,
    block_id: BlockId,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
    state: States,
}

impl<T, Params, Resp, F> core::fmt::Debug for RpcWithBlock<T, Params, Resp, F>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    F: FnOnce(
        Cow<'static, str>,
        Params,
        BlockId,
        Option<WeakClient<T>>,
    ) -> TransportResult<ProviderCall<T, Params, Resp>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcWithBlock")
            .field("method", &self.method)
            .field("params", &self.params)
            .field("block_id", &self.block_id)
            .finish()
    }
}

impl<T, Params, Resp, F> RpcWithBlock<T, Params, Resp, F>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    F: FnOnce(
        Cow<'static, str>,
        Params,
        BlockId,
        Option<WeakClient<T>>,
    ) -> TransportResult<ProviderCall<T, Params, Resp>>,
{
    /// Create a new [`RpcWithBlock`] instance.
    pub fn new(
        client: WeakClient<T>,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> Self {
        Self {
            client,
            method: method.into(),
            params,
            into_prov_call: Default::default(),
            block_id: Default::default(),
            map: std::convert::identity,
            _pd: PhantomData,
            state: States::Preparing,
        }
    }
}

impl<T, Params, Resp, F, Output> RpcWithBlock<T, Params, Resp, F, Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    F: FnOnce(
        Cow<'static, str>,
        Params,
        BlockId,
        Option<WeakClient<T>>,
    ) -> TransportResult<ProviderCall<T, Params, Resp>>,
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
    ) -> RpcWithBlock<T, Params, Resp, F, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput + Clone,
    {
        RpcWithBlock {
            client: self.client,
            method: self.method,
            params: self.params,
            into_prov_call: self.into_prov_call,
            block_id: self.block_id,
            map,
            _pd: PhantomData,
            state: match self.state {
                States::Invalid => States::Invalid,
                States::Preparing => States::Preparing,
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

    /// Sets the closure that converts `RpcWithBlock` into a [`ProviderCall`] when polled.
    pub fn into_prov_call(mut self, f: F) -> Self {
        self.into_prov_call = Some(f);
        self
    }
}

impl<T, Params, Resp, F, Output, Map> RpcWithBlock<T, Params, Resp, F, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone,
    F: FnOnce(
        Cow<'static, str>,
        Params,
        BlockId,
        Option<WeakClient<T>>,
    ) -> TransportResult<ProviderCall<T, Params, Resp>>,
{
    fn poll_preparing(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let this = self.project();
        let States::Preparing { .. } = std::mem::replace(this.state, States::Invalid) else {
            unreachable!("bad state")
        };

        let mut fut = if let Some(f) = this.into_prov_call.take() {
            match f(
                this.method.clone(),
                this.params.clone(),
                *this.block_id,
                Some(this.client.clone()),
            ) {
                Ok(call) => call,
                Err(e) => return Poll::Ready(Err(e)),
            }
        } else {
            unreachable!("into_prov_call not set")
        };

        // poll the call immediately
        match fut.poll_unpin(cx) {
            Poll::Ready(value) => Poll::Ready(value.map(this.map.clone())),
            Poll::Pending => {
                *this.state = States::Running;
                Poll::Pending
            }
        }
    }

    fn poll_running(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let this = self.project();
        let States::Running { .. } = this.state else { unreachable!("bad state") };
        let fut = if let Some(f) = this.into_prov_call.take() {
            tracing::info!("Params in poll running {:#?}", this.params.clone());
            match f(
                this.method.clone(),
                this.params.clone(),
                *this.block_id,
                Some(this.client.clone()),
            ) {
                Ok(call) => call,
                Err(e) => return Poll::Ready(Err(e)),
            }
        } else {
            unreachable!("into_prov_call not set");
        };

        let mut fut = fut.map_resp(this.map.clone()).unwrap();

        fut.poll_unpin(cx)
    }
}

impl<T, Params, Resp, F, Output, Map> Future for RpcWithBlock<T, Params, Resp, F, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Output: 'static,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output + Clone,
    F: FnOnce(
        Cow<'static, str>,
        Params,
        BlockId,
        Option<WeakClient<T>>,
    ) -> TransportResult<ProviderCall<T, Params, Resp>>,
{
    type Output = TransportResult<Output>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if matches!(self.state, States::Preparing) {
            self.poll_preparing(cx)
        } else if matches!(self.state, States::Running) {
            self.poll_running(cx)
        } else {
            panic!("bad state")
        }
    }
}
