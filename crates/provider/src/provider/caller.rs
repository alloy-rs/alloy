use super::EthCallParams;
use crate::ProviderCall;
use alloy_json_rpc::RpcReturn;
use alloy_network::Network;
use alloy_rpc_client::WeakClient;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};

/// Trait that helpes convert `EthCall` into a `ProviderCall`.
pub trait Caller<T, N, Resp>: Send + Sync
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
{
    /// Method that needs to be implemented to convert to a `ProviderCall`.
    ///
    /// This method sends the request to relevant data source and returns a `ProviderCall`.
    fn call(
        &self,
        params: EthCallParams<'_, N>,
    ) -> TransportResult<ProviderCall<T, EthCallParams<'static, N>, Resp>>;

    /// Method that needs to be implemented for estimating gas using "eth_estimateGas" for the
    /// transaction.
    fn estimate_gas(
        &self,
        params: EthCallParams<'_, N>,
    ) -> TransportResult<ProviderCall<T, EthCallParams<'static, N>, Resp>>;
}

impl<T, N, Resp> Caller<T, N, Resp> for WeakClient<T>
where
    T: Transport + Clone,
    N: Network,
    Resp: RpcReturn,
{
    fn call(
        &self,
        params: EthCallParams<'_, N>,
    ) -> TransportResult<ProviderCall<T, EthCallParams<'static, N>, Resp>> {
        provider_rpc_call(self, "eth_call", params)
    }

    fn estimate_gas(
        &self,
        params: EthCallParams<'_, N>,
    ) -> TransportResult<ProviderCall<T, EthCallParams<'static, N>, Resp>> {
        provider_rpc_call(self, "eth_estimateGas", params)
    }
}

fn provider_rpc_call<T: Transport + Clone, N: Network, Resp: RpcReturn>(
    client: &WeakClient<T>,
    method: &'static str,
    params: EthCallParams<'_, N>,
) -> TransportResult<ProviderCall<T, EthCallParams<'static, N>, Resp>> {
    let client = client.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;

    let rpc_call = client.request(method, params.into_owned());

    Ok(ProviderCall::RpcCall(rpc_call))
}
