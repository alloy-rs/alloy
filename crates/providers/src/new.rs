use crate::heart::HeartbeatHandle;
use alloy_network::{Network, Transaction};
use alloy_primitives::{Address, BlockNumber, U64};
use alloy_rpc_client::{ClientRef, RpcClient, WeakClient};
use alloy_rpc_types::Block;
use alloy_transport::{BoxTransport, Transport, TransportResult};
use std::{
    borrow::Cow,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Weak},
};

/// A [`Provider`] in a [`Weak`] reference.
pub type WeakProvider<P> = Weak<P>;

/// A borrowed [`Provider`].
pub type ProviderRef<'a, P> = &'a P;

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProviderInner<N, T> {
    client: RpcClient<T>,

    heart: Option<HeartbeatHandle>,

    _network: PhantomData<fn() -> N>,
}

impl<N, T> RootProviderInner<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { client, heart: None, _network: PhantomData }
    }

    /// Get a weak reference to the RPC client.
    pub fn weak_client(&self) -> WeakClient<T> {
        self.client.get_weak()
    }

    /// Get a reference to the RPC client.
    pub fn client_ref(&self) -> ClientRef<'_, T> {
        self.client.get_ref()
    }

    /// Get a clone of the RPC client.
    pub fn client(&self) -> RpcClient<T> {
        self.client.clone()
    }

    /// Init the heartbeat
    async fn init_heartbeat(&mut self) {
        if self.heart.is_some() {
            return;
        }
        todo!()
    }
}

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<N, T> {
    pub inner: Arc<RootProviderInner<N, T>>,
}

impl<N, T> From<RootProviderInner<N, T>> for RootProvider<N, T> {
    fn from(inner: RootProviderInner<N, T>) -> Self {
        Self { inner: Arc::new(inner) }
    }
}

impl<N, T> Clone for RootProviderInner<N, T> {
    fn clone(&self) -> Self {
        Self { client: self.client.clone(), heart: self.heart.clone(), _network: PhantomData }
    }
}

impl<N, T> Deref for RootProvider<N, T> {
    type Target = RootProviderInner<N, T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
/// Provider is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Provider<N, Http>`.
pub trait Provider<N: Network, T: Transport = BoxTransport>: Send + Sync {
    /// Get a reference to the RPC client.
    fn client_ref(&self) -> ClientRef<'_, T>;

    /// Get a weak reference to the RPC client.
    fn weak_client(&self) -> WeakClient<T>;

    async fn estimate_gas(
        &self,
        tx: &N::TransactionRequest,
    ) -> TransportResult<alloy_primitives::U256>;

    /// Get the last block number available.
    async fn get_block_number(&self) -> TransportResult<BlockNumber>;

    /// Get the transaction count for an address. Used for finding the
    /// appropriate nonce.
    ///
    /// TODO: block number/hash/tag
    async fn get_transaction_count(
        &self,
        address: Address,
    ) -> TransportResult<alloy_primitives::U256>;

    /// Get a block by its number.
    ///
    /// TODO: Network associate
    async fn get_block_by_number(
        &self,
        number: BlockNumber,
        hydrate: bool,
    ) -> TransportResult<Block>;

    /// Populate the gas limit for a transaction.
    async fn populate_gas(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        let gas = self.estimate_gas(&*tx).await;

        gas.map(|gas| tx.set_gas_limit(gas.try_into().unwrap()))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProvider<N, T> {
    fn client_ref(&self) -> ClientRef<'_, T> {
        self.inner.client_ref()
    }

    fn weak_client(&self) -> WeakClient<T> {
        self.inner.weak_client()
    }

    async fn estimate_gas(
        &self,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<alloy_primitives::U256> {
        self.client.prepare("eth_estimateGas", Cow::Borrowed(tx)).await
    }

    async fn get_block_number(&self) -> TransportResult<BlockNumber> {
        self.client.prepare("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    async fn get_block_by_number(
        &self,
        number: BlockNumber,
        hydrate: bool,
    ) -> TransportResult<Block> {
        self.client
            .prepare("eth_getBlockByNumber", Cow::<(BlockNumber, bool)>::Owned((number, hydrate)))
            .await
    }

    async fn get_transaction_count(
        &self,
        address: Address,
    ) -> TransportResult<alloy_primitives::U256> {
        self.client
            .prepare(
                "eth_getTransactionCount",
                Cow::<(Address, String)>::Owned((address, "latest".to_string())),
            )
            .await
    }
}

#[cfg(test)]
mod test {
    use super::Provider;
    use alloy_network::Network;

    // checks that `Provider<N>` is object-safe
    fn __compile_check<N: Network>() -> Box<dyn Provider<N>> {
        unimplemented!()
    }
}
