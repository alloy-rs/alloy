//! This module extends the Ethereum JSON-RPC provider with the Tenderly namespace's RPC methods.
use crate::Provider;
use alloy_eips::BlockNumberOrTag;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, TxHash, B256};
use alloy_rpc_types_eth::{state::StateOverride, BlockOverrides};
use alloy_rpc_types_tenderly::{
    TenderlyDecodeInputResult, TenderlyEstimateGasResult, TenderlyFunctionSignature,
    TenderlyGasPriceResult, TenderlySimulationResult, TenderlyStorageChange,
    TenderlyStorageQueryParams, TenderlyTransactionRangeParams,
};
use alloy_transport::TransportResult;

/// Tenderly namespace rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait TenderlyApi<N: Network>: Send + Sync {
    /// Simulates a transaction as it would execute on the given block, allowing overrides of state
    /// variables and balances of all accounts
    async fn tenderly_simulate_transaction(
        &self,
        tx: N::TransactionRequest,
        block: BlockNumberOrTag,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<BlockOverrides>,
    ) -> TransportResult<TenderlySimulationResult>;

    /// Simulates a transaction as it would execute on the given block, allowing overrides of state
    /// variables and balances of all accounts
    async fn tenderly_simulate_bundle(
        &self,
        txs: &[N::TransactionRequest],
        block: BlockNumberOrTag,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<BlockOverrides>,
    ) -> TransportResult<Vec<TenderlySimulationResult>>;

    /// Replays transaction on the blockchain and provides information about the execution.
    async fn tenderly_trace_transaction(
        &self,
        txs: &[TxHash],
    ) -> TransportResult<TenderlySimulationResult>;

    /// Estimates the gas required for a transaction to execute.
    async fn tenderly_estimate_gas(
        &self,
        tx: N::TransactionRequest,
        block: BlockNumberOrTag,
    ) -> TransportResult<TenderlyEstimateGasResult>;

    /// Gets the current gas price information with tiered pricing.
    async fn tenderly_gas_price(&self) -> TransportResult<TenderlyGasPriceResult>;

    /// Suggests gas fee information with tiered pricing.
    async fn tenderly_suggest_gas_fee(&self) -> TransportResult<TenderlyGasPriceResult>;

    /// Estimates the gas required for a bundle of transactions to execute.
    async fn tenderly_estimate_gas_bundle(
        &self,
        txs: &[N::TransactionRequest],
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<TenderlyEstimateGasResult>>;

    /// Heuristically decodes external function calls. Use for unverified contracts.
    async fn tenderly_decode_input(
        &self,
        call_data: Bytes,
    ) -> TransportResult<TenderlyDecodeInputResult>;

    /// Heuristically decodes custom errors. Use for unverified contracts.
    async fn tenderly_decode_error(
        &self,
        error_data: Bytes,
    ) -> TransportResult<TenderlyDecodeInputResult>;

    /// Retrieve function interface based on 4-byte function selector.
    async fn tenderly_function_signatures(
        &self,
        selector: Bytes,
    ) -> TransportResult<Vec<TenderlyFunctionSignature>>;

    /// Heuristically decodes emitted events. Use for unverified contracts.
    async fn tenderly_decode_event(
        &self,
        topics: Vec<B256>,
        data: Bytes,
    ) -> TransportResult<TenderlyDecodeInputResult>;

    /// Retrieve error interface based on 4-byte error selector.
    async fn tenderly_error_signatures(
        &self,
        selector: Bytes,
    ) -> TransportResult<Vec<TenderlyFunctionSignature>>;

    /// Retrieve event interface based on 32-byte event signature.
    async fn tenderly_event_signature(
        &self,
        signature: B256,
    ) -> TransportResult<TenderlyFunctionSignature>;

    /// Returns an array of transactions between specified addresses within a given block range.
    async fn tenderly_get_transactions_range(
        &self,
        params: TenderlyTransactionRangeParams,
    ) -> TransportResult<Vec<N::TransactionResponse>>;

    /// Returns the ABI for a given contract address.
    ///
    /// The ABI describes the contract's interface including function definitions, event
    /// definitions, constructor arguments, and state variable definitions.
    async fn tenderly_get_contract_abi(
        &self,
        address: Address,
    ) -> TransportResult<Vec<serde_json::Value>>;

    /// Returns an array of storage changes for a given contract address starting from the specified
    /// offset.
    ///
    /// This method returns storage slot changes, block numbers where changes occurred,
    /// transaction hashes that caused the changes, and previous and new values for each change.
    /// The changes are returned in chronological order, with newer changes appearing first.
    async fn tenderly_get_storage_changes(
        &self,
        params: TenderlyStorageQueryParams,
    ) -> TransportResult<Vec<TenderlyStorageChange>>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> TenderlyApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    async fn tenderly_simulate_transaction(
        &self,
        tx: N::TransactionRequest,
        block: BlockNumberOrTag,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<BlockOverrides>,
    ) -> TransportResult<TenderlySimulationResult> {
        self.client()
            .request("tenderly_simulateTransaction", (tx, block, state_overrides, block_overrides))
            .await
    }

    async fn tenderly_simulate_bundle(
        &self,
        txs: &[N::TransactionRequest],
        block: BlockNumberOrTag,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<BlockOverrides>,
    ) -> TransportResult<Vec<TenderlySimulationResult>> {
        self.client()
            .request("tenderly_simulateBundle", (txs, block, state_overrides, block_overrides))
            .await
    }

    async fn tenderly_trace_transaction(
        &self,
        txs: &[TxHash],
    ) -> TransportResult<TenderlySimulationResult> {
        self.client().request("tenderly_traceTransaction", txs).await
    }

    async fn tenderly_estimate_gas(
        &self,
        tx: N::TransactionRequest,
        block: BlockNumberOrTag,
    ) -> TransportResult<TenderlyEstimateGasResult> {
        self.client().request("tenderly_estimateGas", (tx, block)).await
    }

    async fn tenderly_gas_price(&self) -> TransportResult<TenderlyGasPriceResult> {
        self.client().request_noparams("tenderly_gasPrice").await
    }

    async fn tenderly_suggest_gas_fee(&self) -> TransportResult<TenderlyGasPriceResult> {
        self.client().request_noparams("tenderly_suggestGasFee").await
    }

    async fn tenderly_estimate_gas_bundle(
        &self,
        txs: &[N::TransactionRequest],
        block: BlockNumberOrTag,
    ) -> TransportResult<Vec<TenderlyEstimateGasResult>> {
        self.client().request("tenderly_estimateGasBundle", (txs, block)).await
    }

    async fn tenderly_decode_input(
        &self,
        call_data: Bytes,
    ) -> TransportResult<TenderlyDecodeInputResult> {
        self.client().request("tenderly_decodeInput", (call_data,)).await
    }

    async fn tenderly_decode_error(
        &self,
        error_data: Bytes,
    ) -> TransportResult<TenderlyDecodeInputResult> {
        self.client().request("tenderly_decodeError", (error_data,)).await
    }

    async fn tenderly_function_signatures(
        &self,
        selector: Bytes,
    ) -> TransportResult<Vec<TenderlyFunctionSignature>> {
        self.client().request("tenderly_functionSignatures", (selector,)).await
    }

    async fn tenderly_decode_event(
        &self,
        topics: Vec<B256>,
        data: Bytes,
    ) -> TransportResult<TenderlyDecodeInputResult> {
        self.client().request("tenderly_decodeEvent", (topics, data)).await
    }

    async fn tenderly_error_signatures(
        &self,
        selector: Bytes,
    ) -> TransportResult<Vec<TenderlyFunctionSignature>> {
        self.client().request("tenderly_errorSignatures", (selector,)).await
    }

    async fn tenderly_event_signature(
        &self,
        signature: B256,
    ) -> TransportResult<TenderlyFunctionSignature> {
        self.client().request("tenderly_eventSignature", (signature,)).await
    }

    async fn tenderly_get_transactions_range(
        &self,
        params: TenderlyTransactionRangeParams,
    ) -> TransportResult<Vec<N::TransactionResponse>> {
        self.client().request("tenderly_getTransactionsRange", (params,)).await
    }

    async fn tenderly_get_contract_abi(
        &self,
        address: Address,
    ) -> TransportResult<Vec<serde_json::Value>> {
        self.client().request("tenderly_getContractAbi", (address,)).await
    }

    async fn tenderly_get_storage_changes(
        &self,
        params: TenderlyStorageQueryParams,
    ) -> TransportResult<Vec<TenderlyStorageChange>> {
        self.client().request("tenderly_getStorageChanges", (params,)).await
    }
}

