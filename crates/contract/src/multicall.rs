use std::marker::PhantomData;

/// The canon deployed address and chains
pub mod constants;
mod error;

pub use aggregate::{Aggregate, AggregateRef, OwnedAggregate};
pub use aggregate3::{Aggregate3, Aggregate3Ref, OwnedAggregate3};
pub use try_aggregate::{OwnedTryAggregate, TryAggregate, TryAggregateRef};

pub use constants::{MULTICALL_ADDRESS, MULTICALL_SUPPORTED_CHAINS};

#[doc(inline)]
pub use error::MultiCallError;

use std::sync::Arc;

use alloy_json_abi::Function;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_sol_types::sol;
use alloy_transport::Transport;

use crate::{CallBuilder, CallDecoder};

sol! {
    #![sol(alloy_contract = crate)]
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    /// Module containing types and functions of the Multicall3 contract.
    interface IMulticall3 {
        struct Call {
            address target;
            bytes callData;
        }

        struct Call3 {
            address target;
            bool allowFailure;
            bytes callData;
        }

        struct Call3Value {
            address target;
            bool allowFailure;
            uint256 value;
            bytes callData;
        }

        struct Result {
            bool success;
            bytes returnData;
        }

        /// Aggregates multiple calls into a single call.
        function aggregate(Call[] calldata calls)
            external
            payable
            returns (uint256 blockNumber, bytes[] memory returnData);

        /// Aggregates multiple calls into a single call allowing some to fail
        function aggregate3(Call3[] calldata calls) external payable returns (Result[] memory returnData);

        /// Aggregates multiple calls into a single call allowing some to fail
        function tryAggregate(bool requireSuccess, Call[] calldata calls)
            external
            payable
            returns (Result[] memory returnData);
  }
}

/// The Multicall struct either works with a single type or every return type is dynamic.
///
/// Multicall is easier to name via the [`SolMultiCall`] or [`DynMultiCall`] type aliases.
#[derive(Debug)]
pub struct MultiCall<T, P, N: Network> {
    instance: Arc<IMulticall3::IMulticall3Instance<T, P, N>>,
}

impl<T, P, N> MultiCall<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    /// Create a new multicall instance.
    ///
    /// # Errors
    /// - If the chain_id is not in the list of supported chains.
    pub async fn new(provider: P, address: Option<Address>) -> Result<Self, MultiCallError> {
        let instance = Arc::new(IMulticall3::IMulticall3Instance::new(
            {
                match address {
                    Some(address) => address,
                    None => {
                        if !MULTICALL_SUPPORTED_CHAINS.contains(&provider.get_chain_id().await?) {
                            MULTICALL_ADDRESS
                        } else {
                            return Err(error::MultiCallError::MissingTargetAddress);
                        }
                    }
                }
            },
            provider,
        ));

        Ok(Self { instance })
    }

    /// A builder for the aggregate call.
    pub fn aggregate<'a, D: CallDecoder>(&'a self) -> AggregateRef<'a, T, P, D, N> {
        AggregateRef { r#ref: &self.instance, calls: vec![], batch: None }
    }

    /// A builder for the try_aggreate call.
    pub fn try_aggregate<'a, D: CallDecoder>(&'a self) -> TryAggregateRef<'a, T, P, D, N> {
        TryAggregateRef { r#ref: &self.instance, calls: vec![], batch: None }
    }

    /// A builder for the aggregate3 call.
    pub fn aggregate3<'a, D: CallDecoder>(&'a self) -> Aggregate3Ref<'a, T, P, D, N> {
        Aggregate3Ref { r#ref: &self.instance, calls: vec![], batch: None }
    }
}

/// A dyn aggregate call.
pub type DynAggreagate<T, P, N> = OwnedAggregate<T, P, Function, N>;

/// A static aggregate call.
pub type SolAggreagate<T, P, C, N> = OwnedAggregate<T, P, PhantomData<C>, N>;

/// A dyn try aggregate call.
pub type DynTryAggreagate<T, P, N> = OwnedTryAggregate<T, P, Function, N>;

/// A static try aggregate call.
pub type SolTryAggreagate<T, P, C, N> = OwnedTryAggregate<T, P, PhantomData<C>, N>;

/// A dyn aggregate3 call.
pub type DynAggreagate3<T, P, N> = OwnedAggregate3<T, P, Function, N>;

/// A static aggregate3 call.
pub type SolAggreagate3<T, P, C, N> = OwnedAggregate3<T, P, PhantomData<C>, N>;

mod aggregate {
    use std::fmt::Debug;

    use super::into_calls::*;
    use super::*;
    
    /// An aggreagte call that owns the refrence to the underlying instance
    pub type OwnedAggregate<T, P, D, N> =
        Aggregate<Arc<IMulticall3::IMulticall3Instance<T, P, N>>, T, P, D, N>;

