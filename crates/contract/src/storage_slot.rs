use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::{
    state::{AccountOverride, EvmOverrides, StateOverridesBuilder},
    TransactionRequest,
};
use alloy_sol_types::{sol, SolValue};

/// A future type for finding erc20 storage slot.
#[derive(Debug, Clone)]
pub struct StorageSlotFinder<P> {
    provider: P,
    token_address: Address,
    calldata: Bytes,
    expected_value: U256,
}

impl<P> StorageSlotFinder<P>
where
    P: Provider,
{
    /// Creates a new finder.
    pub fn new(provider: P, token_address: Address, calldata: Bytes, expected_value: U256) -> Self {
        Self { provider, token_address, calldata, expected_value }
    }

    /// Convenience constructor for `balanceOf(address)` case.
    pub fn for_balance_of(
        provider: P,
        token_address: Address,
        user: Address,
        expected_balance: U256,
    ) -> Self {
        sol! {
            contract IERC20 {
                function balanceOf(address target) external view returns (uint256);
            }
        }

        let calldata = IERC20::balanceOfCall { target: user }.target.abi_encode().into();
        Self { provider, token_address, calldata, expected_value: expected_balance }
    }

    /// Finds the storage slot.
    pub async fn find(self) -> Result<B256, crate::Error> {
        let tx = TransactionRequest::default()
            .with_to(self.token_address)
            .with_input(self.calldata.clone());

        // first collect all the slots that are used by the function call
        let access_list_result = self.provider.create_access_list(&tx.clone()).await;
        let access_list = access_list_result.unwrap().access_list;
        // iterate over all the accessed slots and try to find the one that contains the
        // target value by overriding the slot and checking the function call result
        for item in access_list.0 {
            if item.address != self.token_address {
                continue;
            };
            for slot in &item.storage_keys {
                let account_override = AccountOverride::default().with_state_diff(std::iter::once(
                    (*slot, B256::from(self.expected_value.to_be_bytes())),
                ));

                let state_override = StateOverridesBuilder::default()
                    .append(self.token_address, account_override)
                    .build();

                let Ok(result) = self.provider.call(tx.clone()).overrides(state_override).await
                else {
                    // overriding this slot failed
                    continue;
                };

                let Ok(result_value) = U256::abi_decode(&result) else {
                    // response returned something other than a U256
                    continue;
                };

                if result_value == self.expected_value {
                    return Ok(*slot);
                }
            }
        }
        Err(crate::Error::StorageSlotNotFound(self.token_address.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::StorageSlotFinder;
    use alloy_network::TransactionBuilder;
    use alloy_primitives::{address, B256, U256};
    use alloy_provider::{ext::AnvilApi, Provider, ProviderBuilder};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_sol_types::sol;
    const FORK_URL: &str = "https://reth-ethereum.ithaca.xyz/rpc";
    use alloy_sol_types::SolCall;

    #[tokio::test]
    async fn test_erc20_set_balance() {
        let provider = ProviderBuilder::new().connect_anvil_with_config(|a| a.fork(FORK_URL));
        let dai = address!("0x6B175474E89094C44Da98b954EedeAC495271d0F");
        let user = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let amount = U256::from(1e18 as u64);
        alloy_sol_types::sol! {
           contract ERC20 {
                function balanceOf(address owner) public view returns (uint256);
           }
        }
        let finder = StorageSlotFinder::for_balance_of(provider.clone(), dai, user, amount);
        let storage_slot = U256::from_be_bytes(finder.find().await.unwrap().0);

        provider
            .anvil_set_storage_at(dai, storage_slot, B256::from(amount.to_be_bytes()))
            .await
            .unwrap();

        sol! {
            function balanceOf(address owner) view returns (uint256);
        }

        let balance_of_call = balanceOfCall::new((user,));
        let input = balanceOfCall::abi_encode(&balance_of_call);

        let result = provider
            .call(TransactionRequest::default().with_to(dai).with_input(input))
            .await
            .unwrap();
        let balance = balanceOfCall::abi_decode_returns(&result).unwrap();

        assert_eq!(balance, amount);
    }
}
