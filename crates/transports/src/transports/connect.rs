use crate::{BoxTransport, RpcClient, Transport};

/// Connection details for a transport. This ob
pub trait TransportConnect {
    /// The transport type that is returned by `connect`.
    type Transport: Transport;

    /// Returns true if the transport is a local transport.
    fn is_local(&self) -> bool {
        false
    }

    /// Connect to the transport, returning a `Transport` instance.
    fn connect(&self) -> Self::Transport;

    /// Connect to the transport, wrapping it into a `RpcClient` instance.
    fn client(&self) -> RpcClient<Self::Transport> {
        let is_local = self.is_local();
        RpcClient::new(self.connect(), is_local)
    }
}

/// Connection details for a transport that can be boxed.
///
/// This trait is implemented for [`TransportConnect`] implementors that
/// produce a boxable transport. It can be used to create a boxed transport
/// without knowing the exact type of the transport.
///
/// This trait is object safe. It is intended to allow creation of several
/// unlike transports or clients at once. E.g. `Vec<&dyn BoxTransportConnect>.
/// into_iter().map(|t| t.connect_boxed())`.
pub trait BoxTransportConnect {
    /// Connect to a transport, and box it.
    fn connect_boxed(&self) -> BoxTransport;

    /// Connect to a transport, and box it, wrapping it into a `RpcClient`.
    fn client_boxed(&self) -> RpcClient<BoxTransport>;
}

impl<T> BoxTransportConnect for T
where
    T: TransportConnect,
    T::Transport: Clone,
{
    fn connect_boxed(&self) -> BoxTransport {
        self.connect().boxed()
    }

    fn client_boxed(&self) -> RpcClient<BoxTransport> {
        let is_local = self.is_local();
        RpcClient::new(self.connect_boxed(), is_local)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn __compile_check(_: Box<dyn BoxTransportConnect>) {
        todo!()
    }
}
