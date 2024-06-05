//! This module extends the Ethereum JSON-RPC provider with the Anvil namespace's RPC methods.
use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, TxHash, B256, U256};
use alloy_rpc_types::{Block, TransactionRequest, WithOtherFields};
use alloy_transport::{Transport, TransportResult};

/// Anvil namespace rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait AnvilApi<N, T>: Send + Sync {
    /// Send transactions impersonating specific account and contract addresses.
    async fn anvil_impersonate_account(&self, address: Address) -> TransportResult<()>;

    /// Stops impersonating an account if previously set with `anvil_impersonateAccount`.
    async fn anvil_stop_impersonating_account(&self, address: Address) -> TransportResult<()>;

    /// If set to true will make every account impersonated.
    async fn anvil_auto_impersonate_account(&self, enabled: bool) -> TransportResult<()>;

    /// Returns true if auto mining is enabled, and false.
    async fn anvil_get_auto_mine(&self) -> TransportResult<bool>;

    /// Enables or disables, based on the single boolean argument, the automatic mining of new
    /// blocks with each new transaction submitted to the network.
    async fn anvil_set_auto_mine(&self, enable_automine: bool) -> TransportResult<()>;

    /// Mines a series of blocks.
    async fn anvil_mine(
        &self,
        num_blocks: Option<U256>,
        interval: Option<U256>,
    ) -> TransportResult<()>;

    /// Sets the mining behavior to interval with the given interval (seconds).
    async fn anvil_set_interval_mining(&self, secs: u64) -> TransportResult<()>;

    /// Removes transactions from the pool.
    async fn anvil_drop_transaction(&self, tx_hash: B256) -> TransportResult<Option<B256>>;

    /// Removes all transactions from the pool.
    async fn anvil_drop_all_transactions(&self) -> TransportResult<()>;

    /// Reset the fork to a fresh forked state, and optionally update the fork config.
    ///
    /// If `forking` is `None` then this will disable forking entirely.
    async fn anvil_reset(&self, forking: Option<Forking>) -> TransportResult<()>;

    /// Sets the chain ID.
    async fn anvil_set_chain_id(&self, chain_id: u64) -> TransportResult<()>;

    /// Modifies the balance of an account.
    async fn anvil_set_balance(&self, address: Address, balance: U256) -> TransportResult<()>;

    /// Sets the code of a contract.
    async fn anvil_set_code(&self, address: Address, code: Bytes) -> TransportResult<()>;

    /// Sets the nonce of an address.
    async fn anvil_set_nonce(&self, address: Address, nonce: U256) -> TransportResult<()>;

    /// Writes a single slot of the account's storage.
    async fn anvil_set_storage_at(
        &self,
        address: Address,
        slot: U256,
        val: B256,
    ) -> TransportResult<bool>;

    /// Enable or disable logging.
    async fn anvil_set_logging(&self, enable: bool) -> TransportResult<()>;

    /// Set the minimum gas price for the node.
    async fn anvil_set_min_gas_price(&self, gas: U256) -> TransportResult<()>;

    /// Sets the base fee of the next block.
    async fn anvil_set_next_block_base_fee_per_gas(&self, basefee: U256) -> TransportResult<()>;

    /// Sets the coinbase address.
    async fn anvil_set_coinbase(&self, address: Address) -> TransportResult<()>;

    /// Create a buffer that represents all state on the chain, which can be loaded to separate
    /// process by calling `anvil_loadState`
    async fn anvil_dump_state(&self) -> TransportResult<Bytes>;

    /// Append chain state buffer to current chain. Will overwrite any conflicting addresses or
    /// storage.
    async fn anvil_load_state(&self, buf: Bytes) -> TransportResult<bool>;

    /// Retrieves the Anvil node configuration params.
    async fn anvil_node_info(&self) -> TransportResult<NodeInfo>;

    /// Retrieves metadata about the Anvil instance.
    async fn anvil_metadata(&self) -> TransportResult<AnvilMetadata>;

    /// Removes all transactions from the pool for a specific address.
    async fn anvil_remove_pool_transactions(&self, address: Address) -> TransportResult<()>;

    /// Snapshot the state of the blockchain at the current block.
    async fn evm_snapshot(&self) -> TransportResult<U256>;

    /// Revert the state of the blockchain to a previous snapshot.
    /// Takes a single parameter, which is the snapshot id to revert to.
    async fn evm_revert(&self, id: U256) -> TransportResult<bool>;

    /// Jump forward in time by the given amount of time, in seconds.
    async fn evm_increase_time(&self, seconds: U256) -> TransportResult<i64>;

    /// Similar to `evm_increaseTime` but takes the exact timestamp that you want in the next block.
    async fn evm_set_next_block_timestamp(&self, seconds: u64) -> TransportResult<()>;

    /// Sets the specific timestamp and returns the number of seconds between the given timestamp
    /// and the current time.
    async fn evm_set_time(&self, timestamp: u64) -> TransportResult<u64>;

    /// Set the next block gas limit.
    async fn evm_set_block_gas_limit(&self, gas_limit: U256) -> TransportResult<bool>;

    /// Sets an interval for the block timestamp.
    async fn evm_set_block_timestamp_interval(&self, seconds: u64) -> TransportResult<()>;

    /// Unsets the interval for the block timestamp.
    async fn evm_remove_block_timestamp_interval(&self) -> TransportResult<bool>;

    /// Mine blocks, instantly.
    /// This will mine the blocks regardless of the configured mining mode.
    async fn evm_mine(&self, opts: Option<EvmMineOptions>) -> TransportResult<String>;

    /// Mine blocks, instantly and return the mined blocks.
    /// This will mine the blocks regardless of the configured mining mode.
    async fn evm_mine_detailed(&self, opts: Option<EvmMineOptions>) -> TransportResult<Vec<Block>>;

    /// Sets the reported block number.
    async fn anvil_set_block(&self, block_number: U256) -> TransportResult<()>;

    /// Sets the backend rpc url.
    async fn anvil_set_rpc_url(&self, url: String) -> TransportResult<()>;

    /// Turn on call traces for transactions that are returned to the user when they execute a
    /// transaction (instead of just transaction hash / receipt).
    async fn anvil_enable_traces(&self) -> TransportResult<()>;

    /// Execute a transaction regardless of signature status.
    async fn eth_send_unsigned_transaction(
        &self,
        request: WithOtherFields<TransactionRequest>,
    ) -> TransportResult<TxHash>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> AnvilApi<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn anvil_impersonate_account(&self, address: Address) -> TransportResult<()> {
        self.client().request("anvil_impersonateAccount", (address,)).await
    }

    async fn anvil_stop_impersonating_account(&self, address: Address) -> TransportResult<()> {
        self.client().request("anvil_stopImpersonatingAccount", (address,)).await
    }

    async fn anvil_auto_impersonate_account(&self, enabled: bool) -> TransportResult<()> {
        self.client().request("anvil_autoImpersonateAccount", (enabled,)).await
    }

    async fn anvil_get_auto_mine(&self) -> TransportResult<bool> {
        self.client().request("anvil_getAutoMine", ()).await
    }

    async fn anvil_set_auto_mine(&self, enabled: bool) -> TransportResult<()> {
        self.client().request("anvil_setAutoMine", (enabled,)).await
    }

    async fn anvil_mine(
        &self,
        num_blocks: Option<U256>,
        interval: Option<U256>,
    ) -> TransportResult<()> {
        self.client().request("anvil_mine", (num_blocks, interval)).await
    }

    async fn anvil_set_interval_mining(&self, secs: u64) -> TransportResult<()> {
        self.client().request("anvil_setIntervalMining", (secs,)).await
    }

    async fn anvil_drop_transaction(&self, tx_hash: B256) -> TransportResult<Option<B256>> {
        self.client().request("anvil_dropTransaction", (tx_hash,)).await
    }

    async fn anvil_drop_all_transactions(&self) -> TransportResult<()> {
        self.client().request("anvil_dropAllTransactions", ()).await
    }

    async fn anvil_reset(&self, forking: Option<Forking>) -> TransportResult<()> {
        self.client().request("anvil_reset", (forking,)).await
    }

    async fn anvil_set_chain_id(&self, chain_id: u64) -> TransportResult<()> {
        self.client().request("anvil_setChainId", (chain_id,)).await
    }

    async fn anvil_set_balance(&self, address: Address, balance: U256) -> TransportResult<()> {
        self.client().request("anvil_setBalance", (address, balance)).await
    }

    async fn anvil_set_code(&self, address: Address, code: Bytes) -> TransportResult<()> {
        self.client().request("anvil_setCode", (address, code)).await
    }

    async fn anvil_set_nonce(&self, address: Address, nonce: U256) -> TransportResult<()> {
        self.client().request("anvil_setNonce", (address, nonce)).await
    }

    async fn anvil_set_storage_at(
        &self,
        address: Address,
        slot: U256,
        val: B256,
    ) -> TransportResult<bool> {
        self.client().request("anvil_setStorageAt", (address, slot, val)).await
    }

    async fn anvil_set_logging(&self, enable: bool) -> TransportResult<()> {
        self.client().request("anvil_setLogging", (enable,)).await
    }

    async fn anvil_set_min_gas_price(&self, gas: U256) -> TransportResult<()> {
        self.client().request("anvil_setMinGasPrice", (gas,)).await
    }

    async fn anvil_set_next_block_base_fee_per_gas(&self, basefee: U256) -> TransportResult<()> {
        self.client().request("anvil_setNextBlockBaseFeePerGas", (basefee,)).await
    }

    async fn anvil_set_coinbase(&self, address: Address) -> TransportResult<()> {
        self.client().request("anvil_setCoinbase", (address,)).await
    }

    async fn anvil_dump_state(&self) -> TransportResult<Bytes> {
        self.client().request("anvil_dumpState", ()).await
    }

    async fn anvil_load_state(&self, buf: Bytes) -> TransportResult<bool> {
        self.client().request("anvil_loadState", (buf,)).await
    }

    async fn anvil_node_info(&self) -> TransportResult<NodeInfo> {
        self.client().request("anvil_nodeInfo", ()).await
    }

    async fn anvil_metadata(&self) -> TransportResult<AnvilMetadata> {
        self.client().request("anvil_metadata", ()).await
    }

    async fn anvil_remove_pool_transactions(&self, address: Address) -> TransportResult<()> {
        self.client().request("anvil_removePoolTransactions", (address,)).await
    }

    async fn evm_snapshot(&self) -> TransportResult<U256> {
        self.client().request("evm_snapshot", ()).await
    }

    async fn evm_revert(&self, id: U256) -> TransportResult<bool> {
        self.client().request("evm_revert", (id,)).await
    }

    async fn evm_increase_time(&self, seconds: U256) -> TransportResult<i64> {
        self.client().request("evm_increaseTime", (seconds,)).await
    }

    async fn evm_set_next_block_timestamp(&self, seconds: u64) -> TransportResult<()> {
        self.client().request("evm_setNextBlockTimestamp", (seconds,)).await
    }

    async fn evm_set_time(&self, timestamp: u64) -> TransportResult<u64> {
        self.client().request("evm_setTime", (timestamp,)).await
    }

    async fn evm_set_block_gas_limit(&self, gas_limit: U256) -> TransportResult<bool> {
        self.client().request("evm_setBlockGasLimit", (gas_limit,)).await
    }

    async fn evm_set_block_timestamp_interval(&self, seconds: u64) -> TransportResult<()> {
        self.client().request("evm_setBlockTimestampInterval", (seconds,)).await
    }

    async fn evm_remove_block_timestamp_interval(&self) -> TransportResult<bool> {
        self.client().request("evm_removeBlockTimestampInterval", ()).await
    }

    async fn evm_mine(&self, opts: Option<EvmMineOptions>) -> TransportResult<String> {
        self.client().request("evm_mine", (opts,)).await
    }

    async fn evm_mine_detailed(&self, opts: Option<EvmMineOptions>) -> TransportResult<Vec<Block>> {
        self.client().request("evm_mineDetailed", (opts,)).await
    }

    async fn anvil_set_block(&self, block_number: U256) -> TransportResult<()> {
        self.client().request("anvil_setBlock", (block_number,)).await
    }

    async fn anvil_set_rpc_url(&self, url: String) -> TransportResult<()> {
        self.client().request("anvil_setRpcUrl", (url,)).await
    }

    async fn anvil_enable_traces(&self) -> TransportResult<()> {
        self.client().request("anvil_enableTraces", ()).await
    }

    async fn eth_send_unsigned_transaction(
        &self,
        request: WithOtherFields<TransactionRequest>,
    ) -> TransportResult<TxHash> {
        self.client().request("eth_sendUnsignedTransaction", (request,)).await
    }
}

