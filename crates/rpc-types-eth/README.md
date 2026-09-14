# alloy-rpc-types-eth

Types for the `eth` Ethereum JSON-RPC namespace.

Common entry points are `TransactionRequest` for calls and transaction construction, `Filter` and
`Log` for log queries, `Block` and `Header` for block responses, `TransactionReceipt` for execution
results, and `StateOverride` for temporary call state.

Transaction type inference is driven by populated fields, while event filters require the complete
canonical event signature:

```rust
use alloy_consensus::TxType;
use alloy_rpc_types_eth::{Filter, TransactionRequest};

let request = TransactionRequest::default();
assert_eq!(request.preferred_type(), TxType::Eip1559);

let filter = Filter::new().event("Transfer(address,address,uint256)");
assert!(filter.has_topics());
```
