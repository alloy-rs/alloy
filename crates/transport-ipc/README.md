# alloy-transport-ipc

IPC transport implementation.

The backend keeps a long-lived connection (unix socket or Windows named
pipe) so subscriptions can share a mux with request/response traffic.

JSON-RPC values that arrive complete in one read are deserialized with
`serde_json` directly. A partial value flips the reader onto an incremental
framer (`memchr` over strings and structural bytes) so a large response is
scanned once, not re-parsed from the start of the buffer on every socket
read.

Existing constructors (`IpcConnect::new`, `ProviderBuilder::connect_ipc`,
`ClientBuilder::ipc`) need no extra configuration.