// /* ------------------------------------------ anvil ----------------------------------------- */
// // TODO: feature gate based on `anvil` feature?

// /// Send transactions impersonating specific account and contract addresses.
// async fn anvil_impersonate_account(&self, address: Address) -> TransportResult<()> {
//     self.client().request("anvil_impersonateAccount", (address,)).await
// }

// /// Stops impersonating an account if previously set with `anvil_impersonateAccount`.
// async fn anvil_stop_impersonating_account(&self, address: Address) -> TransportResult<()> {
//     self.client().request("anvil_stopImpersonatingAccount", (address,)).await
// }

// /// If set to true will make every account impersonated.
// async fn anvil_auto_impersonate_account(&self, enabled: bool) -> TransportResult<()> {
//     self.client().request("anvil_autoImpersonateAccount", (enabled,)).await
// }

// /// Returns true if auto mining is enabled, and false.
// async fn anvil_get_auto_mine(&self) -> TransportResult<bool> {
//     self.client().request("anvil_getAutoMine", ()).await
// }

// /// Enables or disables, based on the single boolean argument, the automatic mining of new
// /// blocks with each new transaction submitted to the network.
// async fn anvil_set_auto_mine(&self, enabled: bool) -> TransportResult<()> {
//     self.client().request("anvil_setAutoMine", (enabled,)).await
// }

