# alloy-next

### Layout

- alloy-json-rpc
  - Core data types for JSON-RPC 2.0
- alloy-transports
  - Transports and RPC call futures.
- alloy-networks
  - Network abstraction for RPC types. Allows capturing different RPC param and response types on a per-network basis.
- alloy-provider
  - Based on ethers::middleware::Middleware, but abstract over <N>, and object-safe.
