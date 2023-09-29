use std::fmt::Debug;

use alloy_json_rpc::Id;
use alloy_primitives::U256;
use serde_json::value::RawValue;
use tokio::sync::broadcast;
use tower::Service;

use crate::{Transport, TransportError, TransportFut};

/// A trait for transports supporting notifications.
///
/// This trait models notifications bodies as a stream of [`RawValue`]s. It is
/// up to the recipient to deserialize the notification.
pub trait PubSub: Transport {
    /// Reserve an ID for a subscription, based on the JSON-RPC request ID of
    /// the subscription request.
    ///
    /// This is intended for internal use by RPC clients, and should not be
    /// called directly.
    fn reserve_id(&self, id: &Id) -> U256;

    /// Get a [`broadcast::Receiver`] for the given subscription ID.
    fn get_watcher(&self, id: U256) -> broadcast::Receiver<Box<RawValue>>;
}

/// Helper trait for constructing [`BoxPubSub`].
trait ClonePubSub: PubSub {
    fn clone_box(&self) -> Box<dyn ClonePubSub + Send + Sync>;
}

impl<T> ClonePubSub for T
where
    T: PubSub + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn ClonePubSub + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A boxed, Clone-able [`PubSub`] trait object.
///
/// This type allows [`RpcClient`] to use a type-erased transport. It is
/// [`Clone`] and [`Send`] + [`Sync`], and implementes [`PubSub`]. This
/// allows for complex behavior abstracting across several different clients
/// with different transport types.
///
/// [`RpcClient`]: crate::client::RpcClient
#[repr(transparent)]
pub struct BoxPubSub {
    inner: Box<dyn ClonePubSub + Send + Sync>,
}

impl Debug for BoxPubSub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxPubSub").finish()
    }
}

impl Clone for BoxPubSub {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

impl PubSub for BoxPubSub {
    fn reserve_id(&self, id: &Id) -> U256 {
        self.inner.reserve_id(id)
    }

    fn get_watcher(&self, id: U256) -> broadcast::Receiver<Box<RawValue>> {
        self.inner.get_watcher(id)
    }
}

impl Service<Box<RawValue>> for BoxPubSub {
    type Response = Box<RawValue>;

    type Error = TransportError;

    type Future = TransportFut<'static>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Service::<Box<RawValue>>::poll_ready(&mut self.inner, cx)
    }

    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        Service::<Box<RawValue>>::call(&mut self.inner, req)
    }
}

/// checks trait + send + sync + 'static
fn __compile_check() {
    fn inner<T: ClonePubSub>() {
        unimplemented!()
    }
    inner::<BoxPubSub>();
}
