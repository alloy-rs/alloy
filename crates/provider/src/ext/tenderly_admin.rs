use alloy_network::Network;
use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};
use alloy_transport::TransportResult;

use crate::Provider;

/// Tenderly namespace rpc interface that gives access to several admin
/// RPC methods on tenderly virtual testnets.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait TenderlyAdminApi<N: Network>: Send + Sync {
    /// Offsets current time to given timestamp without creating an empty block.
    /// Different to `evm_setNextBlockTimestamp` which mines a block.
    async fn tenderly_set_next_block_timestamp(&self, timestamp: u64) -> TransportResult<u64>;

    /// Set the balance of an address by sending an overwrite transaction.
    /// Returns the transaction hash.
    async fn tenderly_set_balance(
        &self,
        wallet: Address,
        balance: U256,
    ) -> TransportResult<FixedBytes<32>>;

    /// Set the balance of multiple addresses.
    /// Returns the transaction hash (storage override is committed via transaction).
    async fn tenderly_set_balance_batch(
        &self,
        wallets: &[Address],
        balance: U256,
    ) -> TransportResult<FixedBytes<32>>;

    /// Adds to the balance of an address
    /// Returns the transaction hash (storage override is committed via transaction).
    async fn tenderly_add_balance(
        &self,
        wallet: Address,
        amount: U256,
    ) -> TransportResult<FixedBytes<32>>;

    /// Adds to the balance of multiple addresses
    /// Returns the transaction hash (storage override is committed via transaction).
    async fn tenderly_add_balance_batch(
        &self,
        wallets: &[Address],
        amount: U256,
    ) -> TransportResult<FixedBytes<32>>;

    /// Sets the ERC20 balance of a wallet.
    /// Returns the transaction hash (storage override is committed via transaction).
    async fn tenderly_set_erc20_balance(
        &self,
        token: Address,
        wallet: Address,
        balance: U256,
    ) -> TransportResult<FixedBytes<32>>;

    /// Sets a storage slot of an address.
    /// Returns the transaction hash (storage override is committed via transaction).
    async fn tenderly_set_storage_at(
        &self,
        address: Address,
        slot: U256,
        value: B256,
    ) -> TransportResult<FixedBytes<32>>;

    /// Sets the code of an address.
    /// Returns the transaction hash (storage override is committed via transaction).
    async fn tenderly_set_code(
        &self,
        address: Address,
        code: Bytes,
    ) -> TransportResult<FixedBytes<32>>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> TenderlyAdminApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    async fn tenderly_set_next_block_timestamp(&self, timestamp: u64) -> TransportResult<u64> {
        self.client().request("tenderly_setNextBlockTimestamp", timestamp).await
    }

    async fn tenderly_set_balance(
        &self,
        wallet: Address,
        balance: U256,
    ) -> TransportResult<FixedBytes<32>> {
        self.client().request("tenderly_setBalance", (wallet, balance)).await
    }

    async fn tenderly_set_balance_batch(
        &self,
        wallets: &[Address],
        balance: U256,
    ) -> TransportResult<FixedBytes<32>> {
        self.client().request("tenderly_setBalance", (wallets, balance)).await
    }

    async fn tenderly_add_balance(
        &self,
        wallet: Address,
        balance: U256,
    ) -> TransportResult<FixedBytes<32>> {
        self.client().request("tenderly_addBalance", (wallet, balance)).await
    }

    async fn tenderly_add_balance_batch(
        &self,
        wallets: &[Address],
        balance: U256,
    ) -> TransportResult<FixedBytes<32>> {
        self.client().request("tenderly_addBalance", (wallets, balance)).await
    }

    async fn tenderly_set_erc20_balance(
        &self,
        token: Address,
        wallet: Address,
        balance: U256,
    ) -> TransportResult<FixedBytes<32>> {
        self.client().request("tenderly_setErc20Balance", (token, wallet, balance)).await
    }

    async fn tenderly_set_storage_at(
        &self,
        address: Address,
        slot: U256,
        value: B256,
    ) -> TransportResult<FixedBytes<32>> {
        self.client()
            .request("tenderly_setStorageAt", (address, FixedBytes::from(slot), value))
            .await
    }

    async fn tenderly_set_code(
        &self,
        address: Address,
        code: Bytes,
    ) -> TransportResult<FixedBytes<32>> {
        self.client().request("tenderly_setCode", (address, code)).await
    }
}
