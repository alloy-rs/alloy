use std::marker::PhantomData;

/// The canon deployed address and chains
pub mod constants;
mod error;

pub use aggregate::Aggregate;
pub use aggregate3::Aggregate3;
pub use try_aggregate::TryAggregate;

pub use constants::{MULTICALL_ADDRESS, MULTICALL_SUPPORTED_CHAINS};

#[doc(inline)]
pub use error::MultiCallError;

use alloy_json_abi::Function;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_sol_types::sol;
use alloy_transport::Transport;
use IMulticall3::IMulticall3Instance;

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

/// The multicall instance, which is responsible for giving out builders for the various multicall
/// operations.
///
/// This type holds no calldata itself, instead you can use one of the aggreagte call types to store
/// the calls.
///
/// Multicall offers three modes of operation:
///     - [Self::aggregate] which will fail fast on any revert
///     - [Self::try_aggregate] which will filter out any failed calls
///     - [Self::aggregate3] which will allow you to specify which calls can fail
///
/// If you have a multicall with calldata that doesnt change, you can easily name these types using
/// one of the following aliases
///     - [DynAggreagate] or [SolAggreagate] for the aggregate call
///     - [DynTryAggreagate] or [SolTryAggreagate] for the try_aggregate call
///     - [DynAggreagate3] or [SolAggreagate3] for the aggregate3 call
#[derive(Debug, Clone)]
pub struct MultiCall<T, P, N: Network> {
    instance: IMulticall3::IMulticall3Instance<T, P, N>,
}

impl<T, P, N> MultiCall<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    /// Create a new multicall instance with a specific address.
    ///
    /// This method does not check the chain_id against the supported chains.
    pub const fn new(address: Address, provider: P) -> Self {
        Self { instance: IMulticall3::IMulticall3Instance::new(address, provider) }
    }

    /// Create a new multicall instance checking if the chain_id is in the list of supported chains.
    pub async fn new_checked(provider: P) -> Result<Self, MultiCallError> {
        if !MULTICALL_SUPPORTED_CHAINS
            .contains(&provider.get_chain_id().await.map_err(crate::Error::from)?)
        {
            Ok(Self::new(MULTICALL_ADDRESS, provider))
        } else {
            return Err(error::MultiCallError::MissingTargetAddress);
        }
    }

    /// A builder for the aggregate call.
    pub fn aggregate_owned<D: CallDecoder>(self) -> OwnedAggreagte<T, P, D, N> {
        Aggregate { instance: self.instance, calls: Vec::new(), batch: None }
    }

    /// A builder for the aggregate call.
    pub fn aggregate<D: CallDecoder>(&self) -> AggregateRef<'_, T, P, D, N> {
        Aggregate { instance: &self.instance, calls: Vec::new(), batch: None }
    }

    /// A builder for the try_aggreate call.
    pub fn try_aggregate_owned<D: CallDecoder>(self) -> OwnedTryAggregate<T, P, D, N> {
        TryAggregate { instance: self.instance, calls: Vec::new(), batch: None }
    }

    /// A builder for the try_aggreate call.
    pub fn try_aggregate<D: CallDecoder>(&self) -> TryAggregateRef<'_, T, P, D, N> {
        TryAggregate { instance: &self.instance, calls: Vec::new(), batch: None }
    }

    /// A builder for the aggregate3 call.
    pub fn aggregate3_owned<D: CallDecoder>(self) -> OwnedAggregate3<T, P, D, N> {
        Aggregate3 { instance: self.instance, calls: Vec::new(), batch: None }
    }

    /// A builder for the aggregate3 call.
    pub fn aggregate3<D: CallDecoder>(&self) -> Aggregate3Ref<'_, T, P, D, N> {
        Aggregate3 { instance: &self.instance, calls: Vec::new(), batch: None }
    }
}

pub use aggregate::{AggregateRef, OwnedAggreagte};
pub use aggregate3::{Aggregate3Ref, OwnedAggregate3};
pub use try_aggregate::{OwnedTryAggregate, TryAggregateRef};

/// A dyn aggregate call.
pub type DynAggreagate<T, P, N> = OwnedAggreagte<T, P, Function, N>;

/// A static aggregate call.
pub type SolAggreagate<T, P, C, N> = OwnedAggreagte<T, P, PhantomData<C>, N>;

/// A dyn try aggregate call.
pub type DynTryAggreagate<T, P, N> = OwnedTryAggregate<T, P, Function, N>;

/// A static try aggregate call.
pub type SolTryAggreagate<T, P, C, N> = OwnedTryAggregate<T, P, PhantomData<C>, N>;

/// A dyn aggregate3 call.
pub type DynAggreagate3<T, P, N> = OwnedAggregate3<T, P, Function, N>;

