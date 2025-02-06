//! A Multicall Builder

use crate::{Error, Result as ContractResult};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolCall;

mod bindings;
use bindings::IMulticall3::{
    aggregate3Call, aggregate3ValueCall, tryAggregateCall, tryAggregateReturn,
};
pub use bindings::IMulticall3::{aggregateCall, Call, Call3};

mod inner_types;
use crate::MulticallError;
pub use inner_types::CallInfo;
use inner_types::CallInfoTrait;
use tuple::{CallTuple, TuplePush};

mod tuple;

/// A multicall builder
#[derive(Debug)]
pub struct MulticallBuilder<T: CallTuple, P: Provider<N>, N: Network> {
    calls: Vec<Box<dyn CallInfoTrait>>,
    provider: P,
    _pd: std::marker::PhantomData<(T, N)>,
}

impl<P, N> MulticallBuilder<(), P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Create a new multicall builder
    pub fn new(provider: P) -> Self {
        Self { calls: Vec::new(), provider, _pd: Default::default() }
    }
}

impl<T, P, N> MulticallBuilder<T, P, N>
where
    T: CallTuple,
    P: Provider<N>,
    N: Network,
{
    /// Add a call to the stack
    pub fn add<C: SolCall + 'static>(
        mut self,
        call: C,
        target: Address,
    ) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<C>,
        <T as TuplePush<C>>::Pushed: CallTuple,
    {
        let call = CallInfo::new(target, call);

        self.calls.push(Box::new(call));
        MulticallBuilder { calls: self.calls, provider: self.provider, _pd: Default::default() }
    }

    /// Add [`CallInfo`] to the stack
    pub fn add_call<C: SolCall + 'static>(
        mut self,
        call: CallInfo<C>,
    ) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<C>,
        <T as TuplePush<C>>::Pushed: CallTuple,
    {
        self.calls.push(Box::new(call));
        MulticallBuilder { calls: self.calls, provider: self.provider, _pd: Default::default() }
    }

    /// Call the `aggregate` function
    ///
    /// Requires that all calls succeed.
    pub async fn call_aggregate(&self) -> ContractResult<(U256, T::SuccessReturns)> {
        let calls = self.calls.iter().map(|c| c.to_call()).collect::<Vec<_>>();
        let call = aggregateCall { calls: calls.to_vec() };
        let output = self.build_and_call(call, None).await?;
        Ok((output.blockNumber, T::decode_returns(&output.returnData)?))
    }

    /// Call the `tryAggregate` function
    ///
    /// Adds flexibility for calls to fail
    pub async fn call_try_aggregate(&self, require_success: bool) -> ContractResult<T::Returns> {
        let calls = &self.calls.iter().map(|c| c.to_call()).collect::<Vec<_>>();
        let call = tryAggregateCall { requireSuccess: require_success, calls: calls.to_vec() };
        let output = self.build_and_call(call, None).await?;
        let tryAggregateReturn { returnData } = output;
        T::decode_return_results(&returnData)
    }

    /// Call the `aggregate3` function
    pub async fn call_aggregate3(&self) -> ContractResult<T::Returns> {
        let calls = self.calls.iter().map(|c| c.to_call3()).collect::<Vec<_>>();
        let call = aggregate3Call { calls: calls.to_vec() };
        let output = self.build_and_call(call, None).await?;
        T::decode_return_results(&output.returnData)
    }

    /// Call the `aggregate3Value` function
    pub async fn call_aggregate3_value(&self) -> ContractResult<T::Returns> {
        let calls = self.calls.iter().map(|c| c.to_call3_value()).collect::<Vec<_>>();
        let total_value = calls.iter().map(|c| c.value).fold(U256::ZERO, |acc, x| acc + x);
        let call = aggregate3ValueCall { calls: calls.to_vec() };
        let output = self.build_and_call(call, Some(total_value)).await?;
        T::decode_return_results(&output.returnData)
    }

    async fn build_and_call<M: SolCall>(
        &self,
        call_type: M,
        value: Option<U256>,
    ) -> ContractResult<M::Return> {
        let call = call_type.abi_encode();
        let mut tx = N::TransactionRequest::default()
            .with_to(address!("cA11bde05977b3631167028862bE2a173976CA11"))
            .with_input(Bytes::from_iter(call));

        if let Some(value) = value {
            tx.set_value(value);
        }

        let res = self.provider.call(&tx).await.map_err(Error::TransportError)?;
        M::abi_decode_returns(&res, true)
            .map_err(|e| Error::MulticallError(MulticallError::DecodeError(e)))
    }
}

