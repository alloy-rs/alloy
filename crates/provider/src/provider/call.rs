use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::Bytes;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_rpc_types::state::StateOverride;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;
use std::{
    borrow::Cow,
    future::Future,
    task::Poll::{self, Ready},
};

type RunningFut<'req, 'state, T, N> = RpcCall<
    T,
    (&'req <N as Network>::TransactionRequest, BlockId, Option<Cow<'state, StateOverride>>),
    Bytes,
>;

/// The [`EthCallFut`] future is the future type for an `eth_call` RPC request.
#[derive(Debug, Clone)]
pub enum EthCallFut<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    Preparing {
        client: WeakClient<T>,
        data: &'req N::TransactionRequest,
        overrides: Option<&'state StateOverride>,
        block: Option<BlockId>,
    },
    Running(RunningFut<'req, 'state, T, N>),
    Polling,
}

impl<'req, 'state, T, N> EthCallFut<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    /// Returns `true` if the future is in the preparing state.
    const fn is_preparing(&self) -> bool {
        matches!(self, Self::Preparing { .. })
    }

    /// Returns `true` if the future is in the running state.
    const fn is_running(&self) -> bool {
        matches!(self, Self::Running(..))
    }

    fn poll_preparing(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Bytes>> {
        let Self::Preparing { client, data, overrides, block } =
            std::mem::replace(self, Self::Polling)
        else {
            unreachable!("bad state")
        };

        let client = match client.upgrade().ok_or_else(TransportErrorKind::backend_gone) {
            Ok(client) => client,
            Err(e) => return Ready(Err(e)),
        };

        let overrides = overrides.map(Cow::Borrowed);

        let fut = client.request("eth_call", (data, block.unwrap_or_default(), overrides));

        *self = Self::Running(fut);
        self.poll_running(cx)
    }

    fn poll_running(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Bytes>> {
        let Self::Running(ref mut call) = self else { unreachable!("bad state") };

        call.poll_unpin(cx)
    }
}

impl<'req, 'state, T, N> Future for EthCallFut<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    type Output = TransportResult<Bytes>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
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
pub struct EthCall<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    client: WeakClient<T>,

    data: &'req N::TransactionRequest,
    overrides: Option<&'state StateOverride>,
    block: Option<BlockId>,
}

impl<'req, T, N> EthCall<'req, 'static, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    /// Create a new CallBuilder.
    pub const fn new(client: WeakClient<T>, data: &'req N::TransactionRequest) -> Self {
        Self { client, data, overrides: None, block: None }
    }
}

impl<'req, 'state, T, N> EthCall<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    /// Set the state overrides for this call.
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn overrides(mut self, overrides: &'state StateOverride) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// Set the block to use for this call.
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'req, 'state, T, N> std::future::IntoFuture for EthCall<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    type Output = TransportResult<Bytes>;

    type IntoFuture = EthCallFut<'req, 'state, T, N>;

    fn into_future(self) -> Self::IntoFuture {
        EthCallFut::Preparing {
            client: self.client,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
        }
    }
}
