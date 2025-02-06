use std::fmt::Debug;

use super::bindings::IMulticall3::{Call, Call3, Call3Value};
use crate::{Error, MulticallError, Result};
use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;

/// A trait for converting CallInfo into relevant call types.
pub(super) trait CallInfoTrait: std::fmt::Debug {
    /// Converts the [`CallInfo`] into a [`Call`] struct for `aggregateCall`
    fn to_call(&self) -> Call;
    /// Converts the [`CallInfo`] into a [`Call3`] struct for `aggregate3Call`
    fn to_call3(&self) -> Call3;
    /// Converts the [`CallInfo`] into a [`Call3Value`] struct for `aggregate3Call`
    fn to_call3_value(&self) -> Call3Value;
}

impl<C: SolCall> CallInfoTrait for CallInfo<C> {
    fn to_call(&self) -> Call {
        Call { target: self.target, callData: self.call.abi_encode().into() }
    }

    fn to_call3(&self) -> Call3 {
        Call3 {
            target: self.target,
            allowFailure: self.allow_failure,
            callData: self.call.abi_encode().into(),
        }
    }

    fn to_call3_value(&self) -> Call3Value {
        Call3Value {
            target: self.target,
            allowFailure: self.allow_failure,
            callData: self.call.abi_encode().into(),
            value: self.value.unwrap_or_default(),
        }
    }
}

/// A call that should be mapped into the relevant aggregate, aggregate3, aggregate3Value input
/// structs.
#[derive(Clone, Default)]
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

impl<C: SolCall> Debug for CallInfo<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallInfo")
            .field("target", &self.target)
            .field("allow_failure", &self.allow_failure)
            .field("value", &self.value)
            .finish()
    }
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