    /// An aggreagte call that doesnt own the refrence to the underlying instance
    pub type AggregateRef<'a, T, P, D, N> =
        Aggregate<&'a Arc<IMulticall3::IMulticall3Instance<T, P, N>>, T, P, D, N>;

    /// Represents a call to the aggregate method.
    ///
    /// [`Aggregate`] multicalls will also fail fast.
    pub struct Aggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        pub(super) r#ref: R,
        pub(super) calls: Vec<CallBuilder<T, P, D, N>>,
        pub(super) batch: Option<usize>,
    }

    impl<R, T, P, D, N> Aggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will fail fast on any reverts
        pub async fn call(&self) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let (decoders, requests) = self.parts_ref();

            self.aggregate_inner(
                &decoders,
                requests
                    .into_iter()
                    .map(|call| call_from_tx_ref::<N>(call))
                    .collect::<Result<Vec<_>, MultiCallError>>()?,
            )
            .await
        }

        /// like [Self::call] but will consume the calldata
        pub async fn call_take(&mut self) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let (decoders, requests) = self.parts();

            self.aggregate_inner(
                decoders.iter().collect::<Vec<_>>().as_slice(),
                requests
                    .into_iter()
                    .map(|call| call_from_tx::<N>(call))
                    .collect::<Result<Vec<_>, MultiCallError>>()?,
            )
            .await
        }

        pub fn set_batch(&mut self, batch: Option<usize>) {
            self.batch = batch;
        }

        /// Clear the calls
        pub fn clear_calls(&mut self) {
            self.calls.clear();
        }

        /// Add a call to the multicall
        pub fn add_call(&mut self, call: CallBuilder<T, P, D, N>) {
            self.calls.push(call);
        }

        /// Add multiple calls to the multicall
        pub fn add_calls<I>(&mut self, calls: I)
        where
            I: Iterator<Item = CallBuilder<T, P, D, N>>,
        {
            self.calls.extend(calls);
        }
    }

    impl<R, T, P, D, N> Aggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will revert on the first failure regardless of what
        /// you set
        async fn aggregate_inner(
            &self,
            decoders: &[&D],
            requests: Vec<IMulticall3::Call>,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let mut results;
            let instance = self.r#ref.as_ref();

            if let Some(batch) = self.batch {
                results = Vec::with_capacity(requests.len());

                for chunk in requests.chunks(batch) {
                    let chunk_results = instance.aggregate(chunk.to_vec()).call().await?;

                    results.extend(chunk_results.returnData);
                }
            } else {
                results = instance.aggregate(requests).call().await?.returnData;
            }

            results
                .into_iter()
                .zip(decoders.into_iter())
                .map(|(out, decoder)| decoder.abi_decode_output(out, true))
                .map(|r| r.map_err(Into::into))
                .collect()
        }

        fn parts(&mut self) -> (Vec<D>, Vec<N::TransactionRequest>) {
            std::mem::take(&mut self.calls)
                .into_iter()
                .map(|call| {
                    let (decoder, req) = call.take_decoder();

                    (decoder, req.into_transaction_request())
                })
                .unzip()
        }

        fn parts_ref(&self) -> (Vec<&D>, Vec<&N::TransactionRequest>) {
            self.calls.iter().map(|call| (call.decoder(), call.as_ref())).unzip()
        }
    }

    impl<R, T, P, D, N> Debug for Aggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Aggregate")
                .field("r#ref type", &std::any::type_name::<R>())
                .field("calls", &self.calls)
                .field("batch", &self.batch)
                .finish()
        }
    }

    impl<'a, T, P, D, N> From<AggregateRef<'a, T, P, D, N>> for OwnedAggregate<T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
    {
        fn from(aggregate: AggregateRef<'a, T, P, D, N>) -> Self {
            Self { r#ref: aggregate.r#ref.clone(), calls: aggregate.calls, batch: aggregate.batch }
        }
    }
}

mod try_aggregate {
    use super::into_calls::*;
    use super::*;
    use std::fmt::Debug;

    /// An aggreagte call that owns the refrence to the underlying instance
    pub type OwnedTryAggregate<T, P, D, N> =
        TryAggregate<Arc<IMulticall3::IMulticall3Instance<T, P, N>>, T, P, D, N>;

    /// An aggreagte call that doesnt own the refrence to the underlying instance
    pub type TryAggregateRef<'a, T, P, D, N> =
        TryAggregate<&'a Arc<IMulticall3::IMulticall3Instance<T, P, N>>, T, P, D, N>;

