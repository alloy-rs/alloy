/// The canon deployed address and chains
pub mod constants;
use std::marker::PhantomData;

pub use constants::{MULTICALL_ADDRESS, MULTICALL_SUPPORTED_CHAINS};

mod error;
pub use error::MultiCallError;

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
    #[sol(rpc, abi)]
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

/// An instance of a dynamically typed MultiCall.
pub type DynMultiCall<T, P, N> = MultiCall<T, P, Function, N>;

/// An instance of static typed MultiCall.
pub type SolMultiCall<T, P, C, N> = MultiCall<T, P, PhantomData<C>, N>;

/// The MultiCall struct is used to aggregate multiple calls into a single call.
#[derive(Debug)]
pub struct MultiCall<T, P, D: CallDecoder, N: Network> {
    instance: IMulticall3::IMulticall3Instance<T, P, N>,
    calls: Vec<(bool, CallBuilder<T, P, D, N>)>,
    batch: Option<usize>,
}

impl<T, P, D, N> MultiCall<T, P, D, N>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    D: CallDecoder,
    N: Network,
{
    /// Create a new multicall instance.
    /// 
    /// # Errors
    /// - If the chain_id is not in the list of supported chains.
    pub async fn new(provider: P, address: Option<Address>) -> Result<Self, MultiCallError> {
        let instance = IMulticall3::IMulticall3Instance::new(
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
        );

        Ok(Self { instance, calls: vec![], batch: None })
    }

    /// Add a call to the multicall instance.
    pub fn add_call(&mut self, call: CallBuilder<T, P, D, N>, allow_failure: bool) {
        self.calls.push((allow_failure, call));
    }

    /// Add multiple calls to the multicall instance.
    pub fn add_calls<I>(&mut self, calls: I)
    where
        I: Iterator<Item = (bool, CallBuilder<T, P, D, N>)>,
    {
        self.calls.extend(calls);
    }

    /// Set the batch size 
    pub fn batch(&mut self, batch: Option<usize>) {
        self.batch = batch;
    }
}

impl<T, P, D, N> MultiCall<T, P, D, N>
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
    D: CallDecoder,
{
    /// Like [Self::aggregate] method but doesnt consume the calls instead
    pub async fn aggregate_ref(&self) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let (decoders, requests) = self.parts_ref();

        self.aggregate_inner(
            &decoders,
            requests
                .into_iter()
                .map(|(_, call)| {
                    call_from_tx_ref::<N>(call)
                })
                .collect::<Result<Vec<_>, MultiCallError>>()?,
        )
        .await
    }

    /// Calls aggreagte, without cloning any of the calldata
    ///
    /// Aggreagte will revert on the first failure and ignores any failure mode set on the individual calls
    pub async fn aggregate(&mut self) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let (decoders, requests) = self.parts();

        self.aggregate_inner(
            decoders.iter().collect::<Vec<_>>().as_slice(),
            requests
                .into_iter()
                .map(|(_, call)| {
                    call_from_tx::<N>(call)
                })
                .collect::<Result<Vec<_>, MultiCallError>>()?,
        )
        .await
    }

    /// Like [Self::try_aggregate] method but clones the calls
    pub async fn try_aggregate_ref(
        &self,
        require_success: bool,
    ) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let (decoders, requests) = self.parts_ref();

        self.try_aggregate_inner(
            require_success,
            &decoders,
            requests
                .into_iter()
                .map(|(_, call)| {
                    call_from_tx_ref::<N>(call)
                })
                .collect::<Result<Vec<_>, MultiCallError>>()?,
        )
        .await
    }

    /// Calls try_aggregate, without cloning any of the calldata, this method ignores the failure mode set on the individual calls
    pub async fn try_aggregate(
        &mut self,
        require_success: bool,
    ) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let (decoders, requests) = self.parts();

        self.try_aggregate_inner(
            require_success,
            decoders.iter().collect::<Vec<_>>().as_slice(),
            requests
                .into_iter()
                .map(|(_, call)| {
                    call_from_tx::<N>(call)
                })
                .collect::<Result<Vec<_>, MultiCallError>>()?,
        )
        .await
    }

    /// Like [Self::aggregate3] method but clones the calls
    pub async fn aggregate3_ref(&self) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let (decoders, requests) = self.parts_ref();

        self.aggregate3_inner(
            &decoders,
            requests
                .into_iter()
                .map(|(allow_failure, call)| {
                    call3_from_tx_ref::<N>(call, allow_failure)
                })
                .collect::<Result<Vec<_>, MultiCallError>>()?,
        )
        .await
    }

    /// Calls aggregate3, without cloning any of the calldata
    pub async fn aggregate3(&mut self) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let (decoders, requests) = self.parts();

        self.aggregate3_inner(
            decoders.iter().collect::<Vec<_>>().as_slice(),
            requests
                .into_iter()
                .map(|(allow_failure, call)| {
                    call3_from_tx::<N>(call, allow_failure)
                })
                .collect::<Result<Vec<_>, MultiCallError>>()?,
        )
        .await
    }

    /// Cleas the calls if any
    pub fn clear_calls(&mut self) {
        self.calls.clear();
    }
}

