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
        self.client().request("tenderly_setNextBlockTimestamp", (timestamp,)).await
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

#[cfg(test)]
mod params_tests {
    use super::*;
    use crate::RootProvider;
    use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
    use alloy_network::Ethereum;
    use alloy_rpc_client::RpcClient;
    use alloy_transport::TransportFut;
    use serde_json::value::RawValue;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn set_next_block_timestamp_sends_params_array() {
        let params = Arc::new(Mutex::new(None));
        let service = {
            let params = params.clone();
            tower::service_fn(move |request: RequestPacket| {
                let params = params.clone();
                Box::pin(async move {
                    let RequestPacket::Single(request) = request else {
                        panic!("expected a single request");
                    };
                    *params.lock().unwrap() = request.params().map(|p| p.get().to_string());
                    Ok(ResponsePacket::Single(Response {
                        id: request.id().clone(),
                        payload: ResponsePayload::Success(
                            RawValue::from_string("1700000000".into()).unwrap(),
                        ),
                    }))
                }) as TransportFut<'static>
            })
        };
        let provider = RootProvider::<Ethereum>::new(RpcClient::new(service, true));

        let timestamp = provider.tenderly_set_next_block_timestamp(1_700_000_000).await.unwrap();
        assert_eq!(timestamp, 1_700_000_000);
        assert_eq!(params.lock().unwrap().as_deref(), Some("[1700000000]"));
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use alloy_network::TransactionBuilder;
    use alloy_primitives::{address, bytes, Address, U256};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_sol_types::{sol, SolCall};

    use crate::ProviderBuilder;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_set_balance() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        provider.tenderly_set_balance(alice, U256::ONE).await.unwrap();

        let balance = provider.get_balance(alice).await.unwrap();
        assert_eq!(balance, U256::ONE);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_set_balance_batch() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        let bob = Address::random();
        let wallets = vec![alice, bob];

        provider.tenderly_set_balance_batch(&wallets, U256::ONE).await.unwrap();

        let balance = provider.get_balance(alice).await.unwrap();
        assert_eq!(balance, U256::ONE);

        let balance = provider.get_balance(bob).await.unwrap();
        assert_eq!(balance, U256::ONE);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_add_balance() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        provider.tenderly_add_balance(alice, U256::ONE).await.unwrap();
        provider.tenderly_add_balance(alice, U256::ONE).await.unwrap();

        let balance = provider.get_balance(alice).await.unwrap();
        assert_eq!(balance, U256::from(2));
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_add_balance_batch() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        let bob = Address::random();
        let wallets = vec![alice, bob];

        provider.tenderly_add_balance_batch(&wallets, U256::ONE).await.unwrap();
        provider.tenderly_add_balance_batch(&wallets, U256::ONE).await.unwrap();

        let balance = provider.get_balance(alice).await.unwrap();
        assert_eq!(balance, U256::from(2));

        let balance = provider.get_balance(bob).await.unwrap();
        assert_eq!(balance, U256::from(2));
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_set_erc20_balance() {
        sol! {
            contract IERC20 {
                function balanceOf(address target) external view returns (uint256);
            }
        }

        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        let dai = address!("0x6B175474E89094C44Da98b954EedeAC495271d0F");

        let input = IERC20::balanceOfCall::new((alice,)).abi_encode();
        let balance_of_tx = TransactionRequest::default().with_to(dai).with_input(input);

        provider.tenderly_set_erc20_balance(dai, alice, U256::ONE).await.unwrap();
        let balance = provider.call(balance_of_tx).await.unwrap();
        assert_eq!(IERC20::balanceOfCall::abi_decode_returns(&balance).unwrap(), U256::ONE);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_set_storage() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        let key = U256::from(42);

        let before = provider.get_storage_at(alice, key).await.unwrap();
        assert_eq!(before, U256::ZERO);

        provider.tenderly_set_storage_at(alice, key, U256::from(7).into()).await.unwrap();

        let after = provider.get_storage_at(alice, key).await.unwrap();
        assert_eq!(after, U256::from(7));
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenderly_set_code() {
        let url = env::var("TENDERLY_URL").unwrap().parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(url);

        let alice = Address::random();
        let code = bytes!("0xdeadbeef");

        provider.tenderly_set_code(alice, code.clone()).await.unwrap();

        let after = provider.get_code_at(alice).await.unwrap();
        assert_eq!(after, code);
    }
}