#[cfg(test)]
mod test {
    use std::{env, str::FromStr};

    use alloy_primitives::{address, bytes, utils::parse_ether, Address, U256};
    use alloy_rpc_types_eth::{
        state::{AccountOverride, StateOverridesBuilder},
        TransactionRequest,
    };

    use crate::ProviderBuilder;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_simulate_transaction_erc20() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let gas_price = provider.get_gas_price().await.unwrap();
        let block = BlockNumberOrTag::Latest;
        let value = parse_ether("1").unwrap();

        // send to WETH9 to cause an erc20 transfer
        let tx = TransactionRequest::default()
            .from(Address::ZERO)
            .to(address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"))
            .value(value)
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);

        let account_override = AccountOverride::default().with_balance(U256::MAX);
        let state_override =
            StateOverridesBuilder::default().append(Address::ZERO, account_override).build();

        let _res = provider
            .tenderly_simulate_transaction(tx, block, Some(state_override), None)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_simulate_transaction_native() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let gas_price = provider.get_gas_price().await.unwrap();
        let block = BlockNumberOrTag::Latest;
        let value = parse_ether("1").unwrap();

        let tx = TransactionRequest::default()
            .from(Address::ZERO)
            .to(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
            .value(value)
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);

        let account_override = AccountOverride::default().with_balance(U256::MAX);
        let state_override =
            StateOverridesBuilder::default().append(Address::ZERO, account_override).build();