impl<T, P, D, N> MultiCall<T, P, D, N>
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
    D: CallDecoder,
{
    /// Call the aggregate method, this method will revert on the first failure regardless of what
    /// you set
    async fn aggregate_inner(
        &self,
        decoders: &[&D],
        requests: Vec<IMulticall3::Call>,
    ) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let mut results;

        if let Some(batch) = self.batch {
            results = Vec::with_capacity(requests.len());

            for chunk in requests.chunks(batch) {
                let chunk_results = self.instance.aggregate(chunk.to_vec()).call().await?;

                results.extend(chunk_results.returnData);
            }
        } else {
            results = self.instance.aggregate(requests).call().await?.returnData;
        }

        results
            .into_iter()
            .zip(decoders.into_iter())
            .map(|(out, decoder)| decoder.abi_decode_output(out, true))
            .map(|r| r.map_err(Into::into))
            .collect()
    }

    /// Try to aggregate the calls, this method ignores the failure mode set on the individual calls
    pub async fn try_aggregate_inner(
        &self,
        require_success: bool,
        decoders: &[&D],
        requests: Vec<IMulticall3::Call>,
    ) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let mut results;

        if let Some(batch) = self.batch {
            results = Vec::with_capacity(requests.len());

            for chunk in requests.chunks(batch) {
                let chunk_results = self.instance.tryAggregate(require_success, chunk.to_vec()).call().await?;

                results.extend(chunk_results.returnData);
            }
        } else {
            results = self.instance.tryAggregate(require_success, requests).call().await?.returnData;
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

    /// Call the aggregate3 method, this method utilizes the allow_failure flag on the individual
    /// calls
    pub async fn aggregate3_inner(
        &self,
        decoders: &[&D],
        requests: Vec<IMulticall3::Call3>,
    ) -> Result<Vec<D::CallOutput>, MultiCallError> {
        let mut results;

        if let Some(batch) = self.batch {
            results = Vec::with_capacity(requests.len());

            for chunk in requests.chunks(batch) {
                let chunk_results = self.instance.aggregate3(chunk.to_vec()).call().await?;

                results.extend(chunk_results.returnData);
            }
        } else {
            results = self.instance.aggregate3(requests).call().await?.returnData;
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

fn call3_from_tx<N>(tx: N::TransactionRequest, allow_failure: bool) -> Result<IMulticall3::Call3, MultiCallError>
where
    N: Network,
{
    Ok(IMulticall3::Call3 {
        target: tx.to().ok_or(error::MultiCallError::MissingTargetAddress)?,
        allowFailure: allow_failure,
        callData: tx.into_input().unwrap_or_default(),
    })
}

fn call3_from_tx_ref<N>(
    tx: &N::TransactionRequest,
    allow_failure: bool,
) -> Result<IMulticall3::Call3, MultiCallError>
where
    N: Network,
{
    Ok(IMulticall3::Call3 {
        target: tx.to().ok_or(error::MultiCallError::MissingTargetAddress)?,
        allowFailure: allow_failure,
        callData: tx.input().cloned().unwrap_or_default(),
    })
}

fn call_from_tx<N>(tx: N::TransactionRequest) -> Result<IMulticall3::Call, MultiCallError>
where
    N: Network,
{
    Ok(IMulticall3::Call {
        target: tx.to().ok_or(error::MultiCallError::MissingTargetAddress)?,
        callData: tx.into_input().unwrap_or_default(),
    })
}

fn call_from_tx_ref<N>(tx: &N::TransactionRequest) -> Result<IMulticall3::Call, MultiCallError>
where
    N: Network,
{
    Ok(IMulticall3::Call {
        target: tx.to().ok_or(error::MultiCallError::MissingTargetAddress)?,
        callData: tx.input().cloned().unwrap_or_default(),
    })
}