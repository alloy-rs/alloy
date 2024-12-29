use crate::{Transport, TransportError, TransportFut};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use std::fmt;
use tower::Service;

#[allow(unnameable_types)]
mod private {
    pub trait Sealed {}
    impl<T: super::Transport + Clone> Sealed for T {}
}

/// Trait for converting a transport into a boxed transport.
///
/// This trait is sealed and implemented for all types that implement
/// [`Transport`] + [`Clone`].
pub trait IntoBoxTransport: Transport + Clone + private::Sealed {
    /// Boxes the transport.
    fn into_box_transport(self) -> BoxTransport;
}

impl<T: Transport + Clone> IntoBoxTransport for T {
    fn into_box_transport(self) -> BoxTransport {
        BoxTransport { inner: Box::new(self) }
    }
}

/// A boxed, Clone-able [`Transport`] trait object.
///
/// This type allows RPC clients to use a type-erased transport. It is
/// [`Clone`] and [`Send`] + [`Sync`], and implements [`Transport`]. This
/// allows for complex behavior abstracting across several different clients
/// with different transport types.
///
/// All higher-level types, such as [`RpcClient`], use this type internally
/// rather than a generic [`Transport`] parameter.
///
/// [`RpcClient`]: crate::client::RpcClient
pub struct BoxTransport {
    inner: Box<dyn CloneTransport>,
}

impl BoxTransport {
    /// Instantiate a new box transport from a suitable transport.
    #[inline]
    pub fn new<T: IntoBoxTransport>(transport: T) -> Self {
        transport.into_box_transport()
    }

    /// Returns a reference to the inner transport.
    #[inline]
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
    #[inline]
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync> {
        Box::new(self.clone())
    }

    #[inline]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Service<RequestPacket> for BoxTransport {
    type Response = ResponsePacket;

    type Error = TransportError;

    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.inner.call(req)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // checks trait + send + sync + 'static
    fn _compile_check() {
        const fn inner<T>()
        where
            T: Transport + CloneTransport + Send + Sync + Clone + IntoBoxTransport + 'static,
        {
        }
        inner::<BoxTransport>();
    }
}
