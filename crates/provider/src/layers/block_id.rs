use crate::{EthCall, Provider, ProviderLayer, RootProvider};
use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Bytes, U64};
use std::marker::PhantomData;

/// A layer that sets a default [`BlockId`] for `eth_call` and `eth_estimateGas`.
#[derive(Debug, Clone, Copy)]
pub struct BlockIdLayer {
    block_id: BlockId,
}

impl BlockIdLayer {
    /// Creates a new layer with the given block ID.
    pub const fn new(block_id: BlockId) -> Self {
        Self { block_id }
    }
}

impl From<BlockId> for BlockIdLayer {
    fn from(block_id: BlockId) -> Self {
        Self::new(block_id)
    }
}

impl<P, N> ProviderLayer<P, N> for BlockIdLayer
where
    P: Provider<N>,
    N: Network,
{
    type Provider = BlockIdProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        BlockIdProvider::new(inner, self.block_id)
    }
}

/// A provider that uses a configured default [`BlockId`].
#[derive(Clone, Debug)]
pub struct BlockIdProvider<P, N = alloy_network::Ethereum> {
    inner: P,
    block_id: BlockId,
    _marker: PhantomData<N>,
}

impl<P: Provider<N>, N: Network> BlockIdProvider<P, N> {
    /// Creates a new provider with the given block ID.
    pub const fn new(inner: P, block_id: BlockId) -> Self {
        Self { inner, block_id, _marker: PhantomData }
    }
}

impl<P: Provider<N>, N: Network> Provider<N> for BlockIdProvider<P, N> {
    #[inline(always)]
    fn root(&self) -> &RootProvider<N> {
        self.inner.root()
    }

    fn call(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        EthCall::call(self.weak_client(), tx).block(self.block_id)
    }

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        EthCall::gas_estimate(self.weak_client(), tx)
            .block(self.block_id)
            .map_resp(crate::utils::convert_u64)
    }
}
