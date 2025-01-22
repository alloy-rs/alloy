use crate::ProviderCall;
use alloy_eips::BlockId;
use alloy_json_rpc::RpcRecv;
use alloy_network::Network;
use alloy_rpc_types_eth::{state::StateOverride, Bundle, StateContext, TransactionIndex};
use alloy_transport::{TransportError, TransportResult};
use futures::FutureExt;
use serde::ser::SerializeSeq;
use std::{future::Future, marker::PhantomData, sync::Arc, task::Poll};

mod params;
pub use params::{CallManyParams, CallParams, EthCallParams};

mod caller;
pub use caller::Caller;

/// The [`EthCallFut`] future is the future type for an `eth_call` RPC request.
#[derive(Debug)]
#[doc(hidden)] // Not public API.
#[allow(unnameable_types)]
#[pin_project::pin_project]
pub struct EthCallFut<'req, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    inner: EthCallFutInner<'req, N, Resp, Output, Map>,
}

enum EthCallFutInner<'req, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    Preparing {
        caller: Arc<dyn Caller<N, Resp>>,
        params: EthCallParams<'req, N>,
        method: &'static str,
        map: Map,
    },
    Running {
        map: Map,
        fut: ProviderCall<EthCallParams<'static, N>, Resp>,
    },
    Polling,
}

impl<N, Resp, Output, Map> core::fmt::Debug for EthCallFutInner<'_, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Preparing { caller: _, params, method, map: _ } => {
                f.debug_struct("Preparing").field("params", params).field("method", method).finish()
            }
            Self::Running { .. } => f.debug_tuple("Running").finish(),
            Self::Polling => f.debug_tuple("Polling").finish(),
        }
    }
}

impl<N, Resp, Output, Map> EthCallFut<'_, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    /// Returns `true` if the future is in the preparing state.
    const fn is_preparing(&self) -> bool {
        matches!(self.inner, EthCallFutInner::Preparing { .. })
    }

    /// Returns `true` if the future is in the running state.
    const fn is_running(&self) -> bool {
        matches!(self.inner, EthCallFutInner::Running { .. })
    }

    fn poll_preparing(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let EthCallFutInner::Preparing { caller, params, method, map } =
            std::mem::replace(&mut self.inner, EthCallFutInner::Polling)
        else {
            unreachable!("bad state")
        };

        let fut = if params.is_call() {
            if method.ne("eth_call") && method.ne("eth_estimateGas") {
                return Poll::Ready(Err(TransportError::local_usage_str(&format!(
                    "bad method: {method} - params provided for `eth_call`/`eth_estimateGas`"
                ))));
            }
            if method.eq("eth_call") {
                caller.call(params)
            } else {
                caller.estimate_gas(params)
            }
        } else {
            if method.eq("eth_callMany") {
                caller.call_many(params)
            } else {
                return Poll::Ready(Err(TransportError::local_usage_str(&format!(
                    "bad method: {method} - params provided for `eth_callMany`"
                ))));
            }
        }?;

        self.inner = EthCallFutInner::Running { map, fut };

        self.poll_running(cx)
    }

    fn poll_running(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let EthCallFutInner::Running { ref map, ref mut fut } = self.inner else {
            unreachable!("bad state")
        };

        fut.poll_unpin(cx).map(|res| res.map(map))
    }
}

impl<N, Resp, Output, Map> Future for EthCallFut<'_, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        if this.is_preparing() {
            this.poll_preparing(cx)
        } else if this.is_running() {
            this.poll_running(cx)
        } else {
            panic!("unexpected state")
        }
    }
}

/// A builder for an `"eth_call"` request. This type is returned by the
/// [`Provider::call`] method.
///
/// [`Provider::call`]: crate::Provider::call
#[must_use = "EthCall must be awaited to execute the call"]
#[derive(Clone)]
pub struct EthCall<'req, N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    caller: Arc<dyn Caller<N, Resp>>,
    params: EthCallParams<'req, N>,
    method: &'static str,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
}

impl<N, Resp> core::fmt::Debug for EthCall<'_, N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthCall")
            .field("params", &self.params)
            .field("method", &self.method)
            .finish()
    }
}

