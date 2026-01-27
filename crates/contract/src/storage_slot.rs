use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::state::{AccountOverride, StateOverridesBuilder};
use alloy_sol_types::{sol, SolCall, SolValue};
use alloy_transport::TransportError;

/// A utility for finding storage slots in smart contracts, particularly useful for ERC20 tokens.
///
/// This struct helps identify which storage slot contains a specific value by:
/// 1. Creating an access list to find all storage slots accessed by a function call
/// 2. Systematically overriding each slot with an expected value
/// 3. Checking if the function returns the expected value to identify the correct slot
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use alloy_contract::StorageSlotFinder;
/// use alloy_primitives::{address, U256};
/// use alloy_provider::ProviderBuilder;
///
/// let provider = ProviderBuilder::new().connect_anvil();
/// let token = address!("0x6B175474E89094C44Da98b954EedeAC495271d0F");
/// let user = address!("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
///
/// // Find the storage slot for a user's balance
/// let finder =
///     StorageSlotFinder::balance_of(provider, token, user).with_expected_value(U256::from(1000));
///
/// if let Some(slot) = finder.find_slot().await? {
///     println!("Balance stored at slot: {:?}", slot);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct StorageSlotFinder<P, N>
where
    N: Network,
{
    provider: P,
    contract: Address,
    calldata: Bytes,
    expected_value: U256,
    base_request: N::TransactionRequest,
}

