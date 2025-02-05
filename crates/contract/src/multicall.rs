//! A Multicall Builder

use crate::{Error, MulticallError, Result as ContractResult};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_sol_types::{sol, SolCall};

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

        let res = self.provider.call(&tx).await.map_err(|e| Error::TransportError(e))?;

        let mut output = aggregateCall::abi_decode_returns(&res, true)?;

        // Reverse the order of the return data to maintain the stack order consistency.
        output.returnData.reverse();

        Ok((output.blockNumber, T::decode_returns(&output.returnData)?))
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

/// Trait for decoding return values from a sequence of calls
pub trait DecodeReturns {
    /// Decoded Return Tuple
    type Returns;

    /// Decode the return values
    fn decode_returns(data: &[Bytes]) -> ContractResult<Self::Returns>;
}

impl DecodeReturns for Identity {
    type Returns = ();
    fn decode_returns(_data: &[Bytes]) -> ContractResult<Self::Returns> {
        Ok(())
    }
}

// Decode the stack recursively.
impl<L: SolCall, R: DecodeReturns> DecodeReturns for Stack<L, R> {
    type Returns = (R::Returns, L::Return); // Maintain call order.
    fn decode_returns(data: &[Bytes]) -> ContractResult<Self::Returns> {
        let (first, rest) =
            data.split_first().ok_or(Error::MulticallError(MulticallError::NoReturnData))?;

        // Recursively decode the rest of the stack.
        Ok((R::decode_returns(rest)?, L::abi_decode_returns(first, true)?))
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
