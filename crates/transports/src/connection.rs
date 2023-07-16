use crate::{
    batch::BatchRequest,
    call::RpcCall,
    error::TransportError,
    types::{Id, JsonRpcRequest, JsonRpcResponse, RpcParam, RpcReturn},
    utils::to_json_raw_value,
};

use std::sync::atomic::{AtomicU64, Ordering};
use tower::Service;

pub trait Transport:
    Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>
    + Service<Vec<JsonRpcRequest>, Response = Vec<JsonRpcResponse>, Error = TransportError>
    + Clone
    + 'static
{
}

impl<T> Transport for T where
    T: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>
        + Service<Vec<JsonRpcRequest>, Response = Vec<JsonRpcResponse>, Error = TransportError>
        + Clone
        + 'static
{
}

pub struct RpcClient<T> {
    pub(crate) transport: T,
    pub(crate) is_local: bool,
    pub(crate) id: AtomicU64,
}

impl<T> RpcClient<T>
where
    T: Transport,
{
    #[inline]
    pub fn increment_id(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn next_id(&self) -> Id {
        Id::Number(self.increment_id())
    }

    pub fn new_batch(&self) -> BatchRequest<T> {
        BatchRequest::new(self.transport.clone())
    }

    pub fn prepare<Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: &'static str,
        params: Params,
    ) -> RpcCall<T, Params, Resp> {
        // Serialize the params greedily, but only return the error lazily
        let request = to_json_raw_value(&params).map(|v| JsonRpcRequest {
            method,
            params: v,
            id: self.next_id(),
        });

        RpcCall::new(request, self.transport.clone())
    }

    pub fn is_local(&self) -> bool {
        self.is_local
    }
}

#[cfg(test)]
mod test {
    use crate::transports::http::Http;

    use super::RpcClient;

    #[test]
    fn basic_instantiation() {
        let h: RpcClient<Http<reqwest::Client>> = "http://localhost:8545".parse().unwrap();

        assert_eq!(h.is_local(), true);
    }
}
