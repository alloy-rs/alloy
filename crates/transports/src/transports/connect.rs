use crate::{BoxTransport, RpcClient, Transport, TransportError};

/// Connection details for a transport. This object captures the details
/// necessary to establish a simple transport.
///
/// ### Note on Fallibity
///

pub trait TransportConnect {
    /// The transport type that is returned by `connect`.
    type Transport: Transport;

    /// Returns `true`` if the transport is a local transport.
    fn is_local(&self) -> bool {
        false
    }

    /// Connect to the transport, returning a `Transport` instance.
    fn to_transport(&self) -> Result<Self::Transport, TransportError>;

    /// Connect to the transport, wrapping it into a `RpcClient` instance.
    fn connect(&self) -> Result<RpcClient<Self::Transport>, TransportError> {
        self.to_transport()
            .map(|t| RpcClient::new(t, self.is_local()))
    }
}

/// Connection details for a transport that can be boxed.
///
/// This trait is implemented for [`TransportConnect`] implementors that
/// produce a boxable transport. It can be used to create a boxed transport
/// without knowing the exact type of the transport.
///
/// This trait separate from TransportConnect to hide the associated type in
/// boxed instances. It is intended to allow creation of several unlike
/// transports or clients at once. E.g.
/// `Vec<&dyn BoxTransportConnect>.into_iter().map(|t| t.connect_boxed())`.
pub trait BoxTransportConnect {
    /// Returns `true`` if the transport is a local transport.
    fn is_local(&self) -> bool;

    /// Connect to a transport, and box it.
    fn to_boxed_transport(&self) -> Result<BoxTransport, TransportError>;

    /// Connect to a transport, and box it, wrapping it into a `RpcClient`.
    fn connect_boxed(&self) -> Result<RpcClient<BoxTransport>, TransportError>;
}

impl<T> BoxTransportConnect for T
where
    T: TransportConnect,
    T::Transport: Clone,
{
    fn is_local(&self) -> bool {
        TransportConnect::is_local(self)
    }

    fn to_boxed_transport(&self) -> Result<BoxTransport, TransportError> {
        self.to_transport().map(Transport::boxed)
    }

    fn connect_boxed(&self) -> Result<RpcClient<BoxTransport>, TransportError> {
        self.to_boxed_transport()
            .map(|boxed| RpcClient::new(boxed, self.is_local()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn __compile_check(_: Box<dyn BoxTransportConnect>) {
        todo!()
    }
}
