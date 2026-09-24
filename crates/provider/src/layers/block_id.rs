use crate::{EthCall, Provider, ProviderLayer, RootProvider, RpcWithBlock};
use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, StorageKey, StorageValue, U256, U64};
use alloy_rpc_types_eth::{
    simulate::{SimulatePayload, SimulatedBlock},
    AccessListResult, EIP1186AccountProofResponse, StorageValuesRequest, StorageValuesResponse,
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
        self.inner.call(tx).block(self.block_id)
    }

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        self.inner.estimate_gas(tx).block(self.block_id)
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

    fn get_storage_values(
        &self,
        requests: StorageValuesRequest,
    ) -> RpcWithBlock<(StorageValuesRequest,), StorageValuesResponse> {
        self.inner.get_storage_values(requests).block_id(self.block_id)
    }

    fn get_transaction_count(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64> {
        self.inner.get_transaction_count(address).block_id(self.block_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_network::Ethereum;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_transport::mock::Asserter;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct CallCounts {
        calls: AtomicUsize,
        gas_estimates: AtomicUsize,
    }

    #[derive(Clone)]
    struct CountingProvider {
        inner: RootProvider,
        counts: Arc<CallCounts>,
    }

    impl Provider<Ethereum> for CountingProvider {
        fn root(&self) -> &RootProvider {
            &self.inner
        }

        fn call(&self, tx: TransactionRequest) -> EthCall<Ethereum, Bytes> {
            self.counts.calls.fetch_add(1, Ordering::Relaxed);
            self.inner.call(tx)
        }

        fn estimate_gas(&self, tx: TransactionRequest) -> EthCall<Ethereum, U64, u64> {
            self.counts.gas_estimates.fetch_add(1, Ordering::Relaxed);
            self.inner.estimate_gas(tx)
        }
    }

    #[test]
    fn forwards_call_and_estimate_gas_to_inner_provider() {
        let inner = ProviderBuilder::new()
            .disable_recommended_fillers()
            .connect_mocked_client(Asserter::new());
        let counts = Arc::new(CallCounts::default());
        let provider = BlockIdProvider::new(
            CountingProvider { inner, counts: counts.clone() },
            BlockId::latest(),
        );

        let _ = provider.call(TransactionRequest::default());
        let _ = provider.estimate_gas(TransactionRequest::default());

        assert_eq!(counts.calls.load(Ordering::Relaxed), 1);
        assert_eq!(counts.gas_estimates.load(Ordering::Relaxed), 1);
    }
}
