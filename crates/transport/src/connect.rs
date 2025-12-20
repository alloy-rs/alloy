use crate::{BoxTransport, TransportError};
use futures_utils_wasm::impl_future;

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
/// - You have implemented a custom [`Transport`](crate::Transport).
/// - You require a specific websocket reconnection strategy.
#[auto_impl::auto_impl(&, &mut, Box, Arc)]
pub trait TransportConnect: Sized + Send + Sync + 'static {
    /// Returns `true` if the transport connects to a local resource.
    ///
    /// This is a best-effort heuristic used to optimize behavior for local vs remote
    /// endpoints (e.g., setting different poll intervals).
    ///
    /// # Examples
    ///
    /// Local resources typically include:
    /// - `localhost` or `127.0.0.1` (IPv4 loopback)
    /// - `::1` (IPv6 loopback)
    /// - IPC paths (Unix sockets, Windows named pipes)
    /// - URLs without a hostname
    ///
    /// # Implementation
    ///
    /// For HTTP/WebSocket transports, consider using
    /// [`guess_local_url`](crate::utils::guess_local_url) to implement this method. IPC
    /// transports should always return `true`.
    fn is_local(&self) -> bool;

    /// Connect to the transport, returning a `Transport` instance.
    fn get_transport(&self) -> impl_future!(<Output = Result<BoxTransport, TransportError>>);
}
