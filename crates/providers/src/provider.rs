//! Alloy main Provider abstraction.

use alloy_primitives::{Address, BlockHash, Bytes, TxHash, U256, U64};
use alloy_rpc_client::{ClientBuilder, RpcClient};
use alloy_rpc_types::{
    Block, BlockId, BlockNumberOrTag, FeeHistory, Filter, Log, RpcBlockHash, SyncStatus,
    Transaction, TransactionReceipt, TransactionRequest,
};
use alloy_transport::{BoxTransport, RpcResult, Transport, TransportError};
use alloy_transport_http::Http;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::borrow::Cow;
use thiserror::Error;

use crate::utils::{self, EstimatorFunction};

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ClientError {
    #[error("Could not parse URL")]
    ParseError,
    #[error("Unsupported Tag")]
    UnsupportedBlockIdError,
}

/// An abstract provider for interacting with the [Ethereum JSON RPC
/// API](https://github.com/ethereum/wiki/wiki/JSON-RPC). Must be instantiated
/// with a transport which implements the [Transport] trait.
#[derive(Debug)]
pub struct Provider<T: Transport = BoxTransport> {
    inner: RpcClient<T>,
    from: Option<Address>,
}

// Simple JSON-RPC bindings.
// In the future, this will be replaced by a Provider trait,
// but as the interface is not stable yet, we define the bindings ourselves
// until we can use the trait and the client abstraction that will use it.
impl<T: Transport + Clone + Send + Sync> Provider<T> {
    /// Gets the transaction count of the corresponding address.
    pub async fn get_transaction_count(
        &self,
        address: Address,
    ) -> RpcResult<alloy_primitives::U256, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getTransactionCount",
                Cow::<(Address, &'static str)>::Owned((address, "latest")),
            )
            .await
    }

    /// Gets the last block number available.
    pub async fn get_block_number(&self) -> RpcResult<U64, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_blockNumber", Cow::<()>::Owned(()))
            .await
    }

    /// Gets the balance of the account at the specified tag, which defaults to latest.
    pub async fn get_balance(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> RpcResult<U256, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getBalance",
                Cow::<(Address, BlockId)>::Owned((
                    address,
                    tag.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)),
                )),
            )
            .await
    }

    /// Gets a block by its [BlockHash], with full transactions or only hashes.
    pub async fn get_block_by_hash(
        &self,
        hash: BlockHash,
        full: bool,
    ) -> RpcResult<Option<Block>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getBlockByHash",
                Cow::<(BlockHash, bool)>::Owned((hash, full)),
            )
            .await
    }

    /// Gets a block by [BlockNumberOrTag], with full transactions or only hashes.
    pub async fn get_block_by_number<B: Into<BlockNumberOrTag> + Send + Sync>(
        &self,
        number: B,
        full: bool,
    ) -> RpcResult<Option<Block>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getBlockByNumber",
                Cow::<(BlockNumberOrTag, bool)>::Owned((number.into(), full)),
            )
            .await
    }

    /// Gets the chain ID.
    pub async fn get_chain_id(&self) -> RpcResult<U64, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_chainId", Cow::<()>::Owned(()))
            .await
    }
    /// Gets the bytecode located at the corresponding [Address].
    pub async fn get_code_at<B: Into<BlockId> + Send + Sync>(
        &self,
        address: Address,
        tag: B,
    ) -> RpcResult<Bytes, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getCode",
                Cow::<(Address, BlockId)>::Owned((address, tag.into())),
            )
            .await
    }

    /// Gets a [Transaction] by its [TxHash].
    pub async fn get_transaction_by_hash(
        &self,
        hash: TxHash,
    ) -> RpcResult<Transaction, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getTransactionByHash",
                // Force alloy-rs/alloy to encode this an array of strings,
                // even if we only need to send one hash.
                Cow::<Vec<TxHash>>::Owned(vec![hash]),
            )
            .await
    }

    /// Retrieves a [`Vec<Log>`] with the given [Filter].
    pub async fn get_logs(
        &self,
        filter: Filter,
    ) -> RpcResult<Vec<Log>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_getLogs", Cow::<Vec<Filter>>::Owned(vec![filter]))
            .await
    }

    /// Gets the accounts in the remote node. This is usually empty unless you're using a local node.
    pub async fn get_accounts(&self) -> RpcResult<Vec<Address>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_accounts", Cow::<()>::Owned(()))
            .await
    }

    /// Gets the current gas price.
    pub async fn get_gas_price(&self) -> RpcResult<U256, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_gasPrice", Cow::<()>::Owned(()))
            .await
    }

    /// Gets a [TransactionReceipt] if it exists, by its [TxHash].
    pub async fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> RpcResult<Option<TransactionReceipt>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getTransactionReceipt",
                Cow::<Vec<TxHash>>::Owned(vec![hash]),
            )
            .await
    }

    /// Returns a collection of historical gas information [FeeHistory] which
    /// can be used to calculate the EIP1559 fields `maxFeePerGas` and `maxPriorityFeePerGas`.
    pub async fn get_fee_history<B: Into<BlockNumberOrTag> + Send + Sync>(
        &self,
        block_count: U256,
        last_block: B,
        reward_percentiles: &[f64],
    ) -> RpcResult<FeeHistory, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_feeHistory",
                Cow::<(U256, BlockNumberOrTag, Vec<f64>)>::Owned((
                    block_count,
                    last_block.into(),
                    reward_percentiles.to_vec(),
                )),
            )
            .await
    }

    /// Gets the selected block [BlockNumberOrTag] receipts.
    pub async fn get_block_receipts(
        &self,
        block: BlockNumberOrTag,
    ) -> RpcResult<Vec<TransactionReceipt>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_getBlockReceipts",
                Cow::<BlockNumberOrTag>::Owned(block),
            )
            .await
    }

    /// Gets an uncle block through the tag [BlockId] and index [U64].
    pub async fn get_uncle<B: Into<BlockId> + Send + Sync>(
        &self,
        tag: B,
        idx: U64,
    ) -> RpcResult<Option<Block>, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        let tag = tag.into();
        match tag {
            BlockId::Hash(hash) => {
                self.inner
                    .prepare(
                        "eth_getUncleByBlockHashAndIndex",
                        Cow::<(RpcBlockHash, U64)>::Owned((hash, idx)),
                    )
                    .await
            }
            BlockId::Number(number) => {
                self.inner
                    .prepare(
                        "eth_getUncleByBlockNumberAndIndex",
                        Cow::<(BlockNumberOrTag, U64)>::Owned((number, idx)),
                    )
                    .await
            }
        }
    }

    /// Gets syncing info.
    pub async fn syncing(&self) -> RpcResult<SyncStatus, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_syncing", Cow::<()>::Owned(()))
            .await
    }

    /// Execute a smart contract call with [TransactionRequest] without publishing a transaction.
    pub async fn call(
        &self,
        tx: TransactionRequest,
        block: Option<BlockId>,
    ) -> RpcResult<Bytes, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "eth_call",
                Cow::<(TransactionRequest, BlockId)>::Owned((
                    tx,
                    block.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)),
                )),
            )
            .await
    }

    /// Estimate the gas needed for a transaction.
    pub async fn estimate_gas(
        &self,
        tx: TransactionRequest,
        block: Option<BlockId>,
    ) -> RpcResult<Bytes, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        if let Some(block_id) = block {
            let params = Cow::<(TransactionRequest, BlockId)>::Owned((tx, block_id));
            self.inner.prepare("eth_estimateGas", params).await
        } else {
            let params = Cow::<TransactionRequest>::Owned(tx);
            self.inner.prepare("eth_estimateGas", params).await
        }
    }

    /// Sends an already-signed transaction.
    pub async fn send_raw_transaction(
        &self,
        tx: Bytes,
    ) -> RpcResult<TxHash, Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare("eth_sendRawTransaction", Cow::<Bytes>::Owned(tx))
            .await
    }

    /// Estimates the EIP1559 `maxFeePerGas` and `maxPriorityFeePerGas` fields.
    /// Receives an optional [EstimatorFunction] that can be used to modify
    /// how to estimate these fees.
    pub async fn estimate_eip1559_fees(
        &self,
        estimator: Option<EstimatorFunction>,
    ) -> RpcResult<(U256, U256), Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        let base_fee_per_gas = match self
            .get_block_by_number(BlockNumberOrTag::Latest, false)
            .await?
        {
            Ok(Some(block)) => match block.header.base_fee_per_gas {
                Some(base_fee_per_gas) => base_fee_per_gas,
                None => {
                    return RpcResult::Err(TransportError::Custom("EIP-1559 not activated".into()))
                }
            },
            Ok(None) => {
                return RpcResult::Err(TransportError::Custom("Latest block not found".into()))
            }
            Err(err) => return RpcResult::Ok(Err(err)),
        };

        let fee_history = match self
            .get_fee_history(
                U256::from(utils::EIP1559_FEE_ESTIMATION_PAST_BLOCKS),
                BlockNumberOrTag::Latest,
                &[utils::EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await?
        {
            Ok(fee_history) => fee_history,
            Err(err) => return Ok(Err(err)),
        };

        // use the provided fee estimator function, or fallback to the default implementation.
        let (max_fee_per_gas, max_priority_fee_per_gas) = if let Some(es) = estimator {
            es(base_fee_per_gas, fee_history.reward.unwrap_or_default())
        } else {
            utils::eip1559_default_estimator(
                base_fee_per_gas,
                fee_history.reward.unwrap_or_default(),
            )
        };

        Ok(Ok((max_fee_per_gas, max_priority_fee_per_gas)))
    }

    #[cfg(feature = "anvil")]
    pub async fn set_code(
        &self,
        address: Address,
        code: &'static str,
    ) -> RpcResult<(), Box<RawValue>, TransportError>
    where
        Self: Sync,
    {
        self.inner
            .prepare(
                "anvil_setCode",
                Cow::<(Address, &'static str)>::Owned((address, code)),
            )
            .await
    }

    pub fn with_sender(mut self, from: Address) -> Self {
        self.from = Some(from);
        self
    }

    pub fn inner(&self) -> &RpcClient<T> {
        &self.inner
    }
}

// HTTP Transport Provider implementation
impl Provider<Http<Client>> {
    pub fn new(url: &str) -> Result<Self, ClientError> {
        let url = url.parse().map_err(|_e| ClientError::ParseError)?;
        let inner = ClientBuilder::default().reqwest_http(url);

        Ok(Self { inner, from: None })
    }
}

impl TryFrom<&str> for Provider<Http<Client>> {
    type Error = ClientError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Provider::new(value)
    }
}

