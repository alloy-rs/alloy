use crate::{Error, Result};
use alloy_dyn_abi::{DynSolValue, FunctionExt, JsonAbiExt};
use alloy_json_abi::Function;
use alloy_primitives::{Address, Bytes, U256, U64};
use alloy_providers::provider::TempProvider;
use alloy_rpc_types::{state::StateOverride, BlockId, CallInput, CallRequest};
use alloy_sol_types::SolCall;
use std::{
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
};

/// [`CallBuilder`] using a [`SolCall`] type as the call decoder.
// NOTE: please avoid changing this type due to its use in the `sol!` macro.
pub type SolCallBuilder<P, C> = CallBuilder<P, PhantomData<C>>;

/// [`CallBuilder`] using a [`Function`] as the call decoder.
pub type DynCallBuilder<P> = CallBuilder<P, Function>;

/// [`CallBuilder`] that does not have a call decoder.
pub type RawCallBuilder<P> = CallBuilder<P, ()>;

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
    type Output;

    /// Decodes the output of a contract function.
    #[doc(hidden)]
    fn abi_decode_output(&self, data: Bytes, validate: bool) -> Result<Self::Output>;

    #[doc(hidden)]
    fn as_debug_field(&self) -> impl std::fmt::Debug;
}

impl CallDecoder for Function {
    type Output = Vec<DynSolValue>;

    #[inline]
    fn abi_decode_output(&self, data: Bytes, validate: bool) -> Result<Self::Output> {
        FunctionExt::abi_decode_output(self, &data, validate).map_err(Error::AbiError)
    }

    #[inline]
    fn as_debug_field(&self) -> impl std::fmt::Debug {
        self
    }
}

impl<C: SolCall> CallDecoder for PhantomData<C> {
    type Output = C::Return;

    #[inline]
    fn abi_decode_output(&self, data: Bytes, validate: bool) -> Result<Self::Output> {
        C::abi_decode_returns(&data, validate).map_err(|e| Error::AbiError(e.into()))
    }

    #[inline]
    fn as_debug_field(&self) -> impl std::fmt::Debug {
        std::any::type_name::<C>()
    }
}

impl CallDecoder for () {
    type Output = Bytes;

