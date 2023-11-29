use crate::{BoxTransport, Pbf, Transport, TransportError};

/// Connection details for a transport.
///
/// This object captures the information necessary to establish a transport,
/// and may encapsulate reconnection logic.
///
/// ## Why implement `TransportConnect`?
///
/// Users may want to implement transport-connect for the following reasons:
/// - You want to customize a `reqwest::Client` before using it.
/// - You need to provide special authentication information to a remote provider.
/// - You have implemented a custom [`Transport`].
/// - You require a specific websocket reconnection strategy.
pub trait TransportConnect: Sized + Send + Sync + 'static {
    /// The transport type that is returned by `connect`.
    type Transport: Transport + Clone;

    /// Returns `true` if the transport connects to a local resource.
    fn is_local(&self) -> bool;

    /// Connect to the transport, returning a `Transport` instance.
    fn get_transport<'a: 'b, 'b>(&'a self) -> Pbf<'b, Self::Transport, TransportError>;
}

/// Connection details for a transport that can be boxed.
///
/// This trait is implemented for [`TransportConnect`] implementors that
/// produce a boxable transport. It can be used to create a boxed transport
/// without knowing the exact type of the transport.
///
/// This trait is separate from `TransportConnect`` to hide the associated type
/// in when this trait is a trai object. It is intended to allow creation of
/// several unlike transports or clients at once. E.g.
/// in something like `Vec<&dyn BoxTransportConnect>.
pub trait BoxTransportConnect {
    /// Returns `true` if the transport is a local transport.
    fn is_local(&self) -> bool;

    /// Connect to a transport, and box it.
    fn get_boxed_transport<'a: 'b, 'b>(&'a self) -> Pbf<'b, BoxTransport, TransportError>;
}

impl<T> BoxTransportConnect for T
where
    T: TransportConnect,
{
    fn is_local(&self) -> bool {
        TransportConnect::is_local(self)
    }

    fn get_boxed_transport<'a: 'b, 'b>(&'a self) -> Pbf<'b, BoxTransport, TransportError> {
        Box::pin(async move { self.get_transport().await.map(Transport::boxed) })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn __compile_check(_: Box<dyn BoxTransportConnect>) {
        todo!()
    }
}
