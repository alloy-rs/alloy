//! A Multicall Builder

use crate::{Error, MulticallError, Result as ContractResult};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::{TransactionInput, TransactionRequest};
use alloy_sol_types::{sol, SolCall};

/// No-op identity call.
#[derive(Debug)]
pub struct Identity;

sol! {
    function identity() public;
}

impl Iterator for Identity {
    type Item = Call3;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// A call that should be mapped into the relevant aggregate, aggregate3, aggregate3Value input
/// structs.
#[derive(Debug, Clone, Default)]
pub struct CallInfo<C: SolCall> {
    target: Address,
    allow_failure: bool,
    value: Option<U256>,
    call: C,
}

impl<C: SolCall> CallInfo<C> {
    /// Create a new [`CallInfo`] instance.
    pub fn new(target: Address, call: C) -> Self {
        Self { target, call, allow_failure: false, value: None }
    }

    /// ABI-decode the return data.
    pub fn decode(&self, data: &[u8]) -> ContractResult<C::Return> {
        C::abi_decode_returns(data, true)
            .map_err(|e| Error::MulticallError(MulticallError::DecodeError(e)))
    }
}

/// A stack of calls
#[derive(Debug)]
pub struct Stack<L: SolCall, R> {
    left: CallInfo<L>,
    right: R,
    /// Used as a flag to return the left call while iterating.
    empty: bool,
}

impl<L, R> Stack<L, R>
where
    L: SolCall,
{
    /// Create a new stack
    pub fn new(left: CallInfo<L>, right: R) -> Self {
        Self { left, right, empty: false }
    }

    /// Get the left call
    pub fn left(&self) -> &CallInfo<L> {
        &self.left
    }

    /// Get the right calls
    pub fn right(&self) -> &R {
        &self.right
    }

    /// Check if the stack contains an identity call
    pub fn contains_identity(&self) -> bool {
        std::mem::size_of::<R>() == 0
    }
}

impl<'a, L, R> Iterator for Stack<L, R>
where
    L: SolCall,
    R: Iterator<Item = Call3>,
{
    type Item = Call3;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(call) = self.right.next() {
            // First get all items from right
            Some(call)
        } else if !self.empty {
            // Then return left call (only once)
            self.empty = true;
            Some(Call3 {
                target: self.left.target,
                allowFailure: false,
                callData: self.left.call.abi_encode().into(),
            })
        } else {
            None
        }
    }
}

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

sol! {
    #[derive(Debug, PartialEq)]
    struct Call {
        address target;
        bytes callData;
    }

    #[derive(Debug, PartialEq)]
    struct Call3 {
        address target;
        bool allowFailure;
        bytes callData;
    }

    function aggregate(Call[] calldata calls) public payable returns (uint256 blockNumber, bytes[] memory returnData);

    #[derive(Debug, PartialEq)]
    struct Result {
        bool success;
        bytes returnData;
    }

    function aggregate3(Call3[] calldata calls) public payable returns (Result[] memory returnData);
}

impl<T, P, N> MulticallBuilder<T, P, N>
where
    T: Iterator<Item = Call3>,
    P: Provider<N>,
    N: Network,
{
    /// Add a call to the stack
    pub fn add<C: SolCall>(self, call: C, target: Address) -> MulticallBuilder<Stack<C, T>, P, N> {
        let stack = Stack::new(CallInfo::new(target, call), self.calls);
        MulticallBuilder { calls: stack, provider: self.provider, _pd: Default::default() }
    }

    /// Call the aggregate function
    pub async fn call_aggregate(self) -> ContractResult<(U256, Vec<Bytes>)> {
        let calls = &self
            .calls
            .map(|c| Call { target: c.target, callData: c.callData.clone() })
            .collect::<Vec<_>>();

        let call = aggregateCall { calls: calls.to_vec() }.abi_encode();

        let tx = N::TransactionRequest::default()
            .with_to(address!("cA11bde05977b3631167028862bE2a173976CA11"))
            .with_input(Bytes::from_iter(call));

        let res = self.provider.call(&tx).await.map_err(|e| Error::TransportError(e))?;

        let output = aggregateCall::abi_decode_returns(&res, true)?;

        Ok((output.blockNumber, output.returnData))
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

        let (block_number, return_data) = multicall.call_aggregate().await.unwrap();
    }
}
