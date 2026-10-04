use std::{fmt::Debug, marker::PhantomData};

use super::{
    bindings::IMulticall3::{aggregate3Call, Call, Call3, Call3Value, Result as MulticallResult},
    CallDecoder, CallTuple,
};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;
use thiserror::Error;

/// Result type for multicall operations.
pub type Result<T, E = MulticallError> = core::result::Result<T, E>;

/// A struct representing a failure in a multicall
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Call failed at index {idx} with return data: {return_data:?}")]
pub struct Failure {
    /// The index-position of the call that failed
    pub idx: usize,
    /// The return data of the call that failed
    pub return_data: Bytes,
}

/// A trait that is to be implemented by a type that can be distilled to a singular contract call
/// item.
pub trait MulticallItem {
    /// Decoder for the return data of the call.
    type Decoder: SolCall;

    /// Returns the value to send with the call.
    fn value(&self) -> U256;

    /// The target address of the call.
    fn target(&self) -> Address;
    /// ABI-encoded input data for the call.
    fn input(&self) -> Bytes;

    /// Converts `self` to a [`CallItem`] while specifying whether it can fail.
    fn into_call(self, allow_failure: bool) -> CallItem<Self::Decoder>
    where
        Self: Sized,
    {
        CallItem::<Self::Decoder>::from(self).allow_failure(allow_failure)
    }
}

/// Helper type to build a [`CallItem`]
#[derive(Debug)]
pub struct CallItemBuilder;

impl CallItemBuilder {
    /// Create a new [`CallItem`] instance.
    #[expect(clippy::new_ret_no_self)]
    pub fn new<Item: MulticallItem>(item: Item) -> CallItem<Item::Decoder> {
        CallItem::new(item.target(), item.input())
    }
}

/// A singular call type that is mapped into aggregate, aggregate3, aggregate3Value call structs via
/// the [`CallInfoTrait`] trait.
#[derive(Clone)]
pub struct CallItem<D: SolCall> {
    target: Address,
    input: Bytes,
    allow_failure: bool,
    value: U256,
    decoder: PhantomData<D>,
}

impl<D: SolCall> Debug for CallItem<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallItem")
            .field("target", &self.target)
            .field("allow_failure", &self.allow_failure)
            .field("value", &self.value)
            .field("input", &self.input)
            .finish()
    }
}

impl<D: SolCall> CallItem<D> {
    /// Create a new [`CallItem`] instance.
    pub const fn new(target: Address, input: Bytes) -> Self {
        Self { target, input, allow_failure: false, value: U256::ZERO, decoder: PhantomData }
    }

    /// Set whether the call should be allowed to fail or not.
    pub const fn allow_failure(mut self, allow_failure: bool) -> Self {
        self.allow_failure = allow_failure;
        self
    }

    /// Convenience function for `allow_failure(true)`
    pub const fn with_failure_allowed(self) -> Self {
        self.allow_failure(true)
    }

    /// Set the value to send with the call.
    pub const fn value(mut self, value: U256) -> Self {
        self.value = value;
        self
    }
}
impl<D: SolCall> CallInfoTrait for CallItem<D> {
    fn to_call(&self) -> Call {
        Call { target: self.target, callData: self.input.clone() }
    }

    fn to_call3(&self) -> Call3 {
        Call3 {
            target: self.target,
            allowFailure: self.allow_failure,
            callData: self.input.clone(),
        }
    }

    fn to_call3_value(&self) -> Call3Value {
        Call3Value {
            target: self.target,
            allowFailure: self.allow_failure,
            callData: self.input.clone(),
            value: self.value,
        }
    }
}
/// A trait for converting CallItem into relevant call types.
pub trait CallInfoTrait: std::fmt::Debug {
    /// Converts the [`CallItem`] into a [`Call`] struct for `aggregateCall`
    fn to_call(&self) -> Call;
    /// Converts the [`CallItem`] into a [`Call3`] struct for `aggregate3Call`
    fn to_call3(&self) -> Call3;
    /// Converts the [`CallItem`] into a [`Call3Value`] struct for `aggregate3Call`
    fn to_call3_value(&self) -> Call3Value;
}

impl<T, D> From<T> for CallItem<D>
where
    T: MulticallItem,
    D: SolCall,
{
    /// Converts a [`MulticallItem`] into a [`CallItem`]
    ///
    /// By default, it doesn't allow for failure when used in
    /// [`aggregate3`][crate::MulticallBuilder::aggregate3].
    /// Call [`allow_failure`][CallItem::allow_failure] on the result to specify the failure
    /// behavior, or use [`into_call`][MulticallItem::into_call] instead.
    fn from(value: T) -> Self {
        Self::new(value.target(), value.input()).value(value.value())
    }
}

