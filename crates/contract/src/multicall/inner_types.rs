use super::bindings::IMulticall3::Call3;
use crate::{Error, MulticallError, Result};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;

/// A call that should be mapped into the relevant aggregate, aggregate3, aggregate3Value input
/// structs.
#[derive(Debug, Clone, Default)]
pub struct CallInfo<C: SolCall> {
    /// The target address of the call.
    target: Address,
    /// Whether this call should be allowed to fail.
    allow_failure: bool,
    /// The value to send with the call, value calls (i.e sending transactions) only works with
    /// `aggregate3Value`.
    value: Option<U256>,
    /// The call implementing `SolCall`. Used to abi encode the inputs and decode the
    /// outputs.
    call: C,
}

impl<C: SolCall> CallInfo<C> {
    /// Create a new [`CallInfo`] instance.
    pub fn new(target: Address, call: C) -> Self {
        Self { target, call, allow_failure: false, value: None }
    }

    /// Set whether the call should be allowed to fail or not.
    ///
    /// By default, this is set to `false`.
    pub fn allow_failure(mut self, allow_failure: bool) -> Self {
        self.allow_failure = allow_failure;
        self
    }

    /// Set the value to send with the call.
    ///
    /// Setting this entails redirection of the multicall batch via `aggregate3Value` instead of the
    /// usual `aggregate3/aggregate`.
    ///
    /// Note:
    ///
    /// For the call to be successful, the `msg.value` should be _strictly_ equal to the sum of
    /// `value` of all calls in the batch, i.e `msg.value = ∑ call.value`.
    pub fn value(mut self, value: U256) -> Self {
        self.value = Some(value);
        self
    }

    /// ABI-decode the return data.
    pub fn decode(&self, data: &[u8]) -> Result<C::Return> {
        C::abi_decode_returns(data, true)
            .map_err(|e| Error::MulticallError(MulticallError::DecodeError(e)))
    }
}

/// Trait for decoding return values from a sequence of calls
pub trait DecodeReturns {
    /// Decoded Return Tuple
    type Returns;

    /// Decode the return values
    fn decode_returns(data: &[Bytes]) -> Result<Self::Returns>;
}

impl DecodeReturns for Identity {
    type Returns = ();
    fn decode_returns(_data: &[Bytes]) -> Result<Self::Returns> {
        Ok(())
    }
}

// Decode the stack recursively.
impl<L: SolCall, R: DecodeReturns> DecodeReturns for Stack<L, R> {
    type Returns = (R::Returns, L::Return); // Maintain call order.
    fn decode_returns(data: &[Bytes]) -> Result<Self::Returns> {
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

impl Iterator for Identity {
    type Item = Call3;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