#[cfg(test)]
mod tests {
    use crate::multicall::tuple::Failure;

    use super::*;
    use alloy_provider::ProviderBuilder;
    use alloy_sol_types::sol;
    use DummyThatFails::{failCall, DummyThatFailsInstance};
    sol! {
        #[derive(Debug, PartialEq)]
        interface ERC20 {
            function totalSupply() external view returns (uint256 totalSupply);
            function balanceOf(address owner) external view returns (uint256 balance);
            function transfer(address to, uint256 value) external returns (bool);
        }
    }

    sol! {
        // solc 0.8.25; solc DummyThatFails.sol --optimize --bin
        #[sol(rpc, bytecode = "6080604052348015600e575f80fd5b5060a780601a5f395ff3fe6080604052348015600e575f80fd5b50600436106030575f3560e01c80630b93381b146034578063a9cc4718146036575b5f80fd5b005b603460405162461bcd60e51b815260040160689060208082526004908201526319985a5b60e21b604082015260600190565b60405180910390fdfea2646970667358221220c90ee107375422bb3516f4f13cdd754387c374edb5d9815fb6aa5ca111a77cb264736f6c63430008190033")]
        #[derive(Debug)]
        contract DummyThatFails {
            function fail() external {
                revert("fail");
            }

            function success() external {}
        }
    }

    async fn deploy_dummy(provider: impl Provider) -> DummyThatFailsInstance<(), impl Provider> {
        DummyThatFails::deploy(provider).await.unwrap()
    }

    const FORK_URL: &str = "https://eth-mainnet.alchemyapi.io/v2/jGiK5vwDfC3F4r0bqukm-W2GqgdrxdSr";
    #[tokio::test]
    async fn test_aggregate() {
        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider = ProviderBuilder::new().on_anvil_with_config(|a| a.fork(FORK_URL));
        let multicall = MulticallBuilder::new(provider)
            .add(ts_call.clone(), weth)
            .add(balance_call.clone(), weth)
            .add(ts_call.clone(), weth)
            .add(balance_call, weth);

        let (_block_num, (t1, b1, t2, b2)) = multicall.call_aggregate().await.unwrap();

        assert_eq!(t1, t2);
        assert_eq!(b1, b2);
    }

    #[tokio::test]
    async fn test_try_aggregate_pass() {
        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider = ProviderBuilder::new().on_anvil_with_config(|a| a.fork(FORK_URL));
        let multicall = MulticallBuilder::new(provider)
            .add(ts_call.clone(), weth)
            .add(balance_call.clone(), weth)
            .add(ts_call.clone(), weth)
            .add(balance_call, weth);

        let (_t1, _b1, _t2, _b2) = multicall.call_try_aggregate(true).await.unwrap();
    }

    #[tokio::test]
    async fn test_try_aggregate_fail() {
        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider =
            ProviderBuilder::new().on_anvil_with_wallet_and_config(|a| a.fork(FORK_URL)).unwrap();

        let dummy = deploy_dummy(provider.clone()).await;
        let dummy_addr = dummy.address();
        let multicall = MulticallBuilder::new(provider)
            .add(ts_call.clone(), weth)
            .add(balance_call.clone(), weth)
            .add(ts_call.clone(), weth)
            .add(balance_call, weth)
            .add(failCall {}, *dummy_addr); // Failing call that will revert the multicall.

        let err = multicall.call_try_aggregate(true).await.unwrap_err();

        assert!(err.to_string().contains("revert: Multicall3: call failed"));

        let (t1, b1, t2, b2, failure) = multicall.call_try_aggregate(false).await.unwrap();

        assert!(t1.is_ok());
        assert!(b1.is_ok());
        assert!(t2.is_ok());
        assert!(b2.is_ok());
        let err = failure.unwrap_err();
        assert!(matches!(err, Failure { idx: 4, return_data: _ }));
    }
}
