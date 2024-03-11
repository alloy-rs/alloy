use crate::{Error, Result};
use alloy_dyn_abi::{DynSolValue, FunctionExt, JsonAbiExt};
use alloy_json_abi::Function;
use alloy_network::{Network, ReceiptResponse, TransactionBuilder};
use alloy_primitives::{Address, Bytes, U256, U64};
use alloy_providers::Provider;
use alloy_rpc_types::{state::StateOverride, BlockId};
use alloy_sol_types::SolCall;
use alloy_transport::Transport;
use futures_util::TryFutureExt;
use std::{
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
};

/// [`CallBuilder`] using a [`SolCall`] type as the call decoder.
// NOTE: please avoid changing this type due to its use in the `sol!` macro.
pub type SolCallBuilder<N, T, P, C> = CallBuilder<N, T, P, PhantomData<C>>;

/// [`CallBuilder`] using a [`Function`] as the call decoder.
pub type DynCallBuilder<N, T, P> = CallBuilder<N, T, P, Function>;

/// [`CallBuilder`] that does not have a call decoder.
pub type RawCallBuilder<N, T, P> = CallBuilder<N, T, P, ()>;

mod private {
    pub trait Sealed {}
    impl Sealed for super::Function {}
    impl<C: super::SolCall> Sealed for super::PhantomData<C> {}
    impl Sealed for () {}
}

/// A trait for decoding the output of a contract function.
///
/// This trait is sealed and cannot be implemented manually.
/// It is an implementation detail of [`CallBuilder`].
pub trait CallDecoder: private::Sealed {
    // Not public API.

    /// The output type of the contract function.
    #[doc(hidden)]
    type CallOutput;

    /// Decodes the output of a contract function.
    #[doc(hidden)]
    fn abi_decode_output(&self, data: Bytes, validate: bool) -> Result<Self::CallOutput>;

    #[doc(hidden)]
    fn as_debug_field(&self) -> impl std::fmt::Debug;
}

impl CallDecoder for Function {
    type CallOutput = Vec<DynSolValue>;

    #[inline]
    fn abi_decode_output(&self, data: Bytes, validate: bool) -> Result<Self::CallOutput> {
        FunctionExt::abi_decode_output(self, &data, validate).map_err(Error::AbiError)
    }

    #[inline]
    fn as_debug_field(&self) -> impl std::fmt::Debug {
        self
    }
}

impl<C: SolCall> CallDecoder for PhantomData<C> {
    type CallOutput = C::Return;

    #[inline]
    fn abi_decode_output(&self, data: Bytes, validate: bool) -> Result<Self::CallOutput> {
        C::abi_decode_returns(&data, validate).map_err(|e| Error::AbiError(e.into()))
    }

    #[inline]
    fn as_debug_field(&self) -> impl std::fmt::Debug {
        std::any::type_name::<C>()
    }
}

impl CallDecoder for () {
    type CallOutput = Bytes;

    #[inline]
    fn abi_decode_output(&self, data: Bytes, _validate: bool) -> Result<Self::CallOutput> {
        Ok(data)
    }

    #[inline]
    fn as_debug_field(&self) -> impl std::fmt::Debug {
        format_args!("()")
    }
}