    #[inline]
    fn abi_decode_output(&self, data: Bytes, _validate: bool) -> Result<Self::Output> {
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
/// ## Note
///
/// Sets the [state overrides](https://geth.ethereum.org/docs/rpc/ns-eth#3-object---state-override-set)
/// for `eth_call`, but this is not supported by all clients.
///
/// ## Examples
///
/// Using [`sol!`][sol]:
///
/// ```no_run
/// # async fn test<P: alloy_contract::private::Provider>(provider: P) -> Result<(), Box<dyn std::error::Error>> {
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
/// let builder: SolCallBuilder<_, MyContract::doStuffCall> = contract.doStuff(a, b);
/// let MyContract::doStuffReturn { c: _, d: _ } = builder.call().await?;
///
/// // Through `contract.call_builder(&<FunctionCall { args... }>)`:
/// // (note that this is discouraged because it's inherently less type-safe)
/// let call = MyContract::doStuffCall { a, b };
/// let builder: SolCallBuilder<_, MyContract::doStuffCall> = contract.call_builder(&call);
/// let MyContract::doStuffReturn { c: _, d: _ } = builder.call().await?;
/// # Ok(())
/// # }
/// ```
///
/// Using [`ContractInstance`](crate::ContractInstance):
///
/// ```no_run
/// # async fn test<P: alloy_contract::private::Provider>(provider: P, dynamic_abi: alloy_json_abi::JsonAbi) -> Result<(), Box<dyn std::error::Error>> {
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
/// let contract: ContractInstance<_> = interface.connect(address, &provider);
///
/// // Build and call the function:
/// let call_builder: DynCallBuilder<_> = contract.function("doStuff", &[U256::ZERO.into(), true.into()])?;
/// let result: Vec<DynSolValue> = call_builder.call().await?;
///
/// // You can also decode the output manually. Get the raw bytes:
/// let raw_result: Bytes = call_builder.call_raw().await?;
/// // Or, equivalently:
/// let raw_builder: RawCallBuilder<_> = call_builder.clone().clear_decoder();
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
pub struct CallBuilder<P, D> {
    // TODO: this will not work with `send_transaction` and does not differentiate between EIP-1559
    // and legacy tx
    request: CallRequest,
    block: Option<BlockId>,
    state: Option<StateOverride>,
    provider: P,
    decoder: D,
}

// See [`ContractInstance`].
impl<P: TempProvider> DynCallBuilder<P> {
    pub(crate) fn new_dyn(provider: P, function: &Function, args: &[DynSolValue]) -> Result<Self> {
        Ok(Self::new_inner(provider, function.abi_encode_input(args)?.into(), function.clone()))
    }

    /// Clears the decoder, returning a raw call builder.
    #[inline]
    pub fn clear_decoder(self) -> RawCallBuilder<P> {
        RawCallBuilder {
            request: self.request,
            block: self.block,
            state: self.state,
            provider: self.provider,
            decoder: (),
        }
    }
}

impl<P: TempProvider, C: SolCall> SolCallBuilder<P, C> {
    // `sol!` macro constructor, see `#[sol(rpc)]`. Not public API.
    // NOTE: please avoid changing this function due to its use in the `sol!` macro.
    #[doc(hidden)]
    pub fn new_sol(provider: P, address: &Address, call: &C) -> Self {
        Self::new_inner(provider, call.abi_encode().into(), PhantomData::<C>).from(*address)
    }

    /// Clears the decoder, returning a raw call builder.
    #[inline]
    pub fn clear_decoder(self) -> RawCallBuilder<P> {
        RawCallBuilder {
            request: self.request,
            block: self.block,
            state: self.state,
            provider: self.provider,
            decoder: (),
        }
    }
}

impl<P: TempProvider> RawCallBuilder<P> {
    /// Creates a new call builder with the provided provider and ABI encoded input.
    ///
    /// Will not decode the output of the call, meaning that [`call`](Self::call) will behave the
    /// same as [`call_raw`](Self::call_raw).
    #[inline]
    pub fn new_raw(provider: P, input: Bytes) -> Self {
        Self::new_inner(provider, input, ())
    }
}

impl<P: TempProvider, D: CallDecoder> CallBuilder<P, D> {
    fn new_inner(provider: P, input: Bytes, decoder: D) -> Self {
        let request = CallRequest { input: CallInput::new(input), ..Default::default() };
        Self { request, decoder, provider, block: None, state: None }
    }

    /// Sets the `from` field in the transaction to the provided value
    pub fn from(mut self, from: Address) -> Self {
        self.request = self.request.from(from);
        self
    }

    /// Uses a Legacy transaction instead of an EIP-1559 one to execute the call
    pub fn legacy(self) -> Self {
        todo!()
    }

    /// Sets the `gas` field in the transaction to the provided value
    pub fn gas(mut self, gas: U256) -> Self {
        self.request = self.request.gas(gas);
        self
    }

    /// Sets the `gas_price` field in the transaction to the provided value
    /// If the internal transaction is an EIP-1559 one, then it sets both
    /// `max_fee_per_gas` and `max_priority_fee_per_gas` to the same value
    pub fn gas_price(mut self, gas_price: U256) -> Self {
        self.request = self.request.gas_price(gas_price);
        self
    }

    /// Sets the `value` field in the transaction to the provided value
    pub fn value(mut self, value: U256) -> Self {
        self.request = self.request.value(value);
        self
    }

    /// Sets the `nonce` field in the transaction to the provided value
    pub fn nonce(mut self, nonce: U64) -> Self {
        self.request = self.request.nonce(nonce);
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
        self.request.input.input().expect("set in the constructor")
    }

    /// Returns the estimated gas cost for the underlying transaction to be executed
    pub async fn estimate_gas(&self) -> Result<U256> {
        self.provider.estimate_gas(self.request.clone(), self.block).await.map_err(Into::into)
    }

    /// Queries the blockchain via an `eth_call` without submitting a transaction to the network.
    ///
    /// Returns the decoded the output by using the provided decoder.
    /// If this is not desired, use [`call_raw`](Self::call_raw) to get the raw output data.
    pub async fn call(&self) -> Result<D::Output> {
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
            self.provider.call_with_overrides(self.request.clone(), self.block, state.clone()).await
        } else {
            self.provider.call(self.request.clone(), self.block).await
        }
        .map_err(Into::into)
    }

    /// Decodes the output of a contract function using the provided decoder.
    #[inline]
    pub fn decode_output(&self, data: Bytes, validate: bool) -> Result<D::Output> {
        self.decoder.abi_decode_output(data, validate)
    }

    /// Signs and broadcasts this call as a transaction.
    pub async fn send(&self) -> Result<()> {
        todo!()
    }
}

impl<P: Clone, D> CallBuilder<&P, D> {
    /// Clones the provider and returns a new builder with the cloned provider.
    pub fn with_cloned_provider(self) -> CallBuilder<P, D> {
        CallBuilder {
            request: self.request,
            block: self.block,
            state: self.state,
            provider: self.provider.clone(),
            decoder: self.decoder,
        }
    }
}

/// [`CallBuilder`] can be turned into a [`Future`] automatically with `.await`.
///
/// Defaults to calling [`CallBuilder::call`].
impl<P, D> IntoFuture for CallBuilder<P, D>
where
    P: TempProvider,
    D: CallDecoder + Send + Sync,
    Self: 'static,
{
    type Output = Result<D::Output>;
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

impl<P, D: CallDecoder> std::fmt::Debug for CallBuilder<P, D> {
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