        let _res = provider
            .tenderly_simulate_transaction(tx, block, Some(state_override), None)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_simulate_batch() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let gas_price = provider.get_gas_price().await.unwrap();
        let block = BlockNumberOrTag::Latest;
        let value = parse_ether("1").unwrap();

        let tx = TransactionRequest::default()
            .from(Address::ZERO)
            .to(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
            .value(value)
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);

        let account_override = AccountOverride::default().with_balance(U256::MAX);
        let state_override =
            StateOverridesBuilder::default().append(Address::ZERO, account_override).build();

        let _res = provider
            .tenderly_simulate_bundle(&[tx.clone(), tx], block, Some(state_override), None)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_trace_transaction() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let hash =
            TxHash::from_str("0x6b2264fa8e28a641d834482d250080b39cbbf39251344573c7504d6137c4b793")
                .unwrap();

        let _res = provider.tenderly_trace_transaction(&[hash]).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_estimate_gas() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let tx = TransactionRequest::default()
            .from(address!("8516feaea147ea0db64d1c5b97bb651ca5435155"))
            .to(address!("6b175474e89094c44da98b954eedeac495271d0f"))
            .input(bytes!("a9059cbb0000000000000000000000003fc3c4c84bdd2db5ab2cc62f93b2a9a347de25fb00000000000000000000000000000000000000000000001869c36187f3430000").into());

        let block = BlockNumberOrTag::Number(21285787);

        let res = provider.tenderly_estimate_gas(tx, block).await.unwrap();

        assert!(res.gas > 0);
        assert!(res.gas_used > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_gas_price() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let res = provider.tenderly_gas_price().await.unwrap();

        assert!(res.current_block_number > 0);
        assert!(res.base_fee_per_gas > 0);
        assert!(res.price.low.max_priority_fee_per_gas > 0);
        assert!(res.price.low.max_fee_per_gas > 0);
        assert!(res.price.medium.max_priority_fee_per_gas > 0);
        assert!(res.price.medium.max_fee_per_gas > 0);
        assert!(res.price.high.max_priority_fee_per_gas > 0);
        assert!(res.price.high.max_fee_per_gas > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_suggest_gas_fee() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let res = provider.tenderly_suggest_gas_fee().await.unwrap();

        assert!(res.current_block_number > 0);
        assert!(res.base_fee_per_gas > 0);
        assert!(res.price.low.max_priority_fee_per_gas > 0);
        assert!(res.price.low.max_fee_per_gas > 0);
        assert!(res.price.medium.max_priority_fee_per_gas > 0);
        assert!(res.price.medium.max_fee_per_gas > 0);
        assert!(res.price.high.max_priority_fee_per_gas > 0);
        assert!(res.price.high.max_fee_per_gas > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_estimate_gas_bundle() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let tx1 = TransactionRequest::default()
            .from(address!("7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0"))
            .to(address!("ae7ab96520DE3A18E5e111B5EaAb095312D7fE84"))
            .input(bytes!("095ea7b30000000000000000000000009008d19f58aabd9ed0d60971565aa8510560ab410000000000000000000000000000000000000000000000000c1291a92f17a100").into());

        let tx2 = TransactionRequest::default()
            .from(address!("9008D19f58AAbD9eD0D60971565AA8510560ab41"))
            .to(address!("ae7ab96520DE3A18E5e111B5EaAb095312D7fE84"))
            .input(bytes!("23b872dd0000000000000000000000007f39c581f595b53c5cb19bd0b3f8da6c935e2ca00000000000000000000000009008d19f58aabd9ed0d60971565aa8510560ab410000000000000000000000000000000000000000000000000c1291a92f17a100").into());

        let block = BlockNumberOrTag::Latest;

        let res = provider.tenderly_estimate_gas_bundle(&[tx1, tx2], block).await.unwrap();

        assert!(!res.is_empty());
        assert_eq!(res.len(), 2);
        assert!(res[0].gas > 0);
        assert!(res[0].gas_used > 0);
        assert!(res[1].gas > 0);
        assert!(res[1].gas_used > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_decode_input() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let call_data = bytes!("a9059cbb00000000000000000000000011223344551122334455112233445511223344550000000000000000000000000000000000000000000000000000000000000999");

        let res = provider.tenderly_decode_input(call_data).await.unwrap();

        assert!(!res.name.is_empty());
        assert!(!res.decoded_arguments.is_empty());
    }
}