/// A builder for sending a transaction via `eth_sendTransaction`, or calling a contract via
/// `eth_call`.
///
/// The builder can be `.await`ed directly, which is equivalent to invoking [`call`].
/// Prefer using [`call`] when possible, as `await`ing the builder directly will consume it, and
/// currently also boxes the future due to type system limitations.
///
/// A call builder can currently be instantiated in the following ways:
/// - by [`sol!`][sol]-generated contract structs' methods (through the `#[sol(rpc)]` attribute)
///   ([`SolCallBuilder`]);
/// - by [`ContractInstance`](crate::ContractInstance)'s methods ([`DynCallBuilder`]);
/// - using [`CallBuilder::new_raw`] ([`RawCallBuilder`]).
///
/// Each method represents a different way to decode the output of the contract call.
///
/// [`call`]: CallBuilder::call
///
/// # Note
///
/// This will set [state overrides](https://geth.ethereum.org/docs/rpc/ns-eth#3-object---state-override-set)
/// for `eth_call`, but this is not supported by all clients.
///
/// # Examples
///
/// Using [`sol!`][sol]:
///
/// ```no_run
/// # async fn test<N: alloy_contract::private::Network, P: alloy_contract::private::Provider<N>>(provider: P) -> Result<(), Box<dyn std::error::Error>> {
/// use alloy_contract::SolCallBuilder;
/// use alloy_primitives::{Address, U256};
/// use alloy_sol_types::sol;
///
/// sol! {
///     #[sol(rpc)] // <-- Important!
///     contract MyContract {
///         function doStuff(uint a, bool b) public returns(address c, bytes32 d);
///     }
/// }
///
/// # stringify!(
/// let provider = ...;
/// # );
/// let address = Address::ZERO;
/// let contract = MyContract::new(address, &provider);
///
/// // Through `contract.<function_name>(args...)`
/// let a = U256::ZERO;
/// let b = true;
/// let builder: SolCallBuilder<_, _, _, MyContract::doStuffCall> = contract.doStuff(a, b);
/// let MyContract::doStuffReturn { c: _, d: _ } = builder.call().await?;
///
/// // Through `contract.call_builder(&<FunctionCall { args... }>)`:
/// // (note that this is discouraged because it's inherently less type-safe)
/// let call = MyContract::doStuffCall { a, b };
/// let builder: SolCallBuilder<_, _, _, MyContract::doStuffCall> = contract.call_builder(&call);
/// let MyContract::doStuffReturn { c: _, d: _ } = builder.call().await?;
/// # Ok(())
/// # }
/// ```
///
/// Using [`ContractInstance`](crate::ContractInstance):
///
/// ```no_run
/// # async fn test<N: alloy_contract::private::Network, P: alloy_contract::private::Provider<N>>(provider: P, dynamic_abi: alloy_json_abi::JsonAbi) -> Result<(), Box<dyn std::error::Error>> {
/// use alloy_primitives::{Address, Bytes, U256};
/// use alloy_dyn_abi::DynSolValue;
/// use alloy_contract::{CallBuilder, ContractInstance, DynCallBuilder, Interface, RawCallBuilder};
///
/// # stringify!(
/// let dynamic_abi: JsonAbi = ...;
/// # );
/// let interface = Interface::new(dynamic_abi);
///
/// # stringify!(
/// let provider = ...;
/// # );
/// let address = Address::ZERO;
/// let contract: ContractInstance<_, _, _> = interface.connect(address, &provider);
///
/// // Build and call the function:
/// let call_builder: DynCallBuilder<_, _, _> = contract.function("doStuff", &[U256::ZERO.into(), true.into()])?;
/// let result: Vec<DynSolValue> = call_builder.call().await?;
///
/// // You can also decode the output manually. Get the raw bytes:
/// let raw_result: Bytes = call_builder.call_raw().await?;
/// // Or, equivalently:
/// let raw_builder: RawCallBuilder<_, _, _> = call_builder.clone().clear_decoder();
/// let raw_result: Bytes = raw_builder.call().await?;
/// // Decode the raw bytes:
/// let decoded_result: Vec<DynSolValue> = call_builder.decode_output(raw_result, false)?;
/// # Ok(())
/// # }
/// ```
///
/// [sol]: alloy_sol_types::sol
#[derive(Clone)]
#[must_use = "call builders do nothing unless you `.call`, `.send`, or `.await` them"]
pub struct CallBuilder<N: Network, T, P, D> {
    request: N::TransactionRequest,
    block: Option<BlockId>,
    state: Option<StateOverride>,
    /// The provider.
    pub provider: P,
    decoder: D,
    transport: PhantomData<T>,
}

// See [`ContractInstance`].
impl<N: Network, T: Transport + Clone, P: Provider<N, T>> DynCallBuilder<N, T, P> {
    pub(crate) fn new_dyn(provider: P, function: &Function, args: &[DynSolValue]) -> Result<Self> {
        Ok(Self::new_inner(provider, function.abi_encode_input(args)?.into(), function.clone()))
    }

    /// Clears the decoder, returning a raw call builder.
    #[inline]
    pub fn clear_decoder(self) -> RawCallBuilder<N, T, P> {
        RawCallBuilder {
            request: self.request,
            block: self.block,
            state: self.state,
            provider: self.provider,
            decoder: (),
            transport: PhantomData,
        }
    }
}

#[doc(hidden)]
impl<'a, N: Network, T: Transport + Clone, P: Provider<N, T>, C: SolCall>
    SolCallBuilder<N, T, &'a P, C>
{
    // `sol!` macro constructor, see `#[sol(rpc)]`. Not public API.
    // NOTE: please avoid changing this function due to its use in the `sol!` macro.
    pub fn new_sol(provider: &'a P, address: &Address, call: &C) -> Self {
        Self::new_inner(provider, call.abi_encode().into(), PhantomData::<C>).to(Some(*address))
    }
}

