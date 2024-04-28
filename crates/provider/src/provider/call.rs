use std::borrow::Cow;

use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::Bytes;
use alloy_rpc_client::{ClientRef, RpcCall};
use alloy_rpc_types::state::StateOverride;
use alloy_transport::{Transport, TransportResult};

/// A builder for an `"eth_call"` request. This type is returned by the
/// [`Provider::call`] method.
///
/// [`Provider::call`]: crate::Provider::call
#[must_use = "EthCall must be awaited to execute the call"]
#[derive(Debug, Clone)]
pub struct EthCall<'client, 'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    client: ClientRef<'client, T>,

    data: &'req N::TransactionRequest,
    overrides: Option<&'state StateOverride>,
    block: Option<BlockId>,
}

impl<'client, 'req, T, N> EthCall<'client, 'req, 'static, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    /// Create a new CallBuilder.
    pub const fn new(client: ClientRef<'client, T>, data: &'req N::TransactionRequest) -> Self {
        Self { client, data, overrides: None, block: None }
    }
}

impl<'client, 'req, 'state, T, N> EthCall<'client, 'req, 'state, T, N>
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

impl<'client, 'req, 'state, T, N> std::future::IntoFuture for EthCall<'client, 'req, 'state, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    type Output = TransportResult<Bytes>;

    type IntoFuture =
        RpcCall<T, (&'req N::TransactionRequest, BlockId, Cow<'state, StateOverride>), Bytes>;

    fn into_future(self) -> Self::IntoFuture {
        let overrides = match self.overrides {
            Some(overrides) => Cow::Borrowed(overrides),
            None => Cow::Owned(StateOverride::default()),
        };

        self.client.request("eth_call", (self.data, self.block.unwrap_or_default(), overrides))
    }
}
