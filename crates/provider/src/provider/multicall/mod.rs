//! A Multicall Builder

use crate::Provider;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{address, Address, BlockNumber, Bytes, U256};
use alloy_rpc_types_eth::{state::StateOverride, BlockId};
use alloy_sol_types::SolCall;

/// Multicall bindings
pub mod bindings;
use crate::provider::multicall::bindings::IMulticall3::{
    aggregate3Call, aggregate3ValueCall, aggregateCall, getBasefeeCall, getBlockHashCall,
    getBlockNumberCall, getChainIdCall, getCurrentBlockCoinbaseCall, getCurrentBlockDifficultyCall,
    getCurrentBlockGasLimitCall, getCurrentBlockTimestampCall, getEthBalanceCall,
    getLastBlockHashCall, tryAggregateCall, tryAggregateReturn,
};

mod inner_types;
pub use inner_types::{CallInfo, CallInfoTrait, Failure, MulticallError, Result};

mod tuple;
pub use tuple::CallTuple;
use tuple::TuplePush;

/// Default address for the Multicall3 contract on most chains. See: <https://github.com/mds1/multicall>
pub const MULTICALL3_ADDRESS: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");

/// A Multicall3 builder
///
/// This builder implements a simple API interface to build and execute multicalls using the
/// [`IMultiCall3`](crate::multicall::bindings::IMulticall3) contract which is available on 270+
/// chains.
///
/// ## Example
///
/// ```ignore
/// use alloy_contract::MulticallBuilder;
/// use alloy_primitives::address;
/// use alloy_provider::ProviderBuilder;
/// use alloy_sol_types::sol;
///
/// sol! {
///    #[derive(Debug, PartialEq)]
///    interface ERC20 {
///        function totalSupply() external view returns (uint256 totalSupply);
///        function balanceOf(address owner) external view returns (uint256 balance);
///    }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let ts_call = ERC20::totalSupplyCall {};
///     let balance_call =
///         ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };
///
///     let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
///     let provider =
///         ProviderBuilder::new().on_anvil_with_config(|a| a.fork("https://eth.merkle.io"));
///
///     let multicall = MulticallBuilder::new(provider).add(ts_call, weth).add(balance_call, weth);
///
///     let (_block_num, (total_supply, balance)) = multicall.aggregate().await.unwrap();
///
///     println!("Total Supply: {:?}, Balance: {:?}", total_supply, balance);
/// }
/// ```
#[derive(Debug)]
pub struct MulticallBuilder<T: CallTuple, P: Provider<N>, N: Network> {
    /// Batched calls
    calls: Vec<Box<dyn CallInfoTrait>>,
    /// The provider to use
    provider: P,
    /// The [`BlockId`] to use for the call
    block: Option<BlockId>,
    /// The [`StateOverride`] for the call
    state_override: Option<StateOverride>,
    /// This is the address of the [`IMulticall3`](crate::multicall::bindings::IMulticall3)
    /// contract.
    ///
    /// By default it is set to [`MULTICALL3_ADDRESS`].
    address: Address,
    _pd: std::marker::PhantomData<(T, N)>,
}

impl<P, N> MulticallBuilder<(), P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Instantiate a new [`MulticallBuilder`]
    pub fn new(provider: P) -> Self {
        Self {
            calls: Vec::new(),
            provider,
            _pd: Default::default(),
            block: None,
            state_override: None,
            address: MULTICALL3_ADDRESS,
        }
    }
}

