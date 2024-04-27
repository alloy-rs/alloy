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
#[derive(Debug, Clone)]
pub struct EthCall<'a, 'b, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    client: ClientRef<'a, T>,

    data: &'b N::TransactionRequest,
    overrides: Option<StateOverride>,
    block: Option<BlockId>,
}

impl<'a, 'b, T, N> EthCall<'a, 'b, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    /// Create a new CallBuilder.
    pub const fn new(client: ClientRef<'a, T>, data: &'b N::TransactionRequest) -> Self {
        Self { client, data, overrides: None, block: None }
    }

    /// Set the state overrides for this call.
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn overrides(mut self, overrides: StateOverride) -> Self {
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

impl<'a, 'b, T, N> std::future::IntoFuture for EthCall<'a, 'b, T, N>
where
    T: Transport + Clone,
    N: Network,
{
    type Output = TransportResult<Bytes>;

    type IntoFuture = RpcCall<T, (&'b N::TransactionRequest, BlockId, StateOverride), Bytes>;

    fn into_future(self) -> Self::IntoFuture {
        self.client.request(
            "eth_call",
            (self.data, self.block.unwrap_or_default(), self.overrides.unwrap_or_default()),
        )
    }
}
