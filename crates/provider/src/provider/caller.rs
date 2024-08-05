use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_rpc_client::WeakClient;
use alloy_transport::{Transport, TransportErrorKind, TransportResult};

use crate::ProviderCall;
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
    ) -> TransportResult<ProviderCall<T, Params, Resp>>;
}

/// A helper struct that implements the [`Caller`] trait and converts [`RpcWithBlock`] into a
/// [`ProviderCall::RpcCall`].
pub(crate) struct WithBlockCall<T>
where
    T: Transport + Clone,
{
    client: WeakClient<T>,
}

impl<T> WithBlockCall<T>
where
    T: Transport + Clone,
{
    pub const fn new(client: WeakClient<T>) -> Self {
        Self { client }
    }
}

impl<T, Params, Resp> Caller<T, Params, Resp> for WithBlockCall<T>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    fn call(
        &self,
        method: Cow<'static, str>,
        params: Params,
    ) -> TransportResult<ProviderCall<T, Params, Resp>> {
        let client = self.client.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;

        let rpc_call = client.request(method, params);

        Ok(ProviderCall::from(rpc_call))
    }
}
