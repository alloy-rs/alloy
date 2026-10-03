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
