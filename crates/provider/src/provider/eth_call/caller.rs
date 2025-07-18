use super::{EthCallManyParams, EthCallParams};
use crate::ProviderCall;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network::Network;
use alloy_rpc_client::WeakClient;
use alloy_transport::{TransportErrorKind, TransportResult};

/// Trait that helpes convert `EthCall` into a `ProviderCall`.
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
        method: &'static str,
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
        method: &'static str,
        params: EthCallParams<N>,
    ) -> TransportResult<ProviderCall<EthCallParams<N>, Resp>> {
        provider_rpc_call(self, method, params)
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
