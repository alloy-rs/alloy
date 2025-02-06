//! A Multicall Builder

use crate::{Error, Result as ContractResult};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolCall;

mod bindings;
pub use bindings::IMulticall3::{aggregateCall, Call, Call3};

mod inner_types;
pub use inner_types::{CallInfo, DecodeReturns, Identity, Stack};

/// A multicall builder
#[derive(Debug)]
pub struct MulticallBuilder<T, P: Provider<N>, N: Network> {
    calls: T,
    provider: P,
    _pd: std::marker::PhantomData<N>,
}

impl<P, N> MulticallBuilder<Identity, P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Create a new multicall builder
    pub fn new(provider: P) -> Self {
        Self { calls: Identity, provider, _pd: Default::default() }
    }
}

impl<T, P, N> MulticallBuilder<T, P, N>
where
    T: Iterator<Item = Call3> + DecodeReturns,
    P: Provider<N>,
    N: Network,
{
    /// Add a call to the stack
    pub fn add<C: SolCall>(self, call: C, target: Address) -> MulticallBuilder<Stack<C, T>, P, N> {
        let stack = Stack::new(CallInfo::new(target, call), self.calls);
        MulticallBuilder { calls: stack, provider: self.provider, _pd: Default::default() }
    }

    /// Call the aggregate function
    pub async fn call_aggregate(self) -> ContractResult<(U256, T::Returns)> {
        let calls = &self
            .calls
            .map(|c| Call { target: c.target, callData: c.callData.clone() })
            .collect::<Vec<_>>();

        let call = aggregateCall { calls: calls.to_vec() }.abi_encode();

        let tx = N::TransactionRequest::default()
            .with_to(address!("cA11bde05977b3631167028862bE2a173976CA11"))
            .with_input(Bytes::from_iter(call));

        let res = self.provider.call(&tx).await.map_err(Error::TransportError)?;

        let mut output = aggregateCall::abi_decode_returns(&res, true)?;

        // Reverse the order of the return data to maintain the stack order consistency.
        output.returnData.reverse();

        Ok((output.blockNumber, T::decode_returns(&output.returnData)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_provider::ProviderBuilder;
    use alloy_sol_types::sol;
    sol! {
        #[derive(Debug, PartialEq)]
        interface ERC20 {
            function totalSupply() external view returns (uint256 totalSupply);
            function balanceOf(address owner) external view returns (uint256 balance);
        }
    }
    #[tokio::test]
    async fn test_stack() {
        let left = ERC20::totalSupplyCall {};
        let right =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider =
            ProviderBuilder::new().on_anvil_with_config(|a| a.fork("https://eth.merkle.io"));
        let multicall = MulticallBuilder::new(provider).add(left, weth).add(right, weth);

        // TODO: Pretty ugly, flatten return stack tuple
        let (block_number, (((), total_supply), balance)) =
            multicall.call_aggregate().await.unwrap();

        println!("block_number: {:?}", block_number);
        println!("balance: {:?}", balance.balance);
        println!("total_supply: {:?}", total_supply.totalSupply);
    }
}