// /// Mines a series of blocks.
// async fn anvil_mine(
//     &self,
//     num_blocks: Option<U256>,
//     interval: Option<U256>,
// ) -> TransportResult<()> {
//     self.client().request("anvil_mine", (num_blocks, interval)).await
// }

// /// Sets the mining behavior to interval with the given interval (seconds).
// async fn anvil_set_interval_mining(&self, secs: u64) -> TransportResult<()> {
//     self.client().request("evm_setIntervalMining", (secs,)).await
// }

// /// Removes transactions from the pool.
// async fn anvil_drop_transaction(&self, tx_hash: B256) -> TransportResult<Option<B256>> {
//     self.client().request("anvil_dropTransaction", (tx_hash,)).await
// }

// /// Removes all transactions from the pool
// async fn anvil_drop_all_transactions(&self) -> TransportResult<()> {
//     self.client().request("anvil_dropAllTransactions", ()).await
// }

// /// Reset the fork to a fresh forked state, and optionally update the fork config.
// ///
// /// If `forking` is `None` then this will disable forking entirely.
// // async fn anvil_reset(&self, forking: Option<Forking>) -> TransportResult<()> {
// //     self.client().request("anvil_reset", (forking,)).await
// // }

// /// Sets the chain ID.
// async fn anvil_set_chain_id(&self, chain_id: u64) -> TransportResult<()> {
//     self.client().request("anvil_setChainId", (chain_id,)).await
// }