impl<N: Network, T: Transport + Clone, P: Provider<N, T>, C: SolCall> SolCallBuilder<N, T, P, C> {
    /// Clears the decoder, returning a raw call builder.
    #[inline]
    pub fn clear_decoder(self) -> RawCallBuilder<N, T, P> {
        RawCallBuilder {
            request: self.request,
            block: self.block,
            state: self.state,
            provider: self.provider,
            decoder: (),
            transport: PhantomData,
        }
    }
}

impl<N: Network, T: Transport + Clone, P: Provider<N, T>> RawCallBuilder<N, T, P> {
    /// Creates a new call builder with the provided provider and ABI encoded input.
    ///
    /// Will not decode the output of the call, meaning that [`call`](Self::call) will behave the
    /// same as [`call_raw`](Self::call_raw).
    #[inline]
    pub fn new_raw(provider: P, input: Bytes) -> Self {
        Self::new_inner(provider, input, ())
    }
}

impl<N: Network, T: Transport + Clone, P: Provider<N, T>, D: CallDecoder> CallBuilder<N, T, P, D> {
    fn new_inner(provider: P, input: Bytes, decoder: D) -> Self {
        Self {
            request: <N::TransactionRequest>::default().with_input(input),
            decoder,
            provider,
            block: None,
            state: None,
            transport: PhantomData,
        }
    }

    /// Sets the `from` field in the transaction to the provided value. Defaults to [Address::ZERO].
    pub fn from(mut self, from: Address) -> Self {
        self.request.set_from(from);
        self
    }

    /// Sets the `to` field in the transaction to the provided address.
    pub fn to(mut self, to: Option<Address>) -> Self {
        self.request.set_to(to.into());
        self
    }

    /// Uses a Legacy transaction instead of an EIP-1559 one to execute the call
    pub fn legacy(self) -> Self {
        todo!()
    }

    /// Sets the `gas` field in the transaction to the provided value
    pub fn gas(mut self, gas: U256) -> Self {
        self.request.set_gas_limit(gas);
        self
    }

    /// Sets the `gas_price` field in the transaction to the provided value
    /// If the internal transaction is an EIP-1559 one, then it sets both
    /// `max_fee_per_gas` and `max_priority_fee_per_gas` to the same value
    pub fn gas_price(mut self, gas_price: U256) -> Self {
        self.request.set_gas_price(gas_price);
        self
    }

    /// Sets the `value` field in the transaction to the provided value
    pub fn value(mut self, value: U256) -> Self {
        self.request.set_value(value);
        self
    }

    /// Sets the `nonce` field in the transaction to the provided value
    pub fn nonce(mut self, nonce: U64) -> Self {
        self.request.set_nonce(nonce);
        self
    }

    /// Applies a function to the internal transaction request.
    pub fn map<F>(mut self, f: F) -> Self
    where
        F: FnOnce(N::TransactionRequest) -> N::TransactionRequest,
    {
        self.request = f(self.request);
        self
    }