/// A static aggregate3 call.
pub type SolAggreagate3<T, P, C, N> = OwnedAggregate3<T, P, PhantomData<C>, N>;

mod aggregate {
    use std::{borrow::Borrow, fmt::Debug};

    use super::{into_calls::*, *};

    /// An aggreagte call that owns its underlying instance.
    pub type OwnedAggreagte<T, P, D, N> = Aggregate<IMulticall3Instance<T, P, N>, T, P, D, N>;

    /// An aggreagte call that borrows its underlying instance.
    pub type AggregateRef<'a, T, P, D, N> = Aggregate<&'a IMulticall3Instance<T, P, N>, T, P, D, N>;

    /// Represents a call to the aggregate method.
    ///
    /// [`Aggregate`] multicalls will also fail fast.
    pub struct Aggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        pub(super) instance: I,
        pub(super) calls: Vec<CallBuilder<T, P, D, N>>,
        pub(super) batch: Option<usize>,
    }

    impl<I, T, P, D, N> Extend<CallBuilder<T, P, D, N>> for Aggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn extend<It>(&mut self, iter: It)
        where
            It: IntoIterator<Item = CallBuilder<T, P, D, N>>,
        {
            self.add_calls(iter)
        }
    }

    impl<I, T, P, D, N> Aggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Set the batch size for this multicall
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
        pub fn add_calls<It>(&mut self, calls: It)
        where
            It: IntoIterator<Item = CallBuilder<T, P, D, N>>,
        {
            self.calls.extend(calls);
        }

        /// Reserve additional space for calls
        pub fn reserve(&mut self, additional: usize) {
            self.calls.reserve(additional);
        }
    }

    impl<I, T, P, D, N> Aggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
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

        /// like [Self::call] but will consumes the call builder
        pub async fn call_consume(mut self) -> Result<Vec<D::CallOutput>, MultiCallError> {
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
    }

    impl<I, T, P, D, N> Aggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will revert on the first failure regardless of
        /// what you set
        async fn aggregate_inner(
            &self,
            decoders: &[&D],
            requests: Vec<IMulticall3::Call>,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let mut results;

            if let Some(batch) = self.batch {
                results = Vec::with_capacity(requests.len());

                for chunk in requests.chunks(batch) {
                    let chunk_results =
                        self.instance.borrow().aggregate(chunk.to_vec()).call().await?;

                    results.extend(chunk_results.returnData);
                }
            } else {
                results = self.instance.borrow().aggregate(requests).call().await?.returnData;
            }

            results
                .into_iter()
                .zip(decoders)
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

    impl<I, T, P, D, N> Debug for Aggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Aggregate")
                .field("calls", &self.calls)
                .field("batch", &self.batch)
                .finish()
        }
    }
}

mod try_aggregate {
    use super::{into_calls::*, *};
    use std::{borrow::Borrow, fmt::Debug};

    /// A try aggregate call that owns its underlying instance.
    pub type OwnedTryAggregate<T, P, D, N> = TryAggregate<IMulticall3Instance<T, P, N>, T, P, D, N>;

    /// A try aggregate call that borrows its underlying instance.
    pub type TryAggregateRef<'a, T, P, D, N> =
        TryAggregate<&'a IMulticall3Instance<T, P, N>, T, P, D, N>;