// /// Modifies the balance of an account.
// async fn anvil_set_balance(&self, address: Address, balance: U256) -> TransportResult<()> {
//     self.client().request("anvil_setBalance", (address, balance)).await
// }

// /// Sets the code of a contract.
// async fn anvil_set_code(&self, address: Address, code: Bytes) -> TransportResult<()> {
//     self.client().request("anvil_setCode", (address, code)).await
// }

// /// Writes a single slot of the account's storage.
// async fn anvil_set_storage_at(
//     &self,
//     address: Address,
//     slot: U256,
//     val: B256,
// ) -> TransportResult<()> {
//     self.client().request("anvil_setStorageAt", (address, slot, val)).await
// }

// /// Enable or disable logging.
// async fn anvil_set_logging(&self, enable: bool) -> TransportResult<()> {
//     self.client().request("anvil_setLogging", (enable,)).await
// }

// /// Set the minimum gas price for the node.
// async fn anvil_set_min_gas_price(&self, gas: U256) -> TransportResult<()> {
//     self.client().request("anvil_setMinGasPrice", (gas,)).await
// }

// /// Sets the base fee of the next block.
// async fn anvil_set_next_block_base_fee_per_gas(&self, basefee: U256) -> TransportResult<()> {
//     self.client().request("anvil_setNextBlockBaseFeePerGas", (basefee,)).await
// }

// /// Sets the coinbase address.
// async fn anvil_set_coinbase(&self, address: Address) -> TransportResult<()> {
//     self.client().request("anvil_setCoinbase", (address,)).await
// }

// /// Create a buffer that represents all state on the chain, which can be loaded to separate
// /// process by calling `anvil_loadState`
// async fn anvil_dump_state(&self) -> TransportResult<Bytes> {
//     self.client().request("anvil_dumpState", ()).await
// }

// /// Append chain state buffer to current chain. Will overwrite any conflicting addresses or
// /// storage.
// async fn anvil_load_state(&self, buf: Bytes) -> TransportResult<bool> {
//     self.client().request("anvil_loadState", (buf,)).await
// }

// /// Retrieves the Anvil node configuration params.
// async fn anvil_node_info(&self) -> TransportResult<NodeInfo> {
//     self.client().request("anvil_nodeInfo", ()).await
// }

// /// Retrieves metadata about the Anvil instance.
// async fn anvil_metadata(&self) -> TransportResult<AnvilMetadata> {
//     self.client().request("anvil_metadata", ()).await
// }

// // async fn anvil_remove_pool_transactions(&self, address: Address) -> TransportResult<()> {
// //     self.client().request("anvil_removePoolTransactions", (address,)).await
// // }