    /// Represents a call to the aggregate method.
    ///
    /// [`Aggregate`] multicalls will also fail fast.
    pub struct TryAggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        pub(super) r#ref: R,
        pub(super) calls: Vec<CallBuilder<T, P, D, N>>,
        pub(super) batch: Option<usize>,
    }

    impl<R, T, P, D, N> Debug for TryAggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TryAggregate")
                .field("r#ref type", &std::any::type_name::<R>())
                .field("calls", &self.calls)
                .field("batch", &self.batch)
                .finish()
        }
    }

    impl<R, T, P, D, N> TryAggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, filtering out any failed calls.
        pub async fn call(&self, require_success: bool) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let (decoders, requests) = self.parts_ref();

            self.try_aggregate_inner(
                require_success,
                &decoders,
                requests
                    .into_iter()
                    .map(|call| call_from_tx_ref::<N>(call))
                    .collect::<Result<Vec<_>, MultiCallError>>()?,
            )
            .await
        }

        /// Call the aggregate method, this method will revert on the first failure regardless of what
        pub async fn call_take(
            &mut self,
            require_success: bool,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let (decoders, requests) = self.parts();

            self.try_aggregate_inner(
                require_success,
                decoders.iter().collect::<Vec<_>>().as_slice(),
                requests
                    .into_iter()
                    .map(|call| call_from_tx::<N>(call))
                    .collect::<Result<Vec<_>, MultiCallError>>()?,
            )
            .await
        }

        /// Clear the calls
        pub fn clear_calls(&mut self) {
            self.calls.clear();
        }

        pub fn set_batch(&mut self, batch: Option<usize>) {
            self.batch = batch;
        }

        /// Add a call to the multicall
        pub fn add_call(&mut self, call: CallBuilder<T, P, D, N>) {
            self.calls.push(call);
        }

        /// Add multiple calls to the multicall
        pub fn add_calls<I>(&mut self, calls: I)
        where
            I: Iterator<Item = CallBuilder<T, P, D, N>>,
        {
            self.calls.extend(calls);
        }
    }

    impl<R, T, P, D, N> TryAggregate<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will revert on the first failure regardless of what
        /// you set
        async fn try_aggregate_inner(
            &self,
            require_success: bool,
            decoders: &[&D],
            requests: Vec<IMulticall3::Call>,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let mut results;
            let instance = self.r#ref.as_ref();

            if let Some(batch) = self.batch {
                results = Vec::with_capacity(requests.len());

                for chunk in requests.chunks(batch) {
                    let chunk_results =
                        instance.tryAggregate(require_success, chunk.to_vec()).call().await?;

                    results.extend(chunk_results.returnData);
                }
            } else {
                results = instance.tryAggregate(require_success, requests).call().await?.returnData;
            }

            results
                .into_iter()
                .zip(decoders.into_iter())
                .filter_map(|(out, decoder)| {
                    if out.success {
                        Some(decoder.abi_decode_output(out.returnData, true))
                    } else {
                        None
                    }
                })
                .map(|r| r.map_err(Into::into))
                .collect()
        }

        fn parts(&mut self) -> (Vec<D>, Vec<N::TransactionRequest>) {
            std::mem::take(&mut self.calls)
                .into_iter()
                .map(|call| {
                    let (decoder, req) = call.take_decoder();

                    (decoder, req.into_transaction_request())
                })
                .unzip()
        }

        fn parts_ref(&self) -> (Vec<&D>, Vec<&N::TransactionRequest>) {
            self.calls.iter().map(|call| (call.decoder(), call.as_ref())).unzip()
        }
    }
}

mod aggregate3 {
    use super::into_calls::*;
    use super::*;
    use std::fmt::Debug;

    /// An aggreagte call that owns the refrence to the underlying instance
    pub type OwnedAggregate3<T, P, D, N> =
        Aggregate3<Arc<IMulticall3::IMulticall3Instance<T, P, N>>, T, P, D, N>;

    /// An aggreagte call that doesnt own the refrence to the underlying instance
    pub type Aggregate3Ref<'a, T, P, D, N> =
        Aggregate3<&'a Arc<IMulticall3::IMulticall3Instance<T, P, N>>, T, P, D, N>;

