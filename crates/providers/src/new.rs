use crate::{
    chain::ChainStreamPoller,
    heart::{Heartbeat, HeartbeatHandle, PendingTransaction, WatchConfig},
};
use alloy_network::{Network, Transaction};
use alloy_primitives::{hex, Address, BlockNumber, B256, U256, U64};
use alloy_rpc_client::{ClientRef, RpcClient, WeakClient};
use alloy_rpc_types::Block;
use alloy_transport::{BoxTransport, Transport, TransportErrorKind, TransportResult};
use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock, Weak},
};

/// A [`Provider`] in a [`Weak`] reference.
pub type WeakProvider<P> = Weak<P>;

/// A borrowed [`Provider`].
pub type ProviderRef<'a, P> = &'a P;

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<N, T> {
    /// The inner state of the root provider.
    pub(crate) inner: Arc<RootProviderInner<N, T>>,
}

impl<N: Network, T: Transport> RootProvider<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { inner: Arc::new(RootProviderInner::new(client)) }
    }
}

impl<N: Network, T: Transport + Clone> RootProvider<N, T> {
    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction> {
        self.get_heart()
            .watch_tx(WatchConfig::new(tx_hash))
            .await
            .map_err(|_| TransportErrorKind::backend_gone())
    }

    #[inline]
    fn get_heart(&self) -> &HeartbeatHandle {
        self.inner.heart.get_or_init(|| {
            let weak = Arc::downgrade(&self.inner);
            let stream = ChainStreamPoller::new(weak, self.inner.weak_client());
            // TODO: Can we avoid `Pin<Box<_>>` here?
            Heartbeat::new(Box::pin(stream.into_stream())).spawn()
        })
    }
}

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub(crate) struct RootProviderInner<N, T> {
    client: RpcClient<T>,
    heart: OnceLock<HeartbeatHandle>,
    _network: PhantomData<N>,
}

impl<N, T> RootProviderInner<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { client, heart: OnceLock::new(), _network: PhantomData }
    }

    fn weak_client(&self) -> WeakClient<T> {
        self.client.get_weak()
    }

    fn client_ref(&self) -> ClientRef<'_, T> {
        self.client.get_ref()
    }
}

/// Provider is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Provider<N, Http>`.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[auto_impl::auto_impl(&, &mut, Rc, Arc, Box)]
pub trait Provider<N: Network, T: Transport + Clone = BoxTransport>: Send + Sync {
    /// Returns the RPC client used to send requests.
    fn client(&self) -> ClientRef<'_, T>;

    /// Returns a [`Weak`] RPC client used to send requests.
    fn weak_client(&self) -> WeakClient<T>;

    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction>;

    async fn estimate_gas(&self, tx: &N::TransactionRequest) -> TransportResult<U256> {
        self.client().prepare("eth_estimateGas", (tx,)).await
    }

    /// Get the last block number available.
    async fn get_block_number(&self) -> TransportResult<BlockNumber> {
        self.client().prepare("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    /// Get the transaction count for an address. Used for finding the
    /// appropriate nonce.
    ///
    /// TODO: block number/hash/tag
    async fn get_transaction_count(&self, address: Address) -> TransportResult<U256> {
        self.client().prepare("eth_getTransactionCount", (address, "latest")).await
    }

    /// Get a block by its number.
    ///
    /// TODO: Network associate
    async fn get_block_by_number(
        &self,
        number: BlockNumber,
        hydrate: bool,
    ) -> TransportResult<Block> {
        self.client().prepare("eth_getBlockByNumber", (number, hydrate)).await
    }

    /// Populate the gas limit for a transaction.
    async fn populate_gas(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        let gas = self.estimate_gas(&*tx).await?;
        if let Ok(gas) = gas.try_into() {
            tx.set_gas_limit(gas);
        }
        Ok(())
    }

    /// Broadcasts a transaction, returning a [`PendingTransaction`] that resolves once the
    /// transaction has been confirmed.
    async fn send_transaction(
        &self,
        tx: &N::TransactionRequest,
    ) -> TransportResult<PendingTransaction> {
        let tx_hash = self.client().prepare("eth_sendTransaction", (tx,)).await?;
        self.new_pending_transaction(tx_hash).await
    }

    /// Broadcasts a transaction's raw RLP bytes, returning a [`PendingTransaction`] that resolves
    /// once the transaction has been confirmed.
    async fn send_raw_transaction(&self, rlp_bytes: &[u8]) -> TransportResult<PendingTransaction> {
        let rlp_hex = hex::encode(rlp_bytes);
        let tx_hash = self.client().prepare("eth_sendRawTransaction", (rlp_hex,)).await?;
        self.new_pending_transaction(tx_hash).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProvider<N, T> {
    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.inner.client_ref()
    }

    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.inner.weak_client()
    }

    #[inline]
    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction> {
        RootProvider::new_pending_transaction(self, tx_hash).await
    }
}

// Internal implementation for [`chain_stream_poller`].
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProviderInner<N, T> {
    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.client_ref()
    }

    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.weak_client()
    }

    #[inline]
    async fn new_pending_transaction(&self, _tx_hash: B256) -> TransportResult<PendingTransaction> {
        unreachable!()
    }
}

#[cfg(test)]
struct _ObjectSafe<N: Network>(dyn Provider<N>);
