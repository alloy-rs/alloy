use crate::{
    batch::BatchRequest, call::RpcCall, transports::Transport, utils::to_json_raw_value,
    TransportError,
};
use alloy_json_rpc::{Id, JsonRpcRequest, RpcParam, RpcReturn};

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct RpcClient<T> {
    pub(crate) transport: T,
    pub(crate) is_local: bool,
    pub(crate) id: AtomicU64,
}

impl<T> RpcClient<T> {
    #[inline]
    pub fn is_local(&self) -> bool {
        self.is_local
    }

    #[inline]
    pub fn increment_id(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn next_id(&self) -> Id {
        Id::Number(self.increment_id())
    }
}

impl<T> RpcClient<T>
where
    T: Transport,
{
    #[inline]
    pub fn new_batch(&self) -> BatchRequest<T> {
        BatchRequest::new(self)
    }

    pub fn make_request<Params: RpcParam>(
        &self,
        method: &'static str,
        params: Params,
    ) -> Result<JsonRpcRequest, TransportError> {
        // Serialize the params greedily, but only return the error lazily
        to_json_raw_value(&params).map(|v| JsonRpcRequest {
            method,
            params: v,
            id: self.next_id(),
        })
    }

    pub fn prepare<Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: &'static str,
        params: Params,
    ) -> RpcCall<T, Params, Resp> {
        let request = self.make_request(method, params);
        RpcCall::new(request, self.transport.clone())
    }
}

#[cfg(test)]
mod test {
    use crate::transports::http::Http;

    use super::RpcClient;

    #[test]
    fn basic_instantiation() {
        let h: RpcClient<Http<reqwest::Client>> = "http://localhost:8545".parse().unwrap();

        assert!(h.is_local());
    }
}