impl<P, N> StorageSlotFinder<P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Creates a new storage slot finder for a generic function call.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to use for making calls
    /// * `contract` - The address of the contract to analyze
    /// * `calldata` - The encoded function call to execute
    /// * `expected_value` - The value we expect the function to return
    ///
    /// For common ERC20 use cases, consider using [`Self::balance_of`] instead.
    pub fn new(provider: P, contract: Address, calldata: Bytes, expected_value: U256) -> Self {
        Self {
            provider,
            contract,
            calldata,
            expected_value,
            base_request: N::TransactionRequest::default(),
        }
    }

    /// Convenience constructor for finding the storage slot of an ERC20 `balanceOf(address)`
    /// mapping.
    ///
    /// Uses a default expected value of 1337. Call [`Self::with_expected_value`] to set a different
    /// value.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to use for making calls
    /// * `token_address` - The address of the ERC20 token contract
    /// * `user` - The address of the user whose balance slot we're finding
    pub fn balance_of(provider: P, token_address: Address, user: Address) -> Self {
        sol! {
            contract IERC20 {
                function balanceOf(address target) external view returns (uint256);
            }
        }
        let calldata = IERC20::balanceOfCall { target: user }.abi_encode().into();
        Self::new(provider, token_address, calldata, U256::from(1337))
    }

    /// Configures a specific value that should be used in the state override to identify the slot.
    pub const fn with_expected_value(mut self, value: U256) -> Self {
        self.expected_value = value;
        self
    }

    /// Overrides the base request object that will be used for slot detection.
    ///
    /// For slot detection the target address of that request is set to the configured contract and
    /// the input to the configured input.
    pub fn with_request(mut self, base_request: N::TransactionRequest) -> Self {
        self.base_request = base_request;
        self
    }

    /// Finds the storage slot containing the expected value.
    ///
    /// This method:
    /// 1. Creates an access list for the function call to identify all storage slots accessed
    /// 2. Iterates through each accessed slot on the target contract
    /// 3. Overrides each slot with the expected value using state overrides
    /// 4. Checks if the function returns the expected value when that slot is overridden
    /// 5. Returns the first slot that causes the function to return the expected value
    ///
    /// # Returns
    ///
    /// * `Ok(Some(slot))` - The storage slot that contains the value
    /// * `Ok(None)` - No storage slot was found containing the value
    /// * `Err(TransportError)` - An error occurred during RPC calls
    ///
    /// # Note
    ///
    /// This method assumes that the value is stored directly in a storage slot without
    /// any encoding or hashing. For mappings, the actual storage location might be
    /// computed using keccak256 hashing.
    pub async fn find_slot(self) -> Result<Option<B256>, TransportError> {
        let Self { provider, contract, calldata, expected_value, base_request } = self;

        let tx = base_request.with_to(contract).with_input(calldata);

        // first collect all the slots that are used by the function call
        let access_list_result = provider.create_access_list(&tx).await?;
        let access_list = access_list_result.access_list;

        // Track whether any call succeeded and capture the first error for diagnostics.
        // If all overridden calls fail, we propagate the first error instead of returning Ok(None).
        let mut any_call_succeeded = false;
        let mut first_call_err: Option<TransportError> = None;

        // iterate over all the accessed slots and try to find the one that contains the
        // target value by overriding the slot and checking the function call result
        for item in access_list.0 {
            if item.address != contract {
                continue;
            };
            for slot in &item.storage_keys {
                let account_override = AccountOverride::default().with_state_diff(std::iter::once(
                    (*slot, B256::from(expected_value.to_be_bytes())),
                ));

                let state_override =
                    StateOverridesBuilder::default().append(contract, account_override).build();

                let result = match provider.call(tx.clone()).overrides(state_override).await {
                    Ok(res) => {
                        any_call_succeeded = true;
                        res
                    }
                    Err(err) => {
                        first_call_err.get_or_insert(err);
                        continue;
                    }
                };

                let Ok(result_value) = U256::abi_decode(&result) else {
                    // response returned something other than a U256
                    continue;
                };

                if result_value == expected_value {
                    return Ok(Some(*slot));
                }
            }
        }

        // If no call succeeded and we have an error, propagate it rather than silently returning
        // None
        if !any_call_succeeded {
            if let Some(err) = first_call_err {
                return Err(err);
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::StorageSlotFinder;
    use alloy_network::TransactionBuilder;
    use alloy_primitives::{address, ruint::uint, Address, B256, U256};
    use alloy_provider::{ext::AnvilApi, Provider, ProviderBuilder};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_sol_types::sol;
    const FORK_URL: &str = "https://reth-ethereum.ithaca.xyz/rpc";
    use alloy_sol_types::SolCall;

    async fn test_erc20_token_set_balance(token: Address) {
        let provider = ProviderBuilder::new().connect_anvil_with_config(|a| a.fork(FORK_URL));
        let user = address!("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let amount = U256::from(500u64);
        let finder = StorageSlotFinder::balance_of(provider.clone(), token, user);
        let storage_slot = U256::from_be_bytes(finder.find_slot().await.unwrap().unwrap().0);

        provider
            .anvil_set_storage_at(token, storage_slot, B256::from(amount.to_be_bytes()))
            .await
            .unwrap();

        sol! {
            function balanceOf(address owner) view returns (uint256);
        }

        let balance_of_call = balanceOfCall::new((user,));
        let input = balanceOfCall::abi_encode(&balance_of_call);

        let result = provider
            .call(TransactionRequest::default().with_to(token).with_input(input))
            .await
            .unwrap();
        let balance = balanceOfCall::abi_decode_returns(&result).unwrap();

        assert_eq!(balance, amount);
    }

    #[tokio::test]
    async fn test_erc20_dai_set_balance() {
        let dai = address!("0x6B175474E89094C44Da98b954EedeAC495271d0F");
        test_erc20_token_set_balance(dai).await
    }

    #[tokio::test]
    async fn test_erc20_usdc_set_balance() {
        let usdc = address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        test_erc20_token_set_balance(usdc).await
    }

    #[tokio::test]
    async fn test_erc20_tether_set_balance() {
        let tether = address!("0xdAC17F958D2ee523a2206206994597C13D831ec7");
        test_erc20_token_set_balance(tether).await
    }
    #[tokio::test]
    async fn test_erc20_token_polygon() {
        let provider =
            ProviderBuilder::new().connect_http("https://polygon-rpc.com".parse().unwrap());
        let usdt = address!("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"); // https://polygonscan.com/address/0xc2132D05D31c914a87C6611C10748AEb04B58e8F
        let user = address!("0x0aD71c9106455801eAe0e11D5A1Dd5232537E662");
        let finder = StorageSlotFinder::balance_of(provider.clone(), usdt, user)
            .with_request(TransactionRequest::default().gas_limit(100000));
        let storage_slot = U256::from_be_bytes(finder.find_slot().await.unwrap().unwrap().0);
        assert_eq!(
            storage_slot,
            uint!(
                38414845661641411266428303013962925072609060211040678298987263275302781786590_U256
            )
        );
    }
}
