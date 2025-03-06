//! Mock Provider Layer

use std::{collections::VecDeque, sync::Arc};

use crate::{utils, EthCallMany, EthGetBlock};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::{ErrorPayload, RpcRecv, RpcSend};
use alloy_network::Network;
use alloy_primitives::{
    Address, BlockHash, Bytes, StorageKey, StorageValue, TxHash, U128, U256, U64,
};
use alloy_rpc_client::NoParams;
use alloy_rpc_types_eth::{
    AccessListResult, Bundle, EIP1186AccountProofResponse, EthCallResponse, Filter, Log,
};
use alloy_transport::{TransportError, TransportErrorKind, TransportResult};
use parking_lot::RwLock;
use serde::Serialize;

use crate::{Caller, EthCall, Provider, ProviderCall, ProviderLayer, RpcWithBlock};

/// A mock provider layer that returns responses that have been pushed to the [`Asserter`].
#[derive(Debug, Clone)]
pub struct MockLayer {
    asserter: Asserter,
}

impl MockLayer {
    /// Instantiate a new mock layer with the given [`Asserter`].
    pub fn new(asserter: Asserter) -> Self {
        Self { asserter }
    }
}

impl<P, N> ProviderLayer<P, N> for MockLayer
where
    P: Provider<N>,
    N: Network,
{
    type Provider = MockProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        MockProvider::new(inner, self.asserter.clone())
    }
}

/// Container for pushing responses into the [`MockProvider`].
#[derive(Debug, Clone, Default)]
pub struct Asserter {
    responses: Arc<RwLock<VecDeque<MockResponse>>>,
}

impl Asserter {
    /// Instantiate a new asserter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a successful response into the queue.
    pub fn push_success<R: Serialize>(&self, response: R) {
        self.responses
            .write()
            .push_back(MockResponse::Success(serde_json::to_value(response).unwrap()));
    }

    /// Push a server error payload into the queue.
    pub fn push_error(&self, error: ErrorPayload) {
        self.push_err(TransportError::err_resp(error));
    }

    /// Insert an error response into the queue.
    pub fn push_err(&self, err: TransportError) {
        self.responses.write().push_back(MockResponse::Err(err));
    }

    /// Pop front to get the next response from the queue.
    pub fn pop_response(&self) -> Option<MockResponse> {
        self.responses.write().pop_front()
    }

    /// Helper function to get and deserialize the next response from the asserter
    pub fn pop_deser_response<T>(&self) -> Result<T, TransportError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let value = self.pop_response().ok_or(TransportErrorKind::custom(MockError::EmptyQueue));

        match value {
            Ok(MockResponse::Success(value)) => serde_json::from_value(value)
                .map_err(|e| TransportErrorKind::custom(MockError::DeserError(e.to_string()))),
            Ok(MockResponse::Err(err)) | Err(err) => Err(err),
        }
    }
}

/// A mock response that can be pushed into the asserter.
#[derive(Debug)]
pub enum MockResponse {
    /// A successful response that will be deserialized into the expected type.
    Success(serde_json::Value),
    /// An error response.
    Err(TransportError),
}

/// A [`MockProvider`] error.
#[derive(Debug, thiserror::Error)]
pub enum MockError {
    /// An error occurred while deserializing the response from asserter into the expected type.
    #[error("could not deserialize response {0}")]
    DeserError(String),
    /// The response queue is empty.
    #[error("empty response queue")]
    EmptyQueue,
}

/// A mock provider implementation that returns responses from the [`Asserter`].
#[derive(Debug, Clone)]
pub struct MockProvider<P: Provider<N>, N: Network> {
    /// Inner dummy provider.
    inner: P,
    /// The [`Asserter`] to which response are pushed using [`Asserter::push_success`].
    ///
    /// Responses are popped from the asserter in the order they were pushed.
    asserter: Asserter,
    _network: std::marker::PhantomData<N>,
}

impl<P, N> MockProvider<P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Instantiate a new mock provider.
    pub fn new(inner: P, asserter: Asserter) -> Self {
        Self { inner, asserter, _network: std::marker::PhantomData }
    }

    /// Return a reference to the asserter.
    pub fn asserter(&self) -> &Asserter {
        &self.asserter
    }

    /// Insert a successful response into the queue.
    pub fn push_success<R: Serialize>(&self, response: R) {
        self.asserter.push_success(response);
    }

    /// Push a server error payload into the queue.
    pub fn push_error(&self, error: ErrorPayload) {
        self.asserter.push_error(error);
    }

    /// Helper function to get and deserialize the next response from the asserter
    fn next_response<T>(&self) -> Result<T, TransportError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.asserter.pop_deser_response()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<P, N> Provider<N> for MockProvider<P, N>
