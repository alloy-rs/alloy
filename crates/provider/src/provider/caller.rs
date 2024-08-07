use crate::ProviderCall;
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_rpc_client::WeakClient;
use alloy_rpc_types_eth::BlockId;
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
        block_id: BlockId,
    ) -> TransportResult<ProviderCall<T, serde_json::Value, Resp>>;
}

/// A helper struct that implements the [`Caller`] trait and converts [`RpcWithBlock`] into a
/// [`ProviderCall::RpcCall`].
///
/// [`RpcWithBlock`]: crate::RpcWithBlock
#[derive(Debug)]
pub struct WithBlockCall<T>
where
    T: Transport + Clone,
{
    client: WeakClient<T>,
}

impl<T> WithBlockCall<T>
where
    T: Transport + Clone,
{
    /// Create a new [`WithBlockCall`] instance using transport client.
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
        block_id: BlockId,
    ) -> TransportResult<ProviderCall<T, serde_json::Value, Resp>> {
        let client = self.client.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;

        // serialize the params
        let mut ser = serde_json::to_value(params).map_err(RpcError::ser_err)?;

        // serialize the block id
        let block_id = serde_json::to_value(block_id).map_err(RpcError::ser_err)?;

        // append the block id to the params
        if let serde_json::Value::Array(ref mut arr) = ser {
            arr.push(block_id);
        } else if ser.is_null() {
            ser = serde_json::Value::Array(vec![block_id]);
        } else {
            ser = serde_json::Value::Array(vec![ser, block_id]);
        }

        let rpc_call = client.request(method, ser);

        Ok(ProviderCall::RpcCall(rpc_call))
    }
}

/// EthCaller
#[derive(Debug)]
pub struct EthCaller<T>
where
    T: Transport + Clone,
{
    client: WeakClient<T>,
}

impl<T> EthCaller<T>
where
    T: Transport + Clone,
{
    /// Create a new [`EthCaller`] instance using transport client.
    pub const fn new(client: WeakClient<T>) -> Self {
        Self { client }
    }
}

impl<T, Params, Resp> Caller<T, Params, Resp> for EthCaller<T>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    fn call(
        &self,
        method: Cow<'static, str>,
        params: Params,
        _block_id: BlockId,
    ) -> TransportResult<ProviderCall<T, serde_json::Value, Resp>> {
        let params = serde_json::to_value(params).map_err(RpcError::ser_err)?;

        let rpc_call = self
            .client
            .upgrade()
            .ok_or_else(TransportErrorKind::backend_gone)?
            .request(method, params);

        Ok(ProviderCall::RpcCall(rpc_call))
    }
}
