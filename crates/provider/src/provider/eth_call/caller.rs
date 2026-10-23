use super::{EthCallManyParams, EthCallParams};
use crate::ProviderCall;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network::Network;
use alloy_rpc_client::WeakClient;
use alloy_transport::{SharedTransportError, TransportErrorKind, TransportResult};

/// Trait that helps convert `EthCall` into a `ProviderCall`.
pub trait Caller<N, Resp>: Send + Sync
where
    N: Network,
    Resp: RpcRecv,
{
    /// Method that needs to be implemented to convert to a `ProviderCall`.
    ///
    /// This method sends the request to relevant data source and returns a `ProviderCall`.
    fn call(
        &self,
        params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>>;

    /// Method that needs to be implemented for estimating gas using "eth_estimateGas" for the
    /// transaction.
    fn estimate_gas(
        &self,
        params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>>;

    /// Method that needs to be implemented for `"eth_callMany"` RPC requests.
    fn call_many(
        &self,
        params: EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<EthCallManyParams<'static>, Resp>>;
}

impl<N, Resp> Caller<N, Resp> for WeakClient
where
    N: Network,
    Resp: RpcRecv,
{
    fn call(
        &self,
        params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>> {
        provider_rpc_call(self, "eth_call", params)
    }

    fn estimate_gas(
        &self,
        params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>> {
        provider_rpc_call(self, "eth_estimateGas", params)
    }

    fn call_many(
        &self,
        params: EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<EthCallManyParams<'static>, Resp>> {
        provider_rpc_call(self, "eth_callMany", params.into_owned())
    }
}

/// Returns a [`ProviderCall::RpcCall`] from the provided method and [`EthCallParams`].
fn provider_rpc_call<Req: RpcSend, Resp: RpcRecv>(
    client: &WeakClient,
    method: &'static str,
    params: Req,
) -> TransportResult<ProviderCall<Req, Resp>> {
    let client = client.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;
    let rpc_call = client.request(method, params);
    Ok(ProviderCall::RpcCall(rpc_call))
}

/// A caller that reports a preparation failure without dispatching a request.
#[derive(Debug)]
pub(super) struct ErrorCaller(pub(super) SharedTransportError);

impl<N: Network, Resp: RpcRecv> Caller<N, Resp> for ErrorCaller {
    fn call(
        &self,
        _params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>> {
        Err(self.0.to_error())
    }

    fn estimate_gas(
        &self,
        _params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>> {
        Err(self.0.to_error())
    }

    fn call_many(
        &self,
        _params: EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<EthCallManyParams<'static>, Resp>> {
        Err(self.0.to_error())
    }
}
