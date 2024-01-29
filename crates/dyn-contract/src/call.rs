use crate::Result;
use alloy_dyn_abi::{DynSolValue, FunctionExt};
use alloy_json_abi::Function;
use alloy_primitives::{Address, Bytes, U256, U64};
use alloy_providers::provider::TempProvider;
use alloy_rpc_types::{state::StateOverride, BlockId, CallInput, CallRequest};
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

/// A builder for sending a transaction via. `eth_sendTransaction`, or calling a function via
/// `eth_call`.
///
/// The builder can be `.await`ed directly which is equivalent to invoking [`CallBuilder::call`].
///
/// # Note
///
/// Sets the [state overrides](https://geth.ethereum.org/docs/rpc/ns-eth#3-object---state-override-set) for `eth_call`, but this is not supported by all clients.
#[derive(Clone)]
pub struct CallBuilder<P> {
    // todo: this will not work with `send_transaction` and does not differentiate between EIP-1559
    // and legacy tx
    request: CallRequest,
    block: Option<BlockId>,
    state: Option<StateOverride>,
    provider: P,
    // todo: only used to decode - should it be some type D to dedupe with `sol!` contracts?
    function: Function,
}

impl<P> CallBuilder<P> {
    pub(crate) fn new(provider: P, function: Function, input: Bytes) -> Self {
        let request = CallRequest { input: CallInput::new(input), ..Default::default() };
        Self { request, function, provider, block: None, state: None }
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

    /// Returns the underlying transaction's ABI encoded data
    pub fn calldata(&self) -> Option<&Bytes> {
        self.request.input.input()
    }
}

impl<P> CallBuilder<P>
where
    P: TempProvider,
{
    /// Returns the estimated gas cost for the underlying transaction to be executed
    pub async fn estimate_gas(&self) -> Result<U256> {
        self.provider.estimate_gas(self.request.clone(), self.block).await.map_err(Into::into)
    }

    /// Queries the blockchain via an `eth_call` for the provided transaction.
    ///
    /// If executed on a non-state mutating smart contract function (i.e. `view`, `pure`)
    /// then it will return the raw data from the chain.
    ///
    /// If executed on a mutating smart contract function, it will do a "dry run" of the call
    /// and return the return type of the transaction without mutating the state.
    ///
    /// # Note
    ///
    /// This function _does not_ send a transaction from your account.
    pub async fn call(&self) -> Result<Vec<DynSolValue>> {
        let bytes = self.call_raw().await?;

        // decode output
        let data = self.function.abi_decode_output(&bytes, true)?;

        Ok(data)
    }

    /// Queries the blockchain via an `eth_call` for the provided transaction without decoding
    /// the output.
    pub async fn call_raw(&self) -> Result<Bytes> {
        if let Some(state) = &self.state {
            self.provider.call_with_overrides(self.request.clone(), self.block, state.clone()).await
        } else {
            self.provider.call(self.request.clone(), self.block).await
        }
        .map_err(Into::into)
    }

    /// Signs and broadcasts the provided transaction
    pub async fn send(&self) -> Result<()> {
        todo!()
    }
}

/// [`CallBuilder`] can be turned into a [`Future`] automatically with `.await`.
///
/// Defaults to calling [`CallBuilder::call`].
impl<P> IntoFuture for CallBuilder<P>
where
    P: TempProvider + 'static,
{
    type Output = Result<Vec<DynSolValue>>;

    #[cfg(target_arch = "wasm32")]
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    #[cfg(not(target_arch = "wasm32"))]
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        #[allow(clippy::redundant_async_block)]
        Box::pin(async move { self.call().await })
    }
}

impl<P> std::fmt::Debug for CallBuilder<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallBuilder")
            .field("function", &self.function)
            .field("block", &self.block)
            .field("state", &self.state)
            .finish()
    }
}
