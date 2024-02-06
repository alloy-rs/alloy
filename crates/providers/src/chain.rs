use std::{marker::PhantomData, sync::Arc};

use alloy_network::Network;
use alloy_rpc_types::Block;
use alloy_transport::{utils::Spawnable, Transport};
use tokio::sync::broadcast;

use crate::{Provider, WeakProvider};

/// Task that emits an ordered set of blocks.
pub(crate) struct ChainTask<P, N, T> {
    provider: WeakProvider<P>,

    _transport: PhantomData<fn() -> (N, T)>,
}

impl<P, T, N> ChainTask<P, N, T>
where
    P: Provider<N, T>,
    N: Network,
    T: Transport,
{
    pub fn new(client: WeakProvider<P>) -> Self {
        Self { provider: client, _transport: PhantomData }
    }

    /// Get the provider, if it still exists.
    pub async fn provider(&self) -> Option<Arc<P>> {
        self.provider.upgrade()
    }

    pub async fn get_height(&self) -> Option<u64> {
        let provider = self.provider.upgrade()?;

        provider.get_block_number().await.ok()
    }
}

pub struct ChainListener {
    rx: broadcast::Receiver<Block>,
}

impl<P, N, T> ChainTask<P, N, T>
where
    P: Provider<N, T>,
    N: Network,
    T: Transport,
{
    pub fn spawn(self) -> ChainListener {
        todo!()
    }
}
