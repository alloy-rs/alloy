# alloy-tx-macros

Derive support for composing EIP-2718 transaction-envelope enums. This crate is normally consumed
through [`alloy-consensus`](https://docs.rs/alloy-consensus), which re-exports
`TransactionEnvelope` and forwards its `serde` and `arbitrary` features.

```rust,ignore
use alloy_consensus::{Signed, TransactionEnvelope, TxEip1559};

#[derive(Clone, Debug, TransactionEnvelope)]
#[envelope(tx_type_name = MyTxType, typed = MyTypedTransaction)]
pub enum MyEnvelope {
    #[envelope(ty = 2)]
    Eip1559(Signed<TxEip1559>),
}
```

The numeric attribute is a dispatch declaration, not an encoding override: the inner type's
`Typed2718`, encoder, and decoder must already use the same ID. Valid EIP-2718 type bytes are at
most `0x7f`; although the macro and generated RLP codec accept any `u8`, larger IDs are invalid and
not interoperable with standard raw EIP-2718 decoding.

When the derive is imported through the `alloy` meta crate rather than a direct
`alloy-consensus` dependency, point generated paths at the re-export:

```rust,ignore
#[derive(Clone, Debug, alloy::consensus::TransactionEnvelope)]
#[envelope(alloy_consensus = alloy::consensus, tx_type_name = MyTxType)]
enum MyEnvelope {
    // ...
}
```
