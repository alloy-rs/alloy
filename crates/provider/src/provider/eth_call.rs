use alloy_eips::BlockId;
use alloy_json_rpc::RpcReturn;
use alloy_network::Network;
use alloy_rpc_types_eth::state::StateOverride;
use alloy_transport::{Transport, TransportResult};
use futures::FutureExt;
use serde::ser::SerializeSeq;
use std::{borrow::Cow, future::Future, marker::PhantomData, sync::Arc, task::Poll};

use crate::{Caller, ProviderCall};

/// The parameters for an `"eth_call"` RPC request.
#[derive(Clone, Debug)]
pub struct EthCallParams<'req, N: Network> {
    data: Cow<'req, N::TransactionRequest>,
    block: Option<BlockId>,
    overrides: Option<Cow<'req, StateOverride>>,
}

impl<'req, N> EthCallParams<'req, N>
where
    N: Network,
{
    /// Instantiates a new `EthCallParams` with the given data (transaction).
    pub const fn new(data: &'req N::TransactionRequest) -> Self {
        Self { data: Cow::Borrowed(data), block: None, overrides: None }
    }

    /// Sets the block to use for this call.
    pub const fn with_block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the state overrides for this call.
    pub fn with_overrides(mut self, overrides: &'req StateOverride) -> Self {
        self.overrides = Some(Cow::Borrowed(overrides));
        self
    }

    /// Returns a reference to the state overrides if set.
    pub fn overrides(&self) -> Option<&StateOverride> {
        self.overrides.as_deref()
    }

    /// Returns a reference to the transaction data.
    pub fn data(&self) -> &N::TransactionRequest {
        &self.data
    }

    /// Returns the block.
    pub const fn block(&self) -> Option<BlockId> {
        self.block
    }

    /// Clones the tx data and overrides into owned data.
    pub fn into_owned(self) -> EthCallParams<'static, N> {
        EthCallParams {
            data: Cow::Owned(self.data.into_owned()),
            block: self.block,
            overrides: self.overrides.map(|o| Cow::Owned(o.into_owned())),
        }
    }
}

impl<N: Network> serde::Serialize for EthCallParams<'_, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
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
    }
}

/// The [`EthCallFut`] future is the future type for an `eth_call` RPC request.
#[derive(Debug)]
#[doc(hidden)] // Not public API.
#[allow(unnameable_types)]
#[pin_project::pin_project]
pub struct EthCallFut<'req, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    inner: EthCallFutInner<'req, T, N, Resp, Output, Map>,
}

enum EthCallFutInner<'req, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    Preparing {
        caller: Arc<dyn Caller<T, N, Resp>>,
        data: &'req N::TransactionRequest,
        overrides: Option<&'req StateOverride>,
        block: Option<BlockId>,
        method: &'static str,
        map: Map,
    },
    Running {
        map: Map,
        fut: ProviderCall<T, EthCallParams<'static, N>, Resp>,
    },
    Polling,
}

impl<T, N, Resp, Output, Map> core::fmt::Debug for EthCallFutInner<'_, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Preparing { caller: _, data, overrides, block, method, map: _ } => f
                .debug_struct("Preparing")
                .field("data", data)
                .field("overrides", overrides)
                .field("block", block)
                .field("method", method)
                .finish(),
            Self::Running { .. } => f.debug_tuple("Running").finish(),
            Self::Polling => f.debug_tuple("Polling").finish(),
        }
    }
}

impl<T, N, Resp, Output, Map> EthCallFut<'_, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
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
        let EthCallFutInner::Preparing { caller, data, overrides, block, method, map } =
            std::mem::replace(&mut self.inner, EthCallFutInner::Polling)
        else {
            unreachable!("bad state")
        };

        let params = EthCallParams {
            data: Cow::Borrowed(data),
            block,
            overrides: overrides.map(Cow::Borrowed),
        };

        let fut =
            if method.eq("eth_call") { caller.call(params) } else { caller.estimate_gas(params) }?;

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

impl<T, N, Resp, Output, Map> Future for EthCallFut<'_, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
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
pub struct EthCall<'req, T, N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    caller: Arc<dyn Caller<T, N, Resp>>,
    data: &'req N::TransactionRequest,
    overrides: Option<&'req StateOverride>,
    block: Option<BlockId>,
    method: &'static str,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
}

impl<T, N, Resp> core::fmt::Debug for EthCall<'_, T, N, Resp>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthCall")
            .field("method", &self.method)
            .field("data", &self.data)
            .field("block", &self.block)
            .field("overrides", &self.overrides)
            .finish()
    }
}

impl<'req, T, N, Resp> EthCall<'req, T, N, Resp>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
{
    /// Create a new CallBuilder.
    pub fn new(
        caller: impl Caller<T, N, Resp> + 'static,
        data: &'req N::TransactionRequest,
    ) -> Self {
        Self {
            caller: Arc::new(caller),
            data,
            overrides: None,
            block: None,
            method: "eth_call",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }

    /// Create new EthCall for gas estimates.
    pub fn gas_estimate(
        caller: impl Caller<T, N, Resp> + 'static,
        data: &'req N::TransactionRequest,
    ) -> Self {
        Self {
            caller: Arc::new(caller),
            data,
            overrides: None,
            block: None,
            method: "eth_estimateGas",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<'req, T, N, Resp, Output, Map> EthCall<'req, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
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
    ) -> EthCall<'req, T, N, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        EthCall {
            caller: self.caller,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
            method: self.method,
            map,
            _pd: PhantomData,
        }
    }

    /// Set the state overrides for this call.
    pub const fn overrides(mut self, overrides: &'req StateOverride) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// Set the block to use for this call.
    pub const fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'req, T, N, Resp, Output, Map> std::future::IntoFuture
    for EthCall<'req, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = EthCallFut<'req, T, N, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        EthCallFut {
            inner: EthCallFutInner::Preparing {
                caller: self.caller,
                data: self.data,
                overrides: self.overrides,
                block: self.block,
                method: self.method,
                map: self.map,
            },
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
        let params: EthCallParams<'_, Ethereum> = EthCallParams::new(&data);

        assert_eq!(params.data(), &data);
        assert_eq!(params.block(), None);
        assert_eq!(params.overrides(), None);
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"}]"#
        );

        // Expected: [data, block, overrides]
        let params: EthCallParams<'_, Ethereum> =
            EthCallParams::new(&data).with_block(block).with_overrides(&overrides);

        assert_eq!(params.data(), &data);
        assert_eq!(params.block(), Some(block));
        assert_eq!(params.overrides(), Some(&overrides));
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"0x1",{}]"#
        );

        // Expected: [data, (default), overrides]
        let params: EthCallParams<'_, Ethereum> =
            EthCallParams::new(&data).with_overrides(&overrides);

        assert_eq!(params.data(), &data);
        assert_eq!(params.block(), None);
        assert_eq!(params.overrides(), Some(&overrides));
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"latest",{}]"#
        );

        // Expected: [data, block]
        let params: EthCallParams<'_, Ethereum> = EthCallParams::new(&data).with_block(block);

        assert_eq!(params.data(), &data);
        assert_eq!(params.block(), Some(block));
        assert_eq!(params.overrides(), None);
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"0x1"]"#
        );
    }
}
