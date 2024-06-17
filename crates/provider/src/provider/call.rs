use alloy_eips::BlockId;
use alloy_json_rpc::RpcReturn;
use alloy_network::Network;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_rpc_types_eth::state::StateOverride;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;
use serde::ser::SerializeSeq;
use std::{future::Future, marker::PhantomData, task::Poll};

type RunningFut<'req, 'state, T, N, Resp, Output, Map> =
    RpcCall<T, EthCallParams<'req, 'state, N>, Resp, Output, Map>;

#[derive(Clone, Debug)]
struct EthCallParams<'req, 'state, N: Network> {
    data: &'req N::TransactionRequest,
    block: Option<BlockId>,
    overrides: Option<&'state StateOverride>,
}

impl<N: Network> serde::Serialize for EthCallParams<'_, '_, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = if self.overrides.is_some() { 3 } else { 2 };

        let mut seq = serializer.serialize_seq(Some(len))?;
        seq.serialize_element(&self.data)?;

        if let Some(overrides) = self.overrides {
            seq.serialize_element(&self.block.unwrap_or_default())?;
            seq.serialize_element(overrides)?;
        } else if let Some(block) = self.block {
            seq.serialize_element(&block)?;
        }

        seq.end()
    }
}

/// The [`EthCallFut`] future is the future type for an `eth_call` RPC request.
#[derive(Clone, Debug)]
#[doc(hidden)] // Not public API.
#[pin_project::pin_project]
pub struct EthCallFut<'req, 'state, T, N, Resp, Output, Map>(
    EthCallFutInner<'req, 'state, T, N, Resp, Output, Map>,
)
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output;

#[derive(Clone, Debug)]
enum EthCallFutInner<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    Preparing {
        client: WeakClient<T>,
        data: &'req N::TransactionRequest,
        overrides: Option<&'state StateOverride>,
        block: Option<BlockId>,
        method: &'static str,
        map: Map,
    },
    Running(RunningFut<'req, 'state, T, N, Resp, Output, Map>),
    Polling,
}

impl<'req, 'state, T, N, Resp, Output, Map> EthCallFutInner<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    /// Returns `true` if the future is in the preparing state.
    const fn is_preparing(&self) -> bool {
        matches!(self, Self::Preparing { .. })
    }

    /// Returns `true` if the future is in the running state.
    const fn is_running(&self) -> bool {
        matches!(self, Self::Running(..))
    }

    fn poll_preparing(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let Self::Preparing { client, data, overrides, block, method, map } =
            std::mem::replace(self, Self::Polling)
        else {
            unreachable!("bad state")
        };

        let client = match client.upgrade().ok_or_else(TransportErrorKind::backend_gone) {
            Ok(client) => client,
            Err(e) => return Poll::Ready(Err(e)),
        };

        let params = EthCallParams { data, block, overrides };

        let fut = client.request(method, params).map_resp(map);

        *self = Self::Running(fut);
        self.poll_running(cx)
    }

    fn poll_running(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let Self::Running(ref mut call) = self else { unreachable!("bad state") };

        call.poll_unpin(cx)
    }
}

impl<'req, 'state, T, N, Resp, Output, Map> Future
    for EthCallFut<'req, 'state, T, N, Resp, Output, Map>
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
        let this = &mut self.get_mut().0;
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
#[derive(Debug, Clone)]
pub struct EthCall<'req, 'state, T, N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    client: WeakClient<T>,

    data: &'req N::TransactionRequest,
    overrides: Option<&'state StateOverride>,
    block: Option<BlockId>,
    method: &'static str,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
}

impl<'req, T, N, Resp> EthCall<'req, 'static, T, N, Resp>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
{
    /// Create a new CallBuilder.
    pub const fn new(client: WeakClient<T>, data: &'req N::TransactionRequest) -> Self {
        Self {
            client,
            data,
            overrides: None,
            block: None,
            method: "eth_call",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }

    /// Create new EthCall for gas estimates.
    pub const fn gas_estimate(client: WeakClient<T>, data: &'req N::TransactionRequest) -> Self {
        Self {
            client,
            data,
            overrides: None,
            block: None,
            method: "eth_estimateGas",
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<'req, 'state, T, N, Resp, Output, Map> EthCall<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    /// Map the response to a different type.
    pub fn map_resp<NewOutput, NewMap>(
        self,
        map: NewMap,
    ) -> EthCall<'req, 'state, T, N, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        EthCall {
            client: self.client,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
            method: self.method,
            map,
            _pd: PhantomData,
        }
    }

    /// Set the state overrides for this call.
    pub const fn overrides(mut self, overrides: &'state StateOverride) -> Self {
        self.overrides = Some(overrides);
        self
    }

    /// Set the block to use for this call.
    pub const fn block(mut self, block: BlockId) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'req, 'state, T, N, Resp, Output, Map> std::future::IntoFuture
    for EthCall<'req, 'state, T, N, Resp, Output, Map>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = EthCallFut<'req, 'state, T, N, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        EthCallFut(EthCallFutInner::Preparing {
            client: self.client,
            data: self.data,
            overrides: self.overrides,
            block: self.block,
            method: self.method,
            map: self.map,
        })
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
        let params: EthCallParams<'_, '_, Ethereum> =
            EthCallParams { data: &data, block: None, overrides: None };

        assert_eq!(params.data, &data);
        assert_eq!(params.block, None);
        assert_eq!(params.overrides, None);
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"}]"#
        );

        // Expected: [data, block, overrides]
        let params: EthCallParams<'_, '_, Ethereum> =
            EthCallParams { data: &data, block: Some(block), overrides: Some(&overrides) };

        assert_eq!(params.data, &data);
        assert_eq!(params.block, Some(block));
        assert_eq!(params.overrides, Some(&overrides));
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"0x1",{}]"#
        );

        // Expected: [data, (default), overrides]
        let params: EthCallParams<'_, '_, Ethereum> =
            EthCallParams { data: &data, block: None, overrides: Some(&overrides) };

        assert_eq!(params.data, &data);
        assert_eq!(params.block, None);
        assert_eq!(params.overrides, Some(&overrides));
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"latest",{}]"#
        );

        // Expected: [data, block]
        let params: EthCallParams<'_, '_, Ethereum> =
            EthCallParams { data: &data, block: Some(block), overrides: None };

        assert_eq!(params.data, &data);
        assert_eq!(params.block, Some(block));
        assert_eq!(params.overrides, None);
        assert_eq!(
            serde_json::to_string(&params).unwrap(),
            r#"[{"from":"0x0000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000002","maxFeePerGas":"0x4a817c800","maxPriorityFeePerGas":"0x3b9aca00","gas":"0x5208","value":"0x64","nonce":"0x0","chainId":"0x1"},"0x1"]"#
        );
    }
}
