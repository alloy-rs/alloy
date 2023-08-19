use alloy_json_rpc::{Id, JsonRpcRequest, RpcParam, RpcReturn};
use serde_json::value::RawValue;
use tower::{layer::util::Stack, util::BoxCloneService, Layer, ServiceBuilder};

use std::{
    borrow::Cow,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{BatchRequest, RpcCall, Transport, TransportError};

#[derive(Debug)]
pub struct RpcClient<T> {
    pub(crate) transport: T,
    pub(crate) is_local: bool,
    pub(crate) id: AtomicU64,
}

impl<T> RpcClient<T> {
    pub fn new(t: T, is_local: bool) -> Self {
        Self {
            transport: t,
            is_local,
            id: AtomicU64::new(0),
        }
    }

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
    T: Transport + Clone,
    T::Future: Send,
{
    #[inline]
    pub fn new_batch(&self) -> BatchRequest<T> {
        BatchRequest::new(self)
    }

    pub fn make_request<'a, Params: RpcParam>(
        &self,
        method: &'static str,
        params: Cow<'a, Params>,
    ) -> JsonRpcRequest<Cow<'a, Params>> {
        JsonRpcRequest {
            method,
            params,
            id: self.next_id(),
        }
    }

    pub fn prepare<'a, Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: &'static str,
        params: Cow<'a, Params>,
    ) -> RpcCall<T, Cow<'a, Params>, Resp> {
        let request = self.make_request(method, params);
        RpcCall::new(request, self.transport.clone())
    }

    /// Type erase the transport, allowing it to be used in a generic context.
    #[inline]
    pub fn boxed_service(
        self,
    ) -> RpcClient<BoxCloneService<Box<RawValue>, Box<RawValue>, TransportError>> {
        RpcClient {
            transport: BoxCloneService::new(self.transport),
            is_local: self.is_local,
            id: self.id,
        }
    }
}

pub struct ClientBuilder<L> {
    builder: ServiceBuilder<L>,
}

impl<L> ClientBuilder<L> {
    pub fn layer<M>(self, layer: M) -> ClientBuilder<Stack<M, L>> {
        ClientBuilder {
            builder: self.builder.layer(layer),
        }
    }

    pub fn transport<T>(self, transport: T, is_local: bool) -> RpcClient<L::Service>
    where
        L: Layer<T>,
        T: Transport,
        L::Service: Transport,
        <L::Service as tower::Service<Box<RawValue>>>::Future: Send,
    {
        RpcClient::new(self.builder.service(transport), is_local)
    }
}

#[cfg(test)]
mod test {
    use crate::transports::Http;

    use super::RpcClient;

    #[test]
    fn basic_instantiation() {
        let h: RpcClient<Http<reqwest::Client>> = "http://localhost:8545".parse().unwrap();

        assert!(h.is_local());
    }
}