    /// Represents a call to the aggregate method.
    ///
    /// [`TryAggregate`] multicalls will also fail fast.
    pub struct TryAggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        pub(super) instance: I,
        pub(super) calls: Vec<CallBuilder<T, P, D, N>>,
        pub(super) batch: Option<usize>,
    }

    impl<I, T, P, D, N> Extend<CallBuilder<T, P, D, N>> for TryAggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn extend<It>(&mut self, iter: It)
        where
            It: IntoIterator<Item = CallBuilder<T, P, D, N>>,
        {
            self.add_calls(iter)
        }
    }

    impl<I, T, P, D, N> Debug for TryAggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TryAggregate")
                .field("calls", &self.calls)
                .field("batch", &self.batch)
                .finish()
        }
    }

    impl<I, T, P, D, N> TryAggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Clear the calls
        pub fn clear_calls(&mut self) {
            self.calls.clear();
        }

        /// Set the batch size for this multicall
        pub fn set_batch(&mut self, batch: Option<usize>) {
            self.batch = batch;
        }

        /// Add a call to the multicall
        pub fn add_call(&mut self, call: CallBuilder<T, P, D, N>) {
            self.calls.push(call);
        }

        /// Add multiple calls to the multicall
        pub fn add_calls<It>(&mut self, calls: It)
        where
            It: IntoIterator<Item = CallBuilder<T, P, D, N>>,
        {
            self.calls.extend(calls);
        }

        /// Reserve additional space for calls
        pub fn reserve(&mut self, additional: usize) {
            self.calls.reserve(additional);
        }
    }

    impl<I, T, P, D, N> TryAggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, filtering out any failed calls if require_success is false
        pub async fn call(
            &self,
            require_success: bool,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
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

        /// Like [Self::call] but will consumes this call builder
        pub async fn call_consume(
            mut self,
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
    }

    impl<I, T, P, D, N> TryAggregate<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will revert on the first failure regardless of
        /// what you set
        async fn try_aggregate_inner(
            &self,
            require_success: bool,
            decoders: &[&D],
            requests: Vec<IMulticall3::Call>,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let mut results;

            if let Some(batch) = self.batch {
                results = Vec::with_capacity(requests.len());

                for chunk in requests.chunks(batch) {
                    let chunk_results = self
                        .instance
                        .borrow()
                        .tryAggregate(require_success, chunk.to_vec())
                        .call()
                        .await?;

                    results.extend(chunk_results.returnData);
                }
            } else {
                results = self
                    .instance
                    .borrow()
                    .tryAggregate(require_success, requests)
                    .call()
                    .await?
                    .returnData;
            }

            results
                .into_iter()
                .zip(decoders)
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
    use super::{into_calls::*, *};
    use std::{borrow::Borrow, fmt::Debug};

    /// A aggregate3 call that owns the underlying instance.
    pub type OwnedAggregate3<T, P, D, N> = Aggregate3<IMulticall3Instance<T, P, N>, T, P, D, N>;

    /// A aggregate3 call that borrows the underlying instance.
    pub type Aggregate3Ref<'a, T, P, D, N> =
        Aggregate3<&'a IMulticall3Instance<T, P, N>, T, P, D, N>;

    /// Represents a call to the aggregate method.
    ///
    /// [`Aggregate3`] multicalls will filter failed results
    pub struct Aggregate3<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        pub(super) instance: I,
        pub(super) calls: Vec<(bool, CallBuilder<T, P, D, N>)>,
        pub(super) batch: Option<usize>,
    }

    impl<I, T, P, D, N> Extend<(bool, CallBuilder<T, P, D, N>)> for Aggregate3<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn extend<It>(&mut self, iter: It)
        where
            It: IntoIterator<Item = (bool, CallBuilder<T, P, D, N>)>,
        {
            self.add_calls(iter)
        }
    }

    impl<I, T, P, D, N> Debug for Aggregate3<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TryAggregate")
                .field("calls", &self.calls)
                .field("batch", &self.batch)
                .finish()
        }
    }

    impl<I, T, P, D, N> Aggregate3<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Clear the calls
        pub fn clear_calls(&mut self) {
            self.calls.clear();
        }

        /// Set the batch size for this multicall
        pub fn set_batch(&mut self, batch: Option<usize>) {
            self.batch = batch;
        }

        /// Add a call to the multicall
        pub fn add_call(&mut self, allow_failure: bool, call: CallBuilder<T, P, D, N>) {
            self.calls.push((allow_failure, call));
        }

        /// Add multiple calls to the multicall
        pub fn add_calls<It>(&mut self, calls: It)
        where
            It: IntoIterator<Item = (bool, CallBuilder<T, P, D, N>)>,
        {
            self.calls.extend(calls);
        }

        /// Reserve additional space for calls
        pub fn reserve(&mut self, additional: usize) {
            self.calls.reserve(additional);
        }
    }

    impl<I, T, P, D, N> Aggregate3<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate3 method, this method will fail fast on any reverts
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

        /// Like [Self::call] but will consume the call builder
        pub async fn call_consume(mut self) -> Result<Vec<D::CallOutput>, MultiCallError> {
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
    }

    impl<I, T, P, D, N> Aggregate3<I, T, P, D, N>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        D: CallDecoder,
        N: Network,
        I: Borrow<IMulticall3::IMulticall3Instance<T, P, N>>,
    {
        /// Call the aggregate method, this method will revert on the first failure regardless of
        /// what you set
        async fn aggregate3_inner(
            &self,
            decoders: &[&D],
            requests: Vec<IMulticall3::Call3>,
        ) -> Result<Vec<D::CallOutput>, MultiCallError> {
            let mut results;

            if let Some(batch) = self.batch {
                results = Vec::with_capacity(requests.len());

                for chunk in requests.chunks(batch) {
                    let chunk_results =
                        self.instance.borrow().aggregate3(chunk.to_vec()).call().await?;

                    results.extend(chunk_results.returnData);
                }
            } else {
                results = self.instance.borrow().aggregate3(requests).call().await?.returnData;
            }

            results
                .into_iter()
                .zip(decoders)
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
