use crate::{Transport, TransportError, TransportFut};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use std::fmt;
use tower::Service;

/// A boxed, Clone-able [`Transport`] trait object.
///
/// This type allows RPC clients to use a type-erased transport. It is
/// [`Clone`] and [`Send`] + [`Sync`], and implements [`Transport`]. This
/// allows for complex behavior abstracting across several different clients
/// with different transport types.
///
/// Most higher-level types will be generic over `T: Transport = BoxTransport`.
/// This allows parameterization with a concrete type, while hiding this
/// complexity from the library consumer.
///
/// [`RpcClient`]: crate::client::RpcClient
#[repr(transparent)]
pub struct BoxTransport {
    inner: Box<dyn CloneTransport + Send + Sync>,
}

impl BoxTransport {
    /// Instantiate a new box transport from a suitable transport.
    pub fn new<T: Transport + Clone + Send + Sync>(inner: T) -> Self {
        Self { inner: Box::new(inner) }
    }

    /// Returns a reference to the inner transport.
    pub fn as_any(&self) -> &dyn std::any::Any {
        self.inner.as_any()
    }
}

impl fmt::Debug for BoxTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxTransport").finish_non_exhaustive()
    }
}

impl Clone for BoxTransport {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone_box() }
    }
}

/// Helper trait for constructing [`BoxTransport`].
trait CloneTransport: Transport + std::any::Any {
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync>;
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T> CloneTransport for T
where
    T: Transport + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Service<RequestPacket> for BoxTransport {
    type Response = ResponsePacket;

    type Error = TransportError;

    type Future = TransportFut<'static>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.inner.call(req)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // checks trait + send + sync + 'static
    fn __compile_check() {
        fn inner<T: CloneTransport>(_: Option<T>) {}
        fn inner_2<T: Transport>(_: Option<T>) {}
        inner::<BoxTransport>(None);
        inner::<BoxTransport>(None);
    }
}
