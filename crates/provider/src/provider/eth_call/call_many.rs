use std::sync::Arc;

use alloy_eips::BlockId;
use alloy_json_rpc::RpcRecv;
use alloy_network::Network;
use alloy_rpc_types_eth::{state::StateOverride, Bundle, StateContext, TransactionIndex};
use alloy_transport::TransportResult;
use futures::FutureExt;

use super::{Caller, EthCallManyParams};

/// A builder for an `"eth_callMany"` RPC request.
#[derive(Clone)]
pub struct EthCallMany<'req, N: Network, Resp: RpcRecv> {
    caller: Arc<dyn Caller<N, Resp>>,
    params: EthCallManyParams<'req>,
}

impl<N, Resp> std::fmt::Debug for EthCallMany<'_, N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EthCallMany")
            .field("params", &self.params)
            .field("method", &"eth_callMany")
            .finish()
    }
}

impl<'req, N, Resp> EthCallMany<'req, N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    /// Instantiates a new `EthCallMany` with the given parameters.
    pub fn new(caller: Arc<dyn Caller<N, Resp>>, bundles: &'req Vec<Bundle>) -> Self {
        Self { caller, params: EthCallManyParams::new(bundles) }
    }

    /// Set the [`BlockId`] in the [`StateContext`].
    pub fn block(mut self, block: BlockId) -> Self {
        self.params = self.params.with_block(block);
        self
    }

    /// Set the [`TransactionIndex`] in the [`StateContext`].
    pub fn transaction_index(mut self, tx_index: TransactionIndex) -> Self {
        self.params = self.params.with_transaction_index(tx_index);
        self
    }

    /// Set the [`StateContext`] for the call.
    pub fn context(mut self, context: &'req StateContext) -> Self {
        self.params = self.params.with_context(context.clone());
        self
    }

    /// Set the [`StateOverride`] for the call.
    pub fn overrides(mut self, overrides: &'req StateOverride) -> Self {
        self.params = self.params.with_overrides(overrides);
        self
    }
}

impl<'req, N, Resp> std::future::Future for EthCallMany<'req, N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    type Output = TransportResult<Resp>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut fut = self.caller.call_many(self.params.clone())?;

        fut.poll_unpin(cx)
    }
}
