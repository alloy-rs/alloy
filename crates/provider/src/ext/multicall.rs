use crate::{MulticallBuilder, Provider};
use alloy_network::{Ethereum, Network};
use alloy_primitives::Address;
use alloy_sol_types::SolCall;

/// Multicall Ext Trait
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait MulticallApi<N: Network = Ethereum>: Provider<N> + Send + Sync + Sized {
    /// Initalizes a new [`MulticallBuilder`] with the given call and target.
    fn multicall<C: SolCall + 'static>(
        &self,
        call: C,
        target: Address,
    ) -> MulticallBuilder<(C,), &Self, N>;
}

impl<N, P> MulticallApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    /// Execute a multicall by leveraging the [`MulticallBuilder`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// use alloy_primitives::address;
    /// use alloy_provider::ProviderBuilder;
    /// use alloy_sol_types::sol;
    ///
    /// sol! {
    ///    #[derive(Debug, PartialEq)]
    ///   interface ERC20 {
    ///      function totalSupply() external view returns (uint256 totalSupply);
    ///     function balanceOf(address owner) external view returns (uint256 balance);
    ///   }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     const FORK_URL: &str = "https://eth.merkle.io/";
    ///     let provider = ProviderBuilder::new().on_anvil_with_config(|a| a.fork(FORK_URL));
    ///
    ///     let ts_call = ERC20::totalSupplyCall {};
    ///     let balance_call =
    ///         ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };
    ///
    ///     let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    ///
    ///     let call = provider.multicall(ts_call, weth).add(balance_call, weth);
    ///
    ///     let (block_num, (total_supply, balance)) = call.aggregate().await.unwrap();
    ///
    ///     println!("Current Block Number: {}", block_num);
    ///     println!("Total Supply: {:?}", total_supply);
    ///     println!("Balance: {:?}", balance);
    /// }
    /// ```
    fn multicall<C: SolCall + 'static>(
        &self,
        call: C,
        target: Address,
    ) -> MulticallBuilder<(C,), &P, N> {
        MulticallBuilder::new(self).add(call, target)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_primitives::address;
    use alloy_sol_types::sol;

    sol! {
        #[derive(Debug, PartialEq)]
        interface ERC20 {
            function totalSupply() external view returns (uint256 totalSupply);
            function balanceOf(address owner) external view returns (uint256 balance);
            function transfer(address to, uint256 value) external returns (bool);
        }
    }

    #[tokio::test]
    async fn multicall() {
        const FORK_URL: &str =
            "https://eth-mainnet.alchemyapi.io/v2/jGiK5vwDfC3F4r0bqukm-W2GqgdrxdSr";
        let provider = ProviderBuilder::new().on_anvil_with_config(|a| a.fork(FORK_URL));

        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let call = provider.multicall(ts_call, weth).add(balance_call, weth);

        let (_block_num, (_total_supply, _balance)) = call.aggregate().await.unwrap();
    }
}