/// Marker for dynamic calls: the entry type is fixed, so the multicall returns a `Vec` instead of a
/// tuple.
///
/// `D` may be a plain [`SolCall`] (`Vec` of its return value) or a [`Nested`] multicall (`Vec` of
/// the inner results).
#[derive(Debug)]
pub struct Dynamic<D: CallDecoder>(PhantomData<fn(D) -> D>);

impl<D: CallDecoder> CallTuple for Dynamic<D> {
    type Returns = Vec<D::ResultReturn>;
    type SuccessReturns = Vec<D::SuccessReturn>;

    fn decode_returns(data: &[Bytes]) -> Result<Self::SuccessReturns> {
        data.iter().map(|d| D::abi_decode_success(d)).collect()
    }

    fn decode_return_results(results: &[MulticallResult]) -> Result<Self::Returns> {
        Ok(results.iter().enumerate().map(|(idx, res)| D::abi_decode_result(idx, res)).collect())
    }

    fn try_into_success(results: Self::Returns) -> Result<Self::SuccessReturns> {
        results.into_iter().map(D::try_into_success).collect()
    }
}

/// Marker for a multicall nested inside another multicall.
///
/// `Inner` is the [`CallTuple`] of the inner [`MulticallBuilder`](crate::MulticallBuilder). Its
/// calls are encoded as one `aggregate3` call, so a whole batch decodes as a single outer entry:
///
/// - Inside a tuple multicall, it becomes one tuple element (e.g. `(Vec<A>, Vec<B>)`).
/// - Inside a dynamic multicall (`Dynamic<Nested<Inner>>`), the result is
///   `Vec<Inner::SuccessReturns>` (e.g. `Vec<(A, B)>`).
///
/// Nested payable calls are not supported; entries are always encoded with zero value.
///
/// ## Failure indices
///
/// On the fallible path (`aggregate3`/`try_aggregate`) an entry decodes to
/// `Result<Inner::Returns, Failure>`: the outer `Err(Failure)` uses the **outer** index (the whole
/// nested call failed), while any [`Failure`] inside `Inner::Returns` uses the **inner** index.
#[derive(Debug)]
pub struct Nested<Inner: CallTuple>(PhantomData<fn(Inner) -> Inner>);

impl<Inner: CallTuple> CallDecoder for Nested<Inner> {
    type SuccessReturn = Inner::SuccessReturns;
    type ResultReturn = core::result::Result<Inner::Returns, Failure>;

    fn abi_decode_success(data: &Bytes) -> Result<Self::SuccessReturn> {
        let results =
            aggregate3Call::abi_decode_returns(data).map_err(MulticallError::DecodeError)?;
        // Success path: reject any failed inner call, then decode via the inner success path so a
        // malformed return still surfaces as `DecodeError`.
        let mut return_data = Vec::with_capacity(results.len());
        for result in results {
            if !result.success {
                return Err(MulticallError::CallFailed(result.returnData));
            }
            return_data.push(result.returnData);
        }
        Inner::decode_returns(&return_data)
    }

    fn abi_decode_result(idx: usize, result: &MulticallResult) -> Self::ResultReturn {
        if !result.success {
            return Err(Failure { idx, return_data: result.returnData.clone() });
        }
        let results = aggregate3Call::abi_decode_returns(&result.returnData)
            .map_err(|_| Failure { idx, return_data: result.returnData.clone() })?;
        Inner::decode_return_results(&results)
            .map_err(|_| Failure { idx, return_data: result.returnData.clone() })
    }

    fn try_into_success(result: Self::ResultReturn) -> Result<Self::SuccessReturn> {
        let returns = result.map_err(|f| MulticallError::CallFailed(f.return_data))?;
        Inner::try_into_success(returns)
    }
}

/// Multicall errors.
#[derive(Debug, Error)]
pub enum MulticallError {
    /// Encountered when an `aggregate/aggregate3` batch contains a transaction with a value.
    #[error("batch contains a tx with a value, try using .send() instead")]
    ValueTx,
    /// Error decoding return data.
    #[error("could not decode: {0}")]
    DecodeError(alloy_sol_types::Error),
    /// No return data was found.
    #[error("no return data")]
    NoReturnData,
    /// Call failed.
    #[error("call failed when success was assured, this occurs when try_into_success is called on a failed call")]
    CallFailed(Bytes),
    /// Encountered when a transport error occurs while calling a multicall batch.
    #[error("Transport error: {0}")]
    TransportError(#[from] alloy_transport::TransportError),
}