impl TryFrom<String> for Provider<Http<Client>> {
    type Error = ClientError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Provider::try_from(value.as_str())
    }
}

impl<'a> TryFrom<&'a String> for Provider<Http<Client>> {
    type Error = ClientError;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Provider::try_from(value.as_str())
    }
}

#[cfg(test)]
mod providers_test {
    use crate::{provider::Provider, utils};
    use alloy_primitives::{address, b256, U256, U64};
    use alloy_rpc_types::{BlockNumberOrTag, Filter};

    use ethers_core::utils::Anvil;

    #[tokio::test]
    async fn gets_block_number() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let num = provider.get_block_number().await.unwrap().unwrap();
        assert_eq!(U64::ZERO, num)
    }

    #[tokio::test]
    async fn gets_transaction_count() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let count = provider
            .get_transaction_count(address!("328375e18E7db8F1CA9d9bA8bF3E9C94ee34136A"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count, U256::from(0));
    }

    #[tokio::test]
    async fn gets_block_by_hash() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider
            .get_block_by_number(tag, true)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let hash = block.header.hash.unwrap();
        let block = provider
            .get_block_by_hash(hash, true)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(block.header.hash.unwrap(), hash);
    }

    #[tokio::test]
    async fn gets_block_by_number_full() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider
            .get_block_by_number(tag, true)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_block_by_number() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let num = 0;
        let tag: BlockNumberOrTag = num.into();
        let block = provider
            .get_block_by_number(tag, true)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(block.header.number.unwrap(), U256::from(num));
    }

    #[tokio::test]
    async fn gets_chain_id() {
        let anvil = Anvil::new().args(vec!["--chain-id", "13371337"]).spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let chain_id = provider.get_chain_id().await.unwrap().unwrap();
        assert_eq!(chain_id, U64::from(13371337));
    }

    #[tokio::test]
    #[cfg(feature = "anvil")]
    async fn gets_code_at() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        // Set the code
        let addr = alloy_primitives::Address::with_last_byte(16);
        provider.set_code(addr, "0xbeef").await.unwrap().unwrap();
        let _code = provider
            .get_code_at(
                addr,
                crate::provider::BlockId::Number(alloy_rpc_types::BlockNumberOrTag::Latest),
            )
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn gets_transaction_by_hash() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let tx = provider
            .get_transaction_by_hash(b256!(
                "5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95"
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            tx.block_hash.unwrap(),
            b256!("b20e6f35d4b46b3c4cd72152faec7143da851a0dc281d390bdd50f58bfbdb5d3")
        );
        assert_eq!(tx.block_number.unwrap(), U256::from(4571819));
    }

    #[tokio::test]
    #[ignore]
    async fn gets_logs() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let filter = Filter::new()
            .at_block_hash(b256!(
                "b20e6f35d4b46b3c4cd72152faec7143da851a0dc281d390bdd50f58bfbdb5d3"
            ))
            .event_signature(b256!(
                "e1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"
            ));
        let logs = provider.get_logs(filter).await.unwrap().unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    #[ignore]
    async fn gets_tx_receipt() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let receipt = provider
            .get_transaction_receipt(b256!(
                "5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95"
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(receipt.is_some());
        let receipt = receipt.unwrap();
        assert_eq!(
            receipt.transaction_hash.unwrap(),
            b256!("5c03fab9114ceb98994b43892ade87ddfd9ae7e8f293935c3bd29d435dc9fd95")
        );
    }

    #[tokio::test]
    async fn gets_fee_history() {
        let anvil = Anvil::new().spawn();
        let provider = Provider::new(&anvil.endpoint()).unwrap();
        let block_number = provider.get_block_number().await.unwrap().unwrap();
        let fee_history = provider
            .get_fee_history(
                U256::from(utils::EIP1559_FEE_ESTIMATION_PAST_BLOCKS),
                BlockNumberOrTag::Number(block_number.to()),
                &[utils::EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fee_history.oldest_block, U256::ZERO);
    }
}