where
    P: Provider<N>,
    N: Network,
{
    fn root(&self) -> &crate::RootProvider<N> {
        self.inner.root()
    }

    fn get_accounts(&self) -> ProviderCall<NoParams, Vec<Address>> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_block_number(&self) -> ProviderCall<NoParams, U64, u64> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_blob_base_fee(&self) -> ProviderCall<NoParams, U128, u128> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_chain_id(&self) -> ProviderCall<NoParams, U64, u64> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn call<'req>(&self, tx: N::TransactionRequest) -> EthCall<N, Bytes> {
        EthCall::call(self.asserter.clone(), tx)
    }

    fn call_many<'req>(
        &self,
        bundles: &'req Vec<Bundle>,
    ) -> EthCallMany<'req, N, Vec<Vec<EthCallResponse>>> {
        EthCallMany::new(self.asserter.clone(), bundles)
    }

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        EthCall::gas_estimate(self.asserter.clone(), tx).map_resp(utils::convert_u64)
    }

    fn create_access_list<'a>(
        &self,
        _request: &'a N::TransactionRequest,
    ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response();
            ProviderCall::Ready(Some(res))
        })
    }

    fn get_balance(&self, _address: Address) -> RpcWithBlock<Address, U256, U256> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response();
            ProviderCall::Ready(Some(res))
        })
    }

    fn get_gas_price(&self) -> ProviderCall<NoParams, U128, u128> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_account(&self, _address: Address) -> RpcWithBlock<Address, alloy_consensus::Account> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response();
            ProviderCall::Ready(Some(res))
        })
    }

    fn get_block(&self, block: BlockId) -> EthGetBlock<N::BlockResponse> {
        let asserter = self.asserter.clone();
        EthGetBlock::new_provider(
            block,
            Box::new(move |_kind| {
                let res = asserter.pop_deser_response();
                ProviderCall::Ready(Some(res))
            }),
        )
    }

    fn get_block_by_number(&self, number: BlockNumberOrTag) -> EthGetBlock<N::BlockResponse> {
        let asserter = self.asserter.clone();
        EthGetBlock::new_provider(
            number.into(),
            Box::new(move |_kind| {
                let res = asserter.pop_deser_response();
                ProviderCall::Ready(Some(res))
            }),
        )
    }

    fn get_block_by_hash(&self, hash: BlockHash) -> EthGetBlock<N::BlockResponse> {
        let asserter = self.asserter.clone();
        EthGetBlock::new_provider(
            hash.into(),
            Box::new(move |_kind| {
                let res = asserter.pop_deser_response();
                ProviderCall::Ready(Some(res))
            }),
        )
    }

    async fn get_block_transaction_count_by_hash(
        &self,
        _hash: BlockHash,
    ) -> TransportResult<Option<u64>> {
        let res = self.next_response::<Option<U64>>()?;
        Ok(res.map(utils::convert_u64))
    }

    async fn get_block_transaction_count_by_number(
        &self,
        _block_number: BlockNumberOrTag,
    ) -> TransportResult<Option<u64>> {
        let res = self.next_response::<Option<U64>>()?;
        Ok(res.map(utils::convert_u64))
    }

    fn get_block_receipts(
        &self,
        _block: BlockId,
    ) -> ProviderCall<(BlockId,), Option<Vec<N::ReceiptResponse>>> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_code_at(&self, _address: Address) -> RpcWithBlock<Address, Bytes> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response();
            ProviderCall::Ready(Some(res))
        })
    }

    async fn get_logs(&self, _filter: &Filter) -> TransportResult<Vec<Log>> {
        self.next_response()
    }

    fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<StorageKey>,
    ) -> RpcWithBlock<(Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response();
            ProviderCall::Ready(Some(res))
        })
    }

    fn get_storage_at(
        &self,
        _address: Address,
        _key: U256,
    ) -> RpcWithBlock<(Address, U256), StorageValue> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response();
            ProviderCall::Ready(Some(res))
        })
    }

    fn get_transaction_by_hash(
        &self,
        _hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::TransactionResponse>> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_raw_transaction_by_hash(&self, _hash: TxHash) -> ProviderCall<(TxHash,), Option<Bytes>> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    fn get_transaction_count(
        &self,
        _address: Address,
    ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64> {
        let asserter = self.asserter.clone();
        RpcWithBlock::new_provider(move |_block_id| {
            let res = asserter.pop_deser_response::<U64>();
            let res = res.map(utils::convert_u64);
            ProviderCall::Ready(Some(res))
        })
    }

    fn get_transaction_receipt(
        &self,
        _hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::ReceiptResponse>> {
        ProviderCall::Ready(Some(self.next_response()))
    }

    async fn get_uncle(
        &self,
        tag: BlockId,
        _idx: u64,
    ) -> TransportResult<Option<N::BlockResponse>> {
        match tag {
            BlockId::Hash(_) | BlockId::Number(_) => self.next_response(),
        }
    }

    /// Gets the number of uncles for the block specified by the tag [BlockId].
    async fn get_uncle_count(&self, tag: BlockId) -> TransportResult<u64> {
        match tag {
            BlockId::Hash(_) | BlockId::Number(_) => {
                self.next_response::<U64>().map(utils::convert_u64)
            }
        }
    }
}

/// [`Caller`] implementation for the [`Asserter`] to `eth_call` ops in the [`MockProvider`].
impl<N: Network, Resp: RpcRecv> Caller<N, Resp> for Asserter {
    fn call(
        &self,
        _params: crate::EthCallParams<N>,
    ) -> TransportResult<ProviderCall<crate::EthCallParams<N>, Resp>> {
        provider_eth_call(self)
    }

    fn call_many(
        &self,
        _params: crate::EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<crate::EthCallManyParams<'static>, Resp>> {
        provider_eth_call(self)
    }

    fn estimate_gas(
        &self,
        _params: crate::EthCallParams<N>,
    ) -> TransportResult<ProviderCall<crate::EthCallParams<N>, Resp>> {
        provider_eth_call(self)
    }
}

fn provider_eth_call<Params: RpcSend, Resp: RpcRecv>(
    asserter: &Asserter,
) -> TransportResult<ProviderCall<Params, Resp>> {
    let value = asserter.pop_response().ok_or(TransportErrorKind::custom(MockError::EmptyQueue));

    let res = match value {
        Ok(MockResponse::Success(value)) => serde_json::from_value(value)
            .map_err(|e| TransportErrorKind::custom(MockError::DeserError(e.to_string()))),
        Ok(MockResponse::Err(err)) | Err(err) => Err(err),
    };

    Ok(ProviderCall::Ready(Some(res)))
}

#[cfg(test)]
mod tests {
    use alloy_primitives::bytes;
    use alloy_rpc_types_eth::TransactionRequest;

    use super::*;
    use crate::ProviderBuilder;

    #[tokio::test]
    async fn test_mock() {
        let (provider, asserter) = ProviderBuilder::mocked();

        asserter.push_success(21965802);
        asserter.push_success(21965803);
        asserter.push_err(TransportError::NullResp);

        let response = provider.get_block_number().await.unwrap();
        assert_eq!(response, 21965802);

        let response = provider.get_block_number().await.unwrap();
        assert_eq!(response, 21965803);

        let err_res = provider.get_block_number().await.unwrap_err();
        assert!(matches!(err_res, TransportError::NullResp));

        let response = provider.get_block_number().await.unwrap_err();
        assert!(response.to_string().contains("empty response queue"));

        asserter.push_success(vec![Address::with_last_byte(1), Address::with_last_byte(2)]);
        let response = provider.get_accounts().await.unwrap();
        assert_eq!(response, vec![Address::with_last_byte(1), Address::with_last_byte(2)]);

        let call_resp = bytes!("12345678");

        asserter.push_success(call_resp.clone());
        let tx = TransactionRequest::default();
        let response = provider.call(tx).await.unwrap();

        assert_eq!(response, call_resp);

        let assert_bal = U256::from(123456780);
        asserter.push_success(assert_bal);

        let response = provider.get_balance(Address::default()).await.unwrap();
        assert_eq!(response, assert_bal);
    }
}
