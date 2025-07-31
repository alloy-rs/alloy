//! This module extends the Ethereum JSON-RPC provider with the Tenderly namespace's RPC methods.
use crate::Provider;
use alloy_eips::BlockNumberOrTag;
use alloy_network::Network;
use alloy_primitives::TxHash;
use alloy_rpc_types_eth::{state::StateOverride, BlockOverrides};
use alloy_rpc_types_tenderly::TenderlySimulationResult;
use alloy_transport::TransportResult;

/// Tenderly namespace rpc interface that gives access to several non-standard RPC methods.
///
/// Some methods are currently not implemented:
/// - tenderly_estimateGas
/// - tenderly_gasPrice
/// - tenderly_suggestGasFee
/// - tenderly_estimateGasBundle
/// - tenderly_decodeInput
/// - tenderly_decodeError
/// - tenderly_decodeEvent
/// - tenderly_functionSignatures
/// - tenderly_errorSignatures
/// - tenderly_eventSignatures
/// - tenderly_getTransactionRange
/// - tenderly_getContractAbi
/// - tenderly_getStorageChanges
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
}

#[cfg(test)]
mod test {
    use std::{env, str::FromStr};

    use alloy_primitives::{address, utils::parse_ether, Address, U256};
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
            .tenderly_simulate_bundle(&vec![tx.clone(), tx], block, Some(state_override), None)
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
}
