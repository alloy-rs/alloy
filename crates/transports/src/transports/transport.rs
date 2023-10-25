use serde_json::value::RawValue;
use tower::Service;

use crate::{BoxTransport, TransportError, TransportFut};

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
        Box<RawValue>,
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
        BoxTransport::new(self)
    }
}

impl<T> Transport for T where
    T: private::Sealed
        + Service<
            Box<RawValue>,
            Response = Box<RawValue>,
            Error = TransportError,
            Future = TransportFut<'static>,
        > + Send
        + Sync
        + 'static
{
}

mod private {
    use super::*;

    pub trait Sealed {}
    impl<T> Sealed for T where
        T: Service<
                Box<RawValue>,
                Response = Box<RawValue>,
                Error = TransportError,
                Future = TransportFut<'static>,
            > + Send
            + Sync
            + 'static
    {
    }
}
