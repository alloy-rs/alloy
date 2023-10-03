use alloy_json_rpc::{Id, Request, RpcParam};
use serde_json::value::RawValue;
use std::{borrow::Cow, fmt::Debug};
use tower::Service;

use crate::{utils, TransportError, TransportFut};

/// A single JSON-RPC request, serialized, and accompanied by its ID and method.
#[derive(Debug, Clone)]
pub struct SingleRequest {
    pub id: Id,
    pub method: &'static str,
    pub serialized: Box<RawValue>,
}

/// A Request to be sent by the transport
#[derive(Debug, Clone)]
pub enum TransportRequest {
    Single(SingleRequest),
    Batch(Vec<SingleRequest>),
}

impl From<Vec<SingleRequest>> for TransportRequest {
    fn from(v: Vec<SingleRequest>) -> Self {
        Self::Batch(v)
    }
}

impl<T: RpcParam> TryFrom<&Request<T>> for SingleRequest {
    type Error = TransportError;

    fn try_from(req: &Request<T>) -> Result<Self, Self::Error> {
        let id = req.id.clone();
        let method = req.method;
        let serialized = utils::to_json_raw_value(&req)?;

        Ok(Self {
            id,
            method,
            serialized,
        })
    }
}

impl FromIterator<SingleRequest> for TransportRequest {
    fn from_iter<T: IntoIterator<Item = SingleRequest>>(iter: T) -> Self {
        Self::Batch(iter.into_iter().collect())
    }
}

impl<T: RpcParam> TryFrom<Request<T>> for SingleRequest {
    type Error = TransportError;

    fn try_from(req: Request<T>) -> Result<Self, Self::Error> {
        Self::try_from(&req)
    }
}

impl<T: RpcParam> TryFrom<&Request<T>> for TransportRequest {
    type Error = TransportError;

    fn try_from(req: &Request<T>) -> Result<Self, Self::Error> {
        let single = SingleRequest::try_from(req)?;
        Ok(TransportRequest::Single(single))
    }
}

impl<T: RpcParam> TryFrom<Request<T>> for TransportRequest {
    type Error = TransportError;

    fn try_from(req: Request<T>) -> Result<Self, Self::Error> {
        let single = SingleRequest::try_from(&req)?;
        Ok(TransportRequest::Single(single))
    }
}

impl TransportRequest {
    pub fn serialized(&self) -> Result<Cow<str>, TransportError> {
        match self {
            TransportRequest::Single(req) => Ok(Cow::Borrowed(req.serialized.get())),
            TransportRequest::Batch(batch) => {
                let mut reqs = Vec::with_capacity(batch.len());
                for req in batch {
                    reqs.push(&req.serialized);
                }
                serde_json::to_string(&reqs)
                    .map(Cow::Owned)
                    .map_err(TransportError::ser_err)
            }
        }
    }
}

/// A marker trait for transports.
///
/// # Implementing `Transport`
///
/// This trait is blanket implemented for all appropriate types. To implement
/// this trait, you must implement the [`tower::Service`] trait with the
/// appropriate associated types. It cannot be implemented directly.
pub trait Transport:
    private::Sealed
    + Service<
        TransportRequest,
        Response = Box<RawValue>,
        Error = TransportError,
        Future = TransportFut<'static>,
    > + Send
    + Sync
    + 'static
{
    /// Convert this transport into a boxed trait object.
    fn boxed(self) -> BoxTransport
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        BoxTransport {
            inner: Box::new(self),
        }
    }
}

impl<T> Transport for T where
    T: private::Sealed
        + Service<
            TransportRequest,
            Response = Box<RawValue>,
            Error = TransportError,
            Future = TransportFut<'static>,
        > + Send
        + Sync
        + 'static
{
}

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

impl Service<TransportRequest> for BoxTransport {
    type Response = Box<RawValue>;

    type Error = TransportError;

    type Future = TransportFut<'static>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: TransportRequest) -> Self::Future {
        self.inner.call(req)
    }
}

/// checks trait + send + sync + 'static
fn __compile_check() {
    fn inner<T: CloneTransport>() {
        todo!()
    }
    inner::<BoxTransport>();
}

mod private {
    use super::*;

    pub trait Sealed {}
    impl<T> Sealed for T where
        T: Service<
                TransportRequest,
                Response = Box<RawValue>,
                Error = TransportError,
                Future = TransportFut<'static>,
            > + Send
            + Sync
            + 'static
    {
    }
}
