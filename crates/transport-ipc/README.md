# alloy-transport-ipc

IPC transport implementation.

The backend keeps a long-lived connection (unix socket or Windows named
pipe) so subscriptions can share a mux with request/response traffic.

JSON-RPC values are framed incrementally: a small state machine finds the
end of each top-level object or array as bytes arrive, and each complete
frame is deserialized once. Partial responses are not re-parsed from the
start of the buffer on every socket read.

Existing constructors (`IpcConnect::new`, `ProviderBuilder::connect_ipc`,
`ClientBuilder::ipc`) need no extra configuration.
