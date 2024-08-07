use crate::ProviderCall;
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_rpc_client::WeakClient;
use alloy_transport::{RpcError, Transport, TransportErrorKind, TransportResult};
use std::borrow::Cow;

/// A caller that helper convert `RpcWithBlock` and `EthCall` into a `ProviderCall`.
pub trait Caller<T, Params, Resp>: Send + Sync
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    /// Methods that needs to be implemented to convert to a `ProviderCall`.
    fn call(
        &self,
        method: Cow<'static, str>,
        params: Params,
    ) -> TransportResult<ProviderCall<T, serde_json::Value, Resp>>;
}

impl<T, Params, Resp> Caller<T, Params, Resp> for WeakClient<T>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    fn call(
        &self,
        method: Cow<'static, str>,
        params: Params,
    ) -> TransportResult<ProviderCall<T, serde_json::Value, Resp>> {
        let client = self.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;

        // serialize the params
        let ser = serde_json::to_value(params).map_err(RpcError::ser_err)?;

        let rpc_call = client.request(method, ser);

        Ok(ProviderCall::RpcCall(rpc_call))
    }
}
