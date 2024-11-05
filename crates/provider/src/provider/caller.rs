use super::EthCallParams;
use crate::ProviderCall;
use alloy_json_rpc::RpcReturn;
use alloy_network::Network;
use alloy_rpc_client::WeakClient;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use std::borrow::Cow;

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
        method: Cow<'static, str>,
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
        method: Cow<'static, str>,
        params: EthCallParams<'_, N>,
    ) -> TransportResult<ProviderCall<T, EthCallParams<'static, N>, Resp>> {
        let client = self.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;

        let rpc_call = client.request(method, params.into_owned());

        Ok(ProviderCall::RpcCall(rpc_call))
    }
}