    /// Sets the `block` field for sending the tx to the chain
    pub const fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the [state override set](https://geth.ethereum.org/docs/rpc/ns-eth#3-object---state-override-set).
    ///
    /// # Note
    ///
    /// Not all client implementations will support this as a parameter to `eth_call`.
    pub fn state(mut self, state: StateOverride) -> Self {
        self.state = Some(state);
        self
    }

    /// Returns the underlying transaction's ABI-encoded data.
    pub fn calldata(&self) -> &Bytes {
        self.request.input().expect("set in the constructor")
    }

    /// Returns the estimated gas cost for the underlying transaction to be executed
    pub async fn estimate_gas(&self) -> Result<U256> {
        self.provider.estimate_gas(&self.request, self.block).await.map_err(Into::into)
    }

    /// Queries the blockchain via an `eth_call` without submitting a transaction to the network.
    ///
    /// Returns the decoded the output by using the provided decoder.
    /// If this is not desired, use [`call_raw`](Self::call_raw) to get the raw output data.
    pub async fn call(&self) -> Result<D::CallOutput> {
        let data = self.call_raw().await?;
        self.decode_output(data, false)
    }

    /// Queries the blockchain via an `eth_call` without submitting a transaction to the network.
    ///
    /// Does not decode the output of the call, returning the raw output data instead.
    ///
    /// See [`call`](Self::call) for more information.
    pub async fn call_raw(&self) -> Result<Bytes> {
        if let Some(state) = &self.state {
            self.provider.call_with_overrides(&self.request, self.block, state.clone()).await
        } else {
            self.provider.call(&self.request, self.block).await
        }
        .map_err(Into::into)
    }

    /// Decodes the output of a contract function using the provided decoder.
    #[inline]
    pub fn decode_output(&self, data: Bytes, validate: bool) -> Result<D::CallOutput> {
        self.decoder.abi_decode_output(data, validate)
    }

    /// Broadcasts the underlying transaction to the network as a deployment transaction, returning
    /// the address of the deployed contract after the transaction has been confirmed.
    ///
    /// Returns an error if the transaction is not a deployment transaction, or if the contract
    /// address is not found in the deployment transaction’s receipt.
    ///
    /// For more fine-grained control over the deployment process, use [`send`](Self::send) instead.
    ///
    /// Note that the deployment address can be pre-calculated if the `from` address and `nonce` are
    /// known using [`calculate_create_address`](Self::calculate_create_address).
    pub async fn deploy(&self) -> Result<Address> {
        if !self.request.to().is_some_and(|to| to.is_create()) {
            return Err(Error::NotADeploymentTransaction);
        }
        let pending_tx = self.send().await?;
        let receipt = pending_tx.await?;
        receipt
            .ok_or(Error::ContractNotDeployed)?
            .contract_address()
            .ok_or(Error::ContractNotDeployed)
    }

    /// Broadcasts the underlying transaction to the network.
    ///
    /// See [`Provider::send_transaction`] for more information.
    pub async fn send(
        &self,
    ) -> Result<impl Future<Output = Result<Option<N::ReceiptResponse>>> + '_> {
        let config = self.provider.send_transaction(self.request.clone()).await?;
        Ok(config.with_provider(&self.provider).get_receipt().map_err(Into::into))
    }

    /// Calculates the address that will be created by the transaction, if any.
    ///
    /// Returns `None` if the transaction is not a contract creation (the `to` field is set), or if
    /// the `from` or `nonce` fields are not set.
    pub fn calculate_create_address(&self) -> Option<Address> {
        self.request.calculate_create_address()
    }
}

impl<N: Network, T: Transport, P: Clone, D> CallBuilder<N, T, &P, D> {
    /// Clones the provider and returns a new builder with the cloned provider.
    pub fn with_cloned_provider(self) -> CallBuilder<N, T, P, D> {
        CallBuilder {
            request: self.request,
            block: self.block,
            state: self.state,
            provider: self.provider.clone(),
            decoder: self.decoder,
            transport: PhantomData,
        }
    }
}

/// [`CallBuilder`] can be turned into a [`Future`] automatically with `.await`.
///
/// Defaults to calling [`CallBuilder::call`].
///
/// # Note
///
/// This requires `Self: 'static` due to a current limitation in the Rust type system, namely that
/// the associated future type, the returned future, must be a concrete type (`Box<dyn Future ...>`)
/// and cannot be an opaque type (`impl Future ...`) because `impl Trait` in this position is not
/// stable yet. See [rust-lang/rust#63063](https://github.com/rust-lang/rust/issues/63063).
impl<N, T, P, D> IntoFuture for CallBuilder<N, T, P, D>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<N, T>,
    D: CallDecoder + Send + Sync,
    Self: 'static,
{
    type Output = Result<D::CallOutput>;
    #[cfg(target_arch = "wasm32")]
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;
    #[cfg(not(target_arch = "wasm32"))]
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    #[inline]
    fn into_future(self) -> Self::IntoFuture {
        #[allow(clippy::redundant_async_block)]
        Box::pin(async move { self.call().await })
    }
}

