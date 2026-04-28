# alloy-transport-mpp

Machine Payments Protocol (MPP) transport for alloy.

This crate adds an opt-in WebSocket transport that speaks the
[MPP](https://github.com/tempoxyz/mpp-rs) wire protocol while remaining a
drop-in `PubSubConnect` implementation for the rest of alloy. JSON-RPC frames
are wrapped in MPP `message` envelopes; payment `challenge`, `needVoucher`,
`receipt` and `error` frames are handled internally via a user-supplied
[`PaymentProvider`](https://docs.rs/mpp/latest/mpp/client/trait.PaymentProvider.html).

```rust,ignore
use alloy_provider::ProviderBuilder;
use alloy_transport_mpp::MppWsConnect;

let connect = MppWsConnect::new("wss://paid.example/rpc", my_provider);
let provider = ProviderBuilder::new().connect_pubsub(connect).await?;
```
