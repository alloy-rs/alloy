//! A Multicall Builder

use alloy_network::{Ethereum, Network};
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types_eth::BlockId;

use crate::{CallBuilder, Result};

/// A Multicall
#[derive(Debug)]
pub struct Multicall<P: Provider<N>, N: Network = Ethereum> {
    provider: P,
    address: Option<Address>,
    block: Option<BlockId>,
    _pd: std::marker::PhantomData<N>,
}

impl<P, N> Multicall<P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Create a new [`Multicall`] instance
    pub fn new(provider: P) -> Self {
        Self { provider, address: None, block: None, _pd: Default::default() }
    }

    /// Set the address of the `Multicall3` contract
    pub fn address(mut self, address: Address) -> Self {
        self.address = Some(address);
        self
    }

    /// Set the [`BlockId`] to use for the call
    pub fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }

    /// Add a call to the batch
    pub fn add<T, D>(&mut self, call: CallBuilder<T, P, D, N>) {
        let tx = call.into_transaction_request();
        self.add_tx(tx);
    }

    /// Add a transaction to the batch
    pub fn add_tx(&mut self, tx: N::TransactionRequest) {}

    /// Execute via Multicall3's aggregate/aggregate3 function
    async fn call(&self) -> Result<()> {
        unimplemented!()
    }

    /// Send a batch of txs via Multicall3's aggregate3Value function
    async fn send(&self) -> Result<()> {
        unimplemented!()
    }
}
