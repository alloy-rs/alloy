# alloy-network-primitives

Response traits and shared types for Alloy's network abstraction.

The response traits (`BlockResponse`, `HeaderResponse`, `TransactionResponse`, and
`ReceiptResponse`) let generic code inspect network-specific RPC values. `BlockTransactions`
represents full transactions, hashes only, or the omitted transaction field used by uncle
responses. Because its JSON representation is untagged, an empty `[]` deserializes as `Full([])`
and cannot preserve whether a hashes-only request produced it. Builder capability traits describe
optional transaction fields without choosing a concrete network.

```rust
use alloy_network_primitives::BlockResponse;
use alloy_primitives::TxHash;

fn transaction_hashes<B: BlockResponse>(block: &B) -> Vec<TxHash> {
    block.transactions().hashes().collect()
}
```
