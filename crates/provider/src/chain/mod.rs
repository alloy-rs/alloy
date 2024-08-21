use alloy_network::{Ethereum, Network};
use alloy_primitives::BlockNumber;
use alloy_rpc_client::WeakClient;
use alloy_rpc_types_eth::Block;
use alloy_transport::Transport;
use futures::Stream;
use std::marker::PhantomData;

use self::stream::PollerStream;

mod stream;

#[derive(Debug, thiserror::Error)]
enum ChainStreamError<T: Transport> {
    #[error("failed to perform an RPC request: {0}")]
    Rpc(#[source] T::Error),
    #[error("Polling stream ended")]
    PollingStreamEnded,
    #[error("Client dropped")]
    ClientDropped,
}

#[derive(Debug)]
pub(crate) struct ChainStreamPoller<T, N = Ethereum> {
    client: WeakClient<T>,
    next_yield: Option<BlockNumber>,
    _phantom: PhantomData<N>,
}

impl<T: Transport + Clone, N: Network> ChainStreamPoller<T, N> {
    pub(crate) fn from_weak_client(client: WeakClient<T>) -> Self {
        Self::with_next_yield(client, None)
    }

    /// Can be used to force the poller to start at a specific block number.
    /// Mostly useful for tests.
    fn with_next_yield(client: WeakClient<T>, next_yield: Option<BlockNumber>) -> Self {
        Self { client, next_yield, _phantom: PhantomData }
    }

    pub(crate) fn into_stream(self) -> impl Stream<Item = Block> + 'static {
        PollerStream::<T, N>::stream(self.client, self.next_yield)
    }
}

#[cfg(all(test, feature = "anvil-api"))] // Tests rely heavily on ability to mine blocks on demand.
mod tests {
    use std::{future::Future, time::Duration};

    use crate::{ext::AnvilApi, ProviderBuilder};
    use alloy_node_bindings::Anvil;
    use alloy_primitives::U256;
    use alloy_rpc_client::ReqwestClient;
    use futures::StreamExt;

    use super::*;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    async fn with_timeout<T: Future>(fut: T) -> T::Output {
        const TEST_TIMEOUT: Duration = Duration::from_secs(1);
        tokio::time::timeout(TEST_TIMEOUT, fut).await.expect("Operation timed out")
    }

    #[tokio::test]
    async fn yield_block() {
        init_tracing();

        let anvil = Anvil::new().spawn();

        let client = ReqwestClient::new_http(anvil.endpoint_url());
        let poller: ChainStreamPoller<_, Ethereum> =
            ChainStreamPoller::with_next_yield(client.get_weak(), Some(1));
        let mut stream = Box::pin(poller.into_stream());

        // We will also use provider to manipulate anvil instance via RPC.
        let provider = ProviderBuilder::new().on_http(anvil.endpoint_url());
        provider.anvil_mine(Some(U256::from(1)), None).await.unwrap();

        let block = with_timeout(stream.next()).await.expect("Block wasn't fetched");
        assert_eq!(block.header.number, Some(1u64));
    }

    #[tokio::test]
    async fn yield_many_blocks() {
        const BLOCKS_TO_MINE: usize = 100;

        init_tracing();

        let anvil = Anvil::new().spawn();

        let client = ReqwestClient::new_http(anvil.endpoint_url());
        let poller: ChainStreamPoller<_, Ethereum> =
            ChainStreamPoller::with_next_yield(client.get_weak(), Some(1));
        let stream = Box::pin(poller.into_stream());

        // We will also use provider to manipulate anvil instance via RPC.
        let provider = ProviderBuilder::new().on_http(anvil.endpoint_url());
        provider.anvil_mine(Some(U256::from(BLOCKS_TO_MINE)), None).await.unwrap();

        let blocks = with_timeout(stream.take(BLOCKS_TO_MINE).collect::<Vec<_>>()).await;
        assert_eq!(blocks.len(), BLOCKS_TO_MINE);
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.header.number, Some((i + 1) as u64), "Unexpected block number");
        }
    }
}
