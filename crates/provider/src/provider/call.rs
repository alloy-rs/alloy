use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::Bytes;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_rpc_types::state::StateOverride;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;
use std::{borrow::Cow, future::Future, task::Poll};

/// States for the [`EthCallFut`] future.
#[derive(Debug, Clone)]
enum States<'req, 'state, T, N>
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
    Running(RpcCall<T, (&'req N::TransactionRequest, BlockId, Cow<'state, StateOverride>), Bytes>),
}

/// Future for [`EthCall`]. Simple wrapper around [`RpcCall`].
#[derive(Debug, Clone)]
pub struct EthCallFut<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    state: States<'req, 'state, T, N>,
}

impl<'req, 'state, T, N> EthCallFut<'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    fn poll_preparing(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Bytes>> {
        let fut = {
            let States::Preparing { client, data, overrides, block } = &self.as_ref().state else {
                unreachable!("bad state")
            };

            let client = match client.upgrade().ok_or_else(TransportErrorKind::backend_gone) {
                Ok(client) => client,
                Err(e) => return std::task::Poll::Ready(Err(e)),
            };

            let overrides = match overrides {
                Some(overrides) => Cow::Borrowed(*overrides),
                None => Cow::Owned(StateOverride::default()),
            };
            client.request("eth_call", (*data, block.unwrap_or_default(), overrides))
        };

        self.state = States::Running(fut);
        self.poll_running(cx)
    }

    fn poll_running(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Bytes>> {
        let Self { state: States::Running(call) } = self.get_mut() else {
            unreachable!("bad state")
        };

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
        if matches!(self.state, States::Preparing { .. }) {
            self.poll_preparing(cx)
        } else {
            self.poll_running(cx)
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
        let state = States::Preparing {
            client: self.client,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
        };

        EthCallFut { state }
    }
}