impl<N: Network, T, P, D: CallDecoder> std::fmt::Debug for CallBuilder<N, T, P, D> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallBuilder")
            .field("request", &self.request)
            .field("block", &self.block)
            .field("state", &self.state)
            .field("decoder", &self.decoder.as_debug_field())
            .finish()
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use alloy_network::Ethereum;
    use alloy_node_bindings::{Anvil, AnvilInstance};
    use alloy_primitives::{address, b256, bytes, hex};
    use alloy_providers::{HttpProvider, Provider, RootProvider};
    use alloy_rpc_client::RpcClient;
    use alloy_sol_types::sol;
    use alloy_transport_http::Http;
    use reqwest::Client;

    fn spawn_anvil() -> (HttpProvider<Ethereum>, AnvilInstance) {
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        (RootProvider::<Ethereum, _>::new(RpcClient::new(http, true)), anvil)
    }

    #[test]
    fn empty_constructor() {
        sol! {
            #[sol(rpc, bytecode = "6942")]
            contract EmptyConstructor {
                constructor();
            }
        }

        let (provider, _anvil) = spawn_anvil();
        let call_builder = EmptyConstructor::deploy_builder(&provider);
        assert_eq!(*call_builder.calldata(), bytes!("6942"));
    }

    sol! {
        // Solc: 0.8.24+commit.e11b9ed9.Linux.g++
        // Command: solc a.sol --bin --via-ir --optimize --optimize-runs 1
        #[sol(rpc, bytecode = "60803461006357601f61014838819003918201601f19168301916001600160401b038311848410176100675780849260209460405283398101031261006357518015158091036100635760ff80195f54169116175f5560405160cc908161007c8239f35b5f80fd5b634e487b7160e01b5f52604160045260245ffdfe60808060405260043610156011575f80fd5b5f3560e01c9081638bf1799f14607a575063b09a261614602f575f80fd5b346076576040366003190112607657602435801515810360765715606f57604060015b81516004356001600160a01b0316815260ff919091166020820152f35b60405f6052565b5f80fd5b346076575f36600319011260765760209060ff5f541615158152f3fea264697066735822122043709781c9bdc30c530978abf5db25a4b4ccfebf989baafd2ba404519a7f7e8264736f6c63430008180033")]
        contract MyContract {
            bool public myState;

            constructor(bool myState_) {
                myState = myState_;
            }

            function doStuff(uint a, bool b) external pure returns(address c, bytes32 d) {
                return (address(uint160(a)), bytes32(uint256(b ? 1 : 0)));
            }
        }
    }

    #[test]
    fn call_encoding() {
        let (provider, _anvil) = spawn_anvil();
        let contract = MyContract::new(Address::ZERO, &&provider).with_cloned_provider();
        let call_builder = contract.doStuff(U256::ZERO, true).with_cloned_provider();
        assert_eq!(
            *call_builder.calldata(),
            bytes!(
                "b09a2616"
                "0000000000000000000000000000000000000000000000000000000000000000"
                "0000000000000000000000000000000000000000000000000000000000000001"
            ),
        );
        // Box the future to assert its concrete output type.
        let _future: Box<dyn Future<Output = Result<MyContract::doStuffReturn>> + Send> =
            Box::new(call_builder.call());
    }

    #[test]
    fn deploy_encoding() {
        let (provider, _anvil) = spawn_anvil();
        let bytecode = &MyContract::BYTECODE[..];
        let call_builder = MyContract::deploy_builder(&provider, false);
        assert_eq!(
            call_builder.calldata()[..],
            [
                bytecode,
                &hex!("0000000000000000000000000000000000000000000000000000000000000000")[..]
            ]
            .concat(),
        );
        let call_builder = MyContract::deploy_builder(&provider, true);
        assert_eq!(
            call_builder.calldata()[..],
            [
                bytecode,
                &hex!("0000000000000000000000000000000000000000000000000000000000000001")[..]
            ]
            .concat(),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deploy_and_call() {
        let (provider, anvil) = spawn_anvil();

        let my_contract = MyContract::deploy(provider, true).await.unwrap();
        let expected_address = anvil.addresses()[0].create(0);
        assert_eq!(*my_contract.address(), expected_address);

        let my_state_builder = my_contract.myState();
        assert_eq!(my_state_builder.calldata()[..], MyContract::myStateCall {}.abi_encode(),);
        let result: MyContract::myStateReturn = my_state_builder.call().await.unwrap();
        assert!(result._0);

        let do_stuff_builder = my_contract.doStuff(U256::from(0x69), true);
        assert_eq!(
            do_stuff_builder.calldata()[..],
            MyContract::doStuffCall { a: U256::from(0x69), b: true }.abi_encode(),
        );
        let result: MyContract::doStuffReturn = do_stuff_builder.call().await.unwrap();
        assert_eq!(result.c, address!("0000000000000000000000000000000000000069"));
        assert_eq!(
            result.d,
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
        );
    }
}