    /// Represents a call to the aggregate method.
    ///
    /// [`Aggregate`] multicalls will also fail fast.
    pub struct Aggregate3<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        pub(super) r#ref: R,
        pub(super) calls: Vec<(bool, CallBuilder<T, P, D, N>)>,
        pub(super) batch: Option<usize>,
    }

    impl<R, T, P, D, N> Debug for Aggregate3<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TryAggregate")
                .field("r#ref type", &std::any::type_name::<R>())
                .field("calls", &self.calls)
                .field("batch", &self.batch)
                .finish()
        }
    }

    impl<R, T, P, D, N> Aggregate3<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate3 method, filtering out any failed calls.
        pub async fn call(&self) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let (decoders, requests) = self.parts_ref();

            self.aggregate3_inner(
                &decoders,
                requests
                    .into_iter()
                    .map(|(allow_failure, call)| call3_from_tx_ref::<N>(call, allow_failure))
                    .collect::<Result<Vec<_>, MultiCallError>>()?,
            )
            .await
        }

        /// like [Self::call] but will consume the calldata
        pub async fn call_take(&mut self) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let (decoders, requests) = self.parts();

            self.aggregate3_inner(
                decoders.iter().collect::<Vec<_>>().as_slice(),
                requests
                    .into_iter()
                    .map(|(allow_failure, call)| call3_from_tx::<N>(call, allow_failure))
                    .collect::<Result<Vec<_>, MultiCallError>>()?,
            )
            .await
        }

        /// Clear the calls
        pub fn clear_calls(&mut self) {
            self.calls.clear();
        }

        pub fn set_batch(&mut self, batch: Option<usize>) {
            self.batch = batch;
        }

        /// Add a call to the multicall
        pub fn add_call(&mut self, allow_failure: bool, call: CallBuilder<T, P, D, N>) {
            self.calls.push((allow_failure, call));
        }

        /// Add multiple calls to the multicall
        pub fn add_calls<I>(&mut self, calls: I)
        where
            I: Iterator<Item = (bool, CallBuilder<T, P, D, N>)>,
        {
            self.calls.extend(calls);
        }
    }

    impl<R, T, P, D, N> Aggregate3<R, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        R: AsRef<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will revert on the first failure regardless of what
        /// you set
        async fn aggregate3_inner(
            &self,
            decoders: &[&D],
            requests: Vec<IMulticall3::Call3>,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let mut results;
            let instance = self.r#ref.as_ref();

            if let Some(batch) = self.batch {
                results = Vec::with_capacity(requests.len());

                for chunk in requests.chunks(batch) {
                    let chunk_results = instance.aggregate3(chunk.to_vec()).call().await?;

                    results.extend(chunk_results.returnData);
                }
            } else {
                results = instance.aggregate3(requests).call().await?.returnData;
            }

            results
                .into_iter()
                .zip(decoders.into_iter())
                .filter_map(|(r, d)| {
                    if r.success {
                        Some(d.abi_decode_output(r.returnData, true))
                    } else {
                        None
                    }
                })
                .map(|r| r.map_err(Into::into))
                .collect()
        }

        fn parts(&mut self) -> (Vec<D>, Vec<(bool, N::TransactionRequest)>) {
            std::mem::take(&mut self.calls)
                .into_iter()
                .map(|(allow_failure, call)| {
                    let (decoder, req) = call.take_decoder();

                    (decoder, (allow_failure, req.into_transaction_request()))
                })
                .unzip()
        }

        fn parts_ref(&self) -> (Vec<&D>, Vec<(bool, &N::TransactionRequest)>) {
            self.calls
                .iter()
                .map(|(allow_failure, call)| (call.decoder(), (*allow_failure, call.as_ref())))
                .unzip()
        }
    }
}

mod into_calls {
    use super::{IMulticall3, MultiCallError, Network, TransactionBuilder};

    #[inline]
    pub(super) fn call3_from_tx<N>(
        tx: N::TransactionRequest,
        allow_failure: bool,
    ) -> Result<IMulticall3::Call3, MultiCallError>
    where
        N: Network,
    {
        Ok(IMulticall3::Call3 {
            target: tx.to().ok_or(MultiCallError::MissingTargetAddress)?,
            allowFailure: allow_failure,
            callData: tx.into_input().unwrap_or_default(),
        })
    }

    #[inline]
    pub(super) fn call3_from_tx_ref<N>(
        tx: &N::TransactionRequest,
        allow_failure: bool,
    ) -> Result<IMulticall3::Call3, MultiCallError>
    where
        N: Network,
    {
        Ok(IMulticall3::Call3 {
            target: tx.to().ok_or(MultiCallError::MissingTargetAddress)?,
            allowFailure: allow_failure,
            callData: tx.input().cloned().unwrap_or_default(),
        })
    }

    #[inline]
    pub(super) fn call_from_tx<N>(
        tx: N::TransactionRequest,
    ) -> Result<IMulticall3::Call, MultiCallError>
    where
        N: Network,
    {
        Ok(IMulticall3::Call {
            target: tx.to().ok_or(MultiCallError::MissingTargetAddress)?,
            callData: tx.into_input().unwrap_or_default(),
        })
    }

    #[inline]
    pub(super) fn call_from_tx_ref<N>(
        tx: &N::TransactionRequest,
    ) -> Result<IMulticall3::Call, MultiCallError>
    where
        N: Network,
    {
        Ok(IMulticall3::Call {
            target: tx.to().ok_or(MultiCallError::MissingTargetAddress)?,
            callData: tx.input().cloned().unwrap_or_default(),
        })
    }
}
