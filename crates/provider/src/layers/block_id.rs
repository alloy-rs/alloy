use crate::{EthCall, Provider, ProviderLayer, RootProvider, RpcWithBlock};
use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, StorageKey, StorageValue, U256, U64};
use alloy_rpc_types_eth::{
    simulate::{SimulatePayload, SimulatedBlock},
    AccessListResult, EIP1186AccountProofResponse,
};
use std::marker::PhantomData;

/// A layer that sets a default [`BlockId`] for RPC methods that support block parameters.
///
/// This layer affects the following methods:
/// - `eth_call`
/// - `eth_estimateGas`
/// - `eth_simulateV1`
/// - `eth_createAccessList`
/// - `eth_getAccountInfo`
/// - `eth_getAccount`
/// - `eth_getBalance`
/// - `eth_getCode`
/// - `eth_getProof`
/// - `eth_getStorageAt`
/// - `eth_getTransactionCount`
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

    fn simulate<'req>(
        &self,
        payload: &'req SimulatePayload,
    ) -> RpcWithBlock<&'req SimulatePayload, Vec<SimulatedBlock<N::BlockResponse>>> {
        self.inner.simulate(payload).block_id(self.block_id)
    }

    fn create_access_list<'a>(
        &self,
        request: &'a N::TransactionRequest,
    ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult> {
        self.inner.create_access_list(request).block_id(self.block_id)
    }

    fn get_account_info(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, alloy_rpc_types_eth::AccountInfo> {
        self.inner.get_account_info(address).block_id(self.block_id)
    }

    fn get_account(&self, address: Address) -> RpcWithBlock<Address, alloy_consensus::TrieAccount> {
        self.inner.get_account(address).block_id(self.block_id)
    }

    fn get_balance(&self, address: Address) -> RpcWithBlock<Address, U256, U256> {
        self.inner.get_balance(address).block_id(self.block_id)
    }

    fn get_code_at(&self, address: Address) -> RpcWithBlock<Address, Bytes> {
        self.inner.get_code_at(address).block_id(self.block_id)
    }

    fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
    ) -> RpcWithBlock<(Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        self.inner.get_proof(address, keys).block_id(self.block_id)
    }

    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> RpcWithBlock<(Address, U256), StorageValue> {
        self.inner.get_storage_at(address, key).block_id(self.block_id)
    }

    fn get_transaction_count(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64> {
        self.inner.get_transaction_count(address).block_id(self.block_id)
    }
}