impl<'req, N, Resp> EthCall<'req, N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    /// Create a new [`EthCall`].
    pub fn new(
        caller: impl Caller<N, Resp> + 'static,
        method: &'static str,
        data: &'req N::TransactionRequest,
    ) -> Self {
        Self {
            caller: Arc::new(caller),
            params: EthCallParams::call(data),
            method,
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }

    /// Create a new [`EthCall`] with method set to `"eth_call"` with params set to [`CallParams`].
    pub fn call(caller: impl Caller<N, Resp> + 'static, data: &'req N::TransactionRequest) -> Self {
        Self::new(caller, "eth_call", data)
    }

    /// Create a new [`EthCall`] with method set to `"eth_estimateGas"` with params set to
    /// [`CallParams`].
    pub fn gas_estimate(
        caller: impl Caller<N, Resp> + 'static,
        data: &'req N::TransactionRequest,
    ) -> Self {
        Self::new(caller, "eth_estimateGas", data)
    }

    /// Create a new [`EthCall`] with method set to `"eth_callMany"` with params set to
    /// [`CallManyParams`].
    pub fn call_many(caller: impl Caller<N, Resp> + 'static, bundles: &'req Vec<Bundle>) -> Self {
        Self {
            caller: Arc::new(caller),
            params: EthCallParams::call_many(bundles),
            method: "eth_callMany",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<'req, N, Resp, Output, Map> EthCall<'req, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Map the response to a different type. This is usable for converting
    /// the response to a more usable type, e.g. changing `U64` to `u64`.
    ///
    /// ## Note
    ///
    /// Carefully review the rust documentation on [fn pointers] before passing
    /// them to this function. Unless the pointer is specifically coerced to a
    /// `fn(_) -> _`, the `NewMap` will be inferred as that function's unique
    /// type. This can lead to confusing error messages.
    ///
    /// [fn pointers]: https://doc.rust-lang.org/std/primitive.fn.html#creating-function-pointers
    pub fn map_resp<NewOutput, NewMap>(
        self,
        map: NewMap,
    ) -> EthCall<'req, N, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        EthCall {
            caller: self.caller,
            params: self.params,
            method: self.method,
            map,
            _pd: PhantomData,
        }
    }

    /// Set the state overrides for this call.
    pub fn overrides(mut self, overrides: &'req StateOverride) -> Self {
        self.params = self.params.with_overrides(overrides);
        self
    }

    /// Set the block to use for this call.
    ///
    /// In case of `"eth_callMany"` requests, this sets the block in the [`StateContext`].
    pub fn block(mut self, block: BlockId) -> Self {
        self.params = self.params.with_block(block);
        self
    }

    /// Set the [`TransactionIndex`] in the [`StateContext`] for an `eth_callMany` request.
    pub fn transaction_index(mut self, transaction_index: TransactionIndex) -> Self {
        self.params = self.params.with_transaction_index(transaction_index);
        self
    }

    /// Set the state context for an `eth_callMany` request.
    pub fn context(mut self, state_context: StateContext) -> Self {
        self.params = self.params.with_context(state_context);
        self
    }
}

impl<'req, N, Resp, Output, Map> std::future::IntoFuture for EthCall<'req, N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = EthCallFut<'req, N, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        EthCallFut {
            inner: EthCallFutInner::Preparing {
                caller: self.caller,
                params: self.params,
                method: self.method,
                map: self.map,
            },
        }
    }
}

impl<N: Network> serde::Serialize for EthCallParams<'_, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.is_call() {
            let len = if self.overrides().is_some() { 3 } else { 2 };

            let mut seq = serializer.serialize_seq(Some(len))?;
            seq.serialize_element(&self.data())?;

            if let Some(overrides) = self.overrides() {
                seq.serialize_element(&self.block().unwrap_or_default())?;
                seq.serialize_element(overrides)?;
            } else if let Some(block) = self.block() {
                seq.serialize_element(&block)?;
            }

            seq.end()
        } else {
            let len = if self.overrides().is_some() { 3 } else { 2 };

            let mut seq = serializer.serialize_seq(Some(len))?;

            seq.serialize_element(&self.bundles())?;

            if let Some(context) = self.context() {
                seq.serialize_element(context)?;
            }

            if let Some(overrides) = self.overrides() {
                seq.serialize_element(overrides)?;
            }

            seq.end()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_eips::BlockNumberOrTag;
    use alloy_network::{Ethereum, TransactionBuilder};
    use alloy_primitives::{address, U256};
    use alloy_rpc_types_eth::{state::StateOverride, TransactionRequest};

    #[test]
    fn test_serialize_eth_call_params() {
        let alice = address!("0000000000000000000000000000000000000001");
        let bob = address!("0000000000000000000000000000000000000002");
        let data = TransactionRequest::default()
            .with_from(alice)
            .with_to(bob)
            .with_nonce(0)
            .with_chain_id(1)
            .value(U256::from(100))
            .with_gas_limit(21_000)
            .with_max_priority_fee_per_gas(1_000_000_000)
            .with_max_fee_per_gas(20_000_000_000);

        let block = BlockId::Number(BlockNumberOrTag::Number(1));
        let overrides = StateOverride::default();

        // Expected: [data]
        let params: EthCallParams<'_, Ethereum> = EthCallParams::call(&data);

        assert_eq!(params.data().unwrap(), &data);
        assert_eq!(params.block(), None);
        assert_eq!(params.overrides(), None);
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"}]"#
        );

        // Expected: [data, block, overrides]
        let params: EthCallParams<'_, Ethereum> =
            EthCallParams::call(&data).with_block(block).with_overrides(&overrides);

        assert_eq!(params.data().unwrap(), &data);
        assert_eq!(params.block(), Some(block));
        assert_eq!(params.overrides(), Some(&overrides));
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"0x1",{}]"#
        );

        // Expected: [data, (default), overrides]
        let params: EthCallParams<'_, Ethereum> =
            EthCallParams::call(&data).with_overrides(&overrides);

        assert_eq!(params.data().unwrap(), &data);
        assert_eq!(params.block(), None);
        assert_eq!(params.overrides(), Some(&overrides));
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"latest",{}]"#
        );

        // Expected: [data, block]
        let params: EthCallParams<'_, Ethereum> = EthCallParams::call(&data).with_block(block);

        assert_eq!(params.data().unwrap(), &data);
        assert_eq!(params.block(), Some(block));
        assert_eq!(params.overrides(), None);
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"0x1"]"#
        );
    }
}
