use crate::{provider::forward, EthCall, Provider, ProviderLayer, RpcWithBlock};
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
///
/// All other [`Provider`] methods are forwarded to the inner provider unchanged, so that
/// layers stacked inside this one (for example a [`CacheProvider`](crate::layers::CacheProvider))
/// keep their overrides.
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

impl<P: Provider<N>, N: Network> forward::ForwardingProvider<N> for BlockIdProvider<P, N> {
    type Inner = P;

    #[inline(always)]
    fn inner_provider(&self) -> &P {
        &self.inner
    }

    fn call(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        self.inner.call(tx).block(self.block_id)
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

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        self.inner.estimate_gas(tx).block(self.block_id)
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

forward::impl_provider_forwarding!(
    impl[P: Provider<N>, N: Network] Provider<N> for BlockIdProvider<P, N>
);

#[cfg(test)]
mod tests {
    use super::BlockIdProvider;
    use crate::{Provider, ProviderCall, RootProvider};
    use alloy_eips::BlockId;
    use alloy_network::Ethereum;
    use alloy_primitives::{Address, BlockNumber, U128, U64};
    use alloy_rpc_client::{NoParams, RpcClient};
    use alloy_rpc_types_eth::{Filter, Log};
    use alloy_transport::{mock::Asserter, TransportResult};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    /// An inner provider that records which of its methods are reached, so we can assert that the
    /// `BlockIdProvider` wrapper forwards non-block methods instead of bypassing it.
    #[derive(Clone, Debug)]
    struct Recorder {
        inner: RootProvider<Ethereum>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Recorder {
        fn record(&self, method: &'static str) {
            self.calls.lock().unwrap().push(method);
        }
    }

    #[async_trait]
    impl Provider<Ethereum> for Recorder {
        fn root(&self) -> &RootProvider<Ethereum> {
            self.inner.root()
        }

        fn get_accounts(&self) -> ProviderCall<NoParams, Vec<Address>> {
            self.record("get_accounts");
            self.inner.get_accounts()
        }

        fn get_blob_base_fee(&self) -> ProviderCall<NoParams, U128, u128> {
            self.record("get_blob_base_fee");
            self.inner.get_blob_base_fee()
        }

        fn get_block_number(&self) -> ProviderCall<NoParams, U64, BlockNumber> {
            self.record("get_block_number");
            self.inner.get_block_number()
        }

        fn get_chain_id(&self) -> ProviderCall<NoParams, U64, u64> {
            self.record("get_chain_id");
            self.inner.get_chain_id()
        }

        async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
            self.record("get_logs");
            self.inner.get_logs(filter).await
        }
    }

    #[tokio::test]
    async fn forwards_non_block_methods_to_inner() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recorder = Recorder {
            inner: RootProvider::new(RpcClient::mocked(Asserter::new())),
            calls: calls.clone(),
        };
        let provider = BlockIdProvider::new(recorder, BlockId::latest());

        // The empty mock fails every request; only the fact that the call reached the inner
        // recorder matters.
        let _ = provider.get_accounts().await;
        let _ = provider.get_blob_base_fee().await;
        let _ = provider.get_block_number().await;
        let _ = provider.get_chain_id().await;
        let _ = provider.get_logs(&Filter::default()).await;

        assert_eq!(
            *calls.lock().unwrap(),
            ["get_accounts", "get_blob_base_fee", "get_block_number", "get_chain_id", "get_logs"]
        );
    }
}
