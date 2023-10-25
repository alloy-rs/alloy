use serde_json::value::RawValue;
use std::fmt::Debug;
use tower::Service;

use crate::{Transport, TransportError, TransportFut};

/// A boxed, Clone-able [`Transport`] trait object.
///
/// This type allows [`RpcClient`] to use a type-erased transport. It is
/// [`Clone`] and [`Send`] + [`Sync`], and implementes [`Transport`]. This
/// allows for complex behavior abstracting across several different clients
/// with different transport types.
///
/// Most higher-level types will be generic over `T: Transport = BoxTransport`.
/// This allows paramterization with a concrete type, while hiding this
/// complexity from the library consumer.
///
/// [`RpcClient`]: crate::client::RpcClient
#[repr(transparent)]
pub struct BoxTransport {
    inner: Box<dyn CloneTransport + Send + Sync>,
}

impl BoxTransport {
    /// Instantiate a new box transport from a suitable transport.
    pub fn new<T>(inner: T) -> Self
    where
        T: Transport + Clone + Send + Sync,
    {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl Debug for BoxTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxTransport").finish()
    }
}

impl Clone for BoxTransport {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

/// Helper trait for constructing [`BoxTransport`].
trait CloneTransport: Transport {
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync>;
}

impl<T> CloneTransport for T
where
    T: Transport + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Service<Box<RawValue>> for BoxTransport {
    type Response = Box<RawValue>;

    type Error = TransportError;

    type Future = TransportFut<'static>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        self.inner.call(req)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    /// checks trait + send + sync + 'static
    fn __compile_check() {
        fn inner<T: CloneTransport>(_: Option<T>) {
            todo!()
        }
        fn inner_2<T: Transport>(_: Option<T>) {
            todo!()
        }
        inner::<BoxTransport>(None);
        inner::<BoxTransport>(None);
    }
}