impl<T, P, N> MulticallBuilder<T, P, N>
where
    T: CallTuple,
    P: Provider<N>,
    N: Network,
{
    /// Set the address of the multicall3 contract
    ///
    /// Default is [`MULTICALL3_ADDRESS`].
    pub fn address(mut self, address: Address) -> Self {
        self.address = address;
        self
    }

    /// Sets the block to be used for the call.
    pub fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the state overrides for the call.
    pub fn overrides(mut self, state_override: StateOverride) -> Self {
        self.state_override = Some(state_override);
        self
    }

    /// Appends a [`SolCall`] to the stack.
    ///
    /// `target` is the address of the contract to call.
    pub fn add<C: SolCall + 'static>(
        mut self,
        call: C,
        target: Address,
    ) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<C>,
        <T as TuplePush<C>>::Pushed: CallTuple,
    {
        let call = CallInfo::new(call, target);

        self.calls.push(Box::new(call));
        MulticallBuilder {
            calls: self.calls,
            provider: self.provider,
            block: self.block,
            state_override: self.state_override,
            address: self.address,
            _pd: Default::default(),
        }
    }

    /// Appends a [`CallInfo`] to the stack.
    pub fn add_call<C: SolCall + 'static>(
        mut self,
        call: CallInfo<C>,
    ) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<C>,
        <T as TuplePush<C>>::Pushed: CallTuple,
    {
        self.calls.push(Box::new(call));
        MulticallBuilder {
            calls: self.calls,
            provider: self.provider,
            block: self.block,
            state_override: self.state_override,
            address: self.address,
            _pd: Default::default(),
        }
    }

    /// Calls the `aggregate` function
    ///
    /// Requires that all calls succeed, else reverts.
    ///
    /// ## Solidity Function Signature
    ///
    /// ```no_run
    /// sol! {
    ///     function aggregate(Call[] memory calls) external returns (uint256 blockNumber, bytes[] memory returnData);
    /// }
    /// ```
    ///
    /// ## Returns
    ///
    /// - (`blockNumber`, `returnData`):
    /// - `blockNumber`: The block number of the call
    /// - `returnData`: A tuple of the decoded return values for the calls
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use alloy_primitives::address;
    /// use alloy_provider::{ext::MulticallApi, MulticallBuilder, ProviderBuilder};
    /// use alloy_sol_types::sol;
    ///
    /// sol! {
    ///    #[derive(Debug, PartialEq)]
    ///    interface ERC20 {
    ///        function totalSupply() external view returns (uint256 totalSupply);
    ///        function balanceOf(address owner) external view returns (uint256 balance);
    ///    }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ts_call = ERC20::totalSupplyCall {};
    ///     let balance_call =
    ///         ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };
    ///
    ///     let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    ///     let provider = ProviderBuilder::new().on_http("https://eth.merkle.io".parse().unwrap());
    ///
    ///     let multicall = provider.multicall(ts_call, weth).add(balance_call, weth);
    ///
    ///     let (_block_num, (total_supply, balance)) = multicall.aggregate().await.unwrap();
    ///
    ///     println!("Total Supply: {:?}, Balance: {:?}", total_supply, balance);
    /// }
    /// ```
    pub async fn aggregate(&self) -> Result<(U256, T::SuccessReturns)> {
        let calls = self.calls.iter().map(|c| c.to_call()).collect::<Vec<_>>();
        let call = aggregateCall { calls: calls.to_vec() };
        let output = self.build_and_call(call, None).await?;
        Ok((output.blockNumber, T::decode_returns(&output.returnData)?))
    }

    /// Call the `tryAggregate` function
    ///
    /// Allows for calls to fail by setting `require_success` to false.
    ///
    /// ## Solidity Function Signature
    ///
    /// ```no_run
    /// sol! {
    ///     function tryAggregate(bool requireSuccess, Call[] calldata calls) external payable returns (Result[] memory returnData);
    /// }
    /// ```
    ///
    /// ## Returns
    ///
    /// - A tuple of the decoded return values for the calls.
    /// - Each return value is wrapped in a [`Result`] struct.
    /// - The [`Result::Ok`] variant contains the decoded return value.
    /// - The [`Result::Err`] variant contains the [`Failure`] struct which holds the
    ///   index(-position) of the call and the returned data as [`Bytes`].
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use alloy_primitives::address;
    /// use alloy_provider::{ext::MulticallApi, MulticallBuilder, ProviderBuilder};
    /// use alloy_sol_types::sol;
    ///
    /// sol! {
    ///   #[derive(Debug, PartialEq)]
    ///  interface ERC20 {
    ///     function totalSupply() external view returns (uint256 totalSupply);
    ///     function balanceOf(address owner) external view returns (uint256 balance);
    ///  }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let ts_call = ERC20::totalSupplyCall {};
    ///    let balance_call = ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };
    ///
    ///    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    ///
    ///    let provider = ProviderBuilder::new().on_http("https://eth.merkle.io".parse().unwrap());
    ///
    ///    let multicall = provider.multicall(ts_call, weth).add(balance_call, weth);
    ///
    ///    let (total_supply, balance) = multicall.try_aggregate(true).await.unwrap();
    ///     
    ///    // Unwrap Result<totalSupplyReturn, Failure>
    ///    let total_supply = total_supply.unwrap();
    ///    // Unwrap Result<balanceOfReturn, Failure>
    ///    let balance = balance.unwrap();
    /// }
    pub async fn try_aggregate(&self, require_success: bool) -> Result<T::Returns> {
        let calls = &self.calls.iter().map(|c| c.to_call()).collect::<Vec<_>>();
        let call = tryAggregateCall { requireSuccess: require_success, calls: calls.to_vec() };
        let output = self.build_and_call(call, None).await?;
        let tryAggregateReturn { returnData } = output;
        T::decode_return_results(&returnData)
    }

    /// Call the `aggregate3` function
    ///
    /// Doesn't require that all calls succeed, reverts only if a call with `allowFailure` set to
    /// false, fails.
    ///
    /// By default, adding a call via [`MulticallBuilder::add`] sets `allow_failure` to false.
    ///
    /// You can add a call that allows failure by using [`MulticallBuilder::add_call`], and setting
    /// `allow_failure` to true in [`CallInfo`].
    ///
    /// ## Solidity Function Signature
    ///
    /// ```ignore
    /// sol! {
    ///     function aggregate3(Call3[] calldata calls) external payable returns (Result[] memory returnData);
    /// }
    /// ```
    ///
    /// ## Returns
    ///
    /// - A tuple of the decoded return values for the calls.
    /// - Each return value is wrapped in a [`Result`] struct.
    /// - The [`Result::Ok`] variant contains the decoded return value.
    /// - The [`Result::Err`] variant contains the [`Failure`] struct which holds the
    ///   index(-position) of the call and the returned data as [`Bytes`].
    pub async fn aggregate3(&self) -> Result<T::Returns> {
        let calls = self.calls.iter().map(|c| c.to_call3()).collect::<Vec<_>>();
        let call = aggregate3Call { calls: calls.to_vec() };
        let output = self.build_and_call(call, None).await?;
        T::decode_return_results(&output.returnData)
    }

    /// Call the `aggregate3Value` function
    ///
    /// Similar to `aggregate3` allows for calls to fail. Moreover, it allows for calling into
    /// `payable` functions with the `value` parameter.
    ///
    /// One can set the `value` field in the [`CallInfo`] struct and use
    /// [`MulticallBuilder::add_call`] to add it to the stack.
    ///
    /// It is important to note the `aggregate3Value` only succeeds when `msg.value` is _strictly_
    /// equal to the sum of the values of all calls. Summing up the values of all calls and setting
    /// it in the transaction request is handled internally by the builder.
    ///
    /// ## Solidity Function Signature
    ///
    /// ```ignore
    /// sol! {
    ///    function aggregate3Value(Call3Value[] calldata calls) external payable returns (Result[] memory returnData);
    /// }
    /// ```
    ///
    /// ## Returns
    ///
    /// - A tuple of the decoded return values for the calls.
    /// - Each return value is wrapped in a [`Result`] struct.
    /// - The [`Result::Ok`] variant contains the decoded return value.
    /// - The [`Result::Err`] variant contains the [`Failure`] struct which holds the
    ///   index(-position) of the call and the returned data as [`Bytes`].
    pub async fn aggregate3_value(&self) -> Result<T::Returns> {
        let calls = self.calls.iter().map(|c| c.to_call3_value()).collect::<Vec<_>>();
        let total_value = calls.iter().map(|c| c.value).fold(U256::ZERO, |acc, x| acc + x);
        let call = aggregate3ValueCall { calls: calls.to_vec() };
        let output = self.build_and_call(call, Some(total_value)).await?;
        T::decode_return_results(&output.returnData)
    }

    /// Helper fn to build a tx and call the multicall contract
    ///
    /// ## Params
    ///
    /// - `call_type`: The [`SolCall`] being made.
    /// - `value`: Total value to send with the call in case of `aggregate3Value` request.
    async fn build_and_call<M: SolCall>(
        &self,
        call_type: M,
        value: Option<U256>,
    ) -> Result<M::Return> {
        let call = call_type.abi_encode();
        let mut tx = N::TransactionRequest::default()
            .with_to(self.address)
            .with_input(Bytes::from_iter(call));

        if let Some(value) = value {
            tx.set_value(value);
        }

        let mut eth_call = self.provider.root().call(&tx);

        if let Some(block) = self.block {
            eth_call = eth_call.block(block);
        }

        if let Some(overrides) = &self.state_override {
            eth_call = eth_call.overrides(overrides);
        }

        let res = eth_call.await.map_err(MulticallError::TransportError)?;
        M::abi_decode_returns(&res, true).map_err(MulticallError::DecodeError)
    }

    /// Add a call to get the block hash from a block number
    pub fn add_get_block_hash(self, number: BlockNumber) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getBlockHashCall>,
        T::Pushed: CallTuple,
    {
        let call =
            CallInfo::new(getBlockHashCall { blockNumber: U256::from(number) }, self.address);
        self.add_call(call)
    }

    /// Add a call to get the coinbase of the current block
    pub fn add_get_current_block_coinbase(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getCurrentBlockCoinbaseCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getCurrentBlockCoinbaseCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the current block number
    pub fn add_get_block_number(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getBlockNumberCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getBlockNumberCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the current block difficulty
    pub fn add_get_current_block_difficulty(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getCurrentBlockDifficultyCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getCurrentBlockDifficultyCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the current block gas limit
    pub fn add_get_current_block_gas_limit(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getCurrentBlockGasLimitCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getCurrentBlockGasLimitCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the current block timestamp
    pub fn add_get_current_block_timestamp(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getCurrentBlockTimestampCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getCurrentBlockTimestampCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the chain id
    pub fn add_get_chain_id(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getChainIdCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getChainIdCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the base fee
    pub fn add_get_base_fee(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getBasefeeCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getBasefeeCall {}, self.address);
        self.add_call(call)
    }

    /// Add a call to get the eth balance of an address
    pub fn add_get_eth_balance(self, address: Address) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getEthBalanceCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getEthBalanceCall { addr: address }, self.address);
        self.add_call(call)
    }

    /// Add a call to get the last block hash
    pub fn add_get_last_block_hash(self) -> MulticallBuilder<T::Pushed, P, N>
    where
        T: TuplePush<getLastBlockHashCall>,
        T::Pushed: CallTuple,
    {
        let call = CallInfo::new(getLastBlockHashCall {}, self.address);
        self.add_call(call)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Failure, ProviderBuilder};
    use alloy_primitives::b256;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_sol_types::sol;
    use DummyThatFails::failCall;
    use PayableCounter::{counterCall, incrementCall};

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
        #[sol(bytecode = "6080604052348015600e575f80fd5b5060a780601a5f395ff3fe6080604052348015600e575f80fd5b50600436106030575f3560e01c80630b93381b146034578063a9cc4718146036575b5f80fd5b005b603460405162461bcd60e51b815260040160689060208082526004908201526319985a5b60e21b604082015260600190565b60405180910390fdfea2646970667358221220c90ee107375422bb3516f4f13cdd754387c374edb5d9815fb6aa5ca111a77cb264736f6c63430008190033")]
        #[derive(Debug)]
        contract DummyThatFails {
            function fail() external {
                revert("fail");
            }

            function success() external {}
        }
    }

    async fn deploy_dummy(provider: impl crate::Provider) -> Address {
        let tx = TransactionRequest::default().with_deploy_code(DummyThatFails::BYTECODE.clone());
        let tx = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();
        tx.contract_address.unwrap()
    }

    const FORK_URL: &str = "https://eth-mainnet.alchemyapi.io/v2/jGiK5vwDfC3F4r0bqukm-W2GqgdrxdSr";

    #[tokio::test]
    async fn test_single() {
        let ts_call = ERC20::totalSupplyCall {};
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider = ProviderBuilder::new().on_anvil_with_config(|a| a.fork(FORK_URL));

        let multicall = MulticallBuilder::new(provider).add(ts_call, weth);

        let (_block_num, (_total_supply,)) = multicall.aggregate().await.unwrap();
    }

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

        let (_block_num, (t1, b1, t2, b2)) = multicall.aggregate().await.unwrap();

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

        let (_t1, _b1, _t2, _b2) = multicall.try_aggregate(true).await.unwrap();
    }

    #[tokio::test]
    async fn aggregate3() {
        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

        let provider =
            ProviderBuilder::new().on_anvil_with_wallet_and_config(|a| a.fork(FORK_URL)).unwrap();

        let dummy_addr = deploy_dummy(provider.clone()).await;
        let multicall = MulticallBuilder::new(provider.clone())
            .add(ts_call.clone(), weth)
            .add(balance_call.clone(), weth)
            .add(failCall {}, dummy_addr); // Failing call that will revert the multicall.

        let err = multicall.aggregate3().await.unwrap_err();

        assert!(err.to_string().contains("revert: Multicall3: call failed"));

        let failing_call = CallInfo::new(failCall {}, dummy_addr).allow_failure(true);
        let multicall = MulticallBuilder::new(provider)
            .add(ts_call, weth)
            .add(balance_call, weth)
            .add_call(failing_call);
        let (t1, b1, failure) = multicall.aggregate3().await.unwrap();

        assert!(t1.is_ok());
        assert!(b1.is_ok());
        let err = failure.unwrap_err();
        assert!(matches!(err, Failure { idx: 2, return_data: _ }));
    }

    #[tokio::test]
    async fn test_try_aggregate_fail() {
        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider =
            ProviderBuilder::new().on_anvil_with_wallet_and_config(|a| a.fork(FORK_URL)).unwrap();

        let dummy_addr = deploy_dummy(provider.clone()).await;
        let multicall = MulticallBuilder::new(provider)
            .add(ts_call.clone(), weth)
            .add(balance_call.clone(), weth)
            .add(ts_call.clone(), weth)
            .add(balance_call, weth)
            .add(failCall {}, dummy_addr); // Failing call that will revert the multicall.

        let err = multicall.try_aggregate(true).await.unwrap_err();

        assert!(err.to_string().contains("revert: Multicall3: call failed"));

        let (t1, b1, t2, b2, failure) = multicall.try_aggregate(false).await.unwrap();

        assert!(t1.is_ok());
        assert!(b1.is_ok());
        assert!(t2.is_ok());
        assert!(b2.is_ok());
        let err = failure.unwrap_err();
        assert!(matches!(err, Failure { idx: 4, return_data: _ }));
    }

    #[tokio::test]
    async fn test_util() {
        let ts_call = ERC20::totalSupplyCall {};
        let balance_call =
            ERC20::balanceOfCall { owner: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045") };

        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let provider = ProviderBuilder::new()
            .on_anvil_with_config(|a| a.fork(FORK_URL).fork_block_number(21787144));
        let multicall = MulticallBuilder::new(provider)
            .add(ts_call.clone(), weth)
            .add(balance_call.clone(), weth)
            .add(ts_call.clone(), weth)
            .add(balance_call, weth)
            .add_get_block_hash(21787144);

        let (_block_num, (t1, b1, t2, b2, block_hash)) = multicall.aggregate().await.unwrap();

        assert_eq!(t1, t2);
        assert_eq!(b1, b2);
        assert_eq!(
            block_hash.blockHash,
            b256!("31be03d4fb9a280d1699f1004f340573cd6d717dae79095d382e876415cb26ba")
        );
    }

    sol! {
        // solc 0.8.25; solc PayableCounter.sol --optimize --bin
        #[sol(bytecode = "6080604052348015600e575f80fd5b5061012c8061001c5f395ff3fe6080604052600436106025575f3560e01c806361bc221a146029578063d09de08a14604d575b5f80fd5b3480156033575f80fd5b50603b5f5481565b60405190815260200160405180910390f35b60536055565b005b5f341160bc5760405162461bcd60e51b815260206004820152602c60248201527f50617961626c65436f756e7465723a2076616c7565206d75737420626520677260448201526b06561746572207468616e20360a41b606482015260840160405180910390fd5b60015f8082825460cb919060d2565b9091555050565b8082018082111560f057634e487b7160e01b5f52601160045260245ffd5b9291505056fea264697066735822122064d656316647d3dc48d7ef0466bd10bc87694802a673183058725926a5190a5564736f6c63430008190033")]
        #[derive(Debug)]
        contract PayableCounter {
            uint256 public counter;

            function increment() public payable {
                require(msg.value > 0, "PayableCounter: value must be greater than 0");
                counter += 1;
            }
        }
    }

    #[tokio::test]
    async fn aggregate3_value() {
        let provider =
            ProviderBuilder::new().on_anvil_with_wallet_and_config(|a| a.fork(FORK_URL)).unwrap();

        let tx = TransactionRequest::default().with_deploy_code(PayableCounter::BYTECODE.clone());
        let tx = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();
        let counter_addr = tx.contract_address.unwrap();

        let increment_call = CallInfo::new(incrementCall {}, counter_addr).value(U256::from(1));

        let multicall = MulticallBuilder::new(provider.clone())
            .add(counterCall {}, counter_addr)
            .add_call(increment_call)
            .add(counterCall {}, counter_addr);

        let (c1, inc, c2) = multicall.aggregate3_value().await.unwrap();

        assert_eq!(c1.unwrap().counter, U256::ZERO);
        assert!(inc.is_ok());
        assert_eq!(c2.unwrap().counter, U256::from(1));

        // Allow failure - due to no value being sent
        let increment_call = CallInfo::new(incrementCall {}, counter_addr).allow_failure(true);

        let multicall = MulticallBuilder::new(provider)
            .add(counterCall {}, counter_addr)
            .add_call(increment_call)
            .add(counterCall {}, counter_addr);

        let (c1, inc, c2) = multicall.aggregate3_value().await.unwrap();

        assert_eq!(c1.unwrap().counter, U256::ZERO);
        assert!(inc.is_err_and(|failure| matches!(failure, Failure { idx: 1, return_data: _ })));
        assert_eq!(c2.unwrap().counter, U256::ZERO);
    }
}
