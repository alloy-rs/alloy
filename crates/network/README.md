# alloy-network

Ethereum blockchain RPC behavior abstraction.

This crate contains a simple abstraction of the RPC behavior of an
Ethereum-like blockchain. It is intended to be used by the Alloy client to
provide a consistent interface to the rest of the library, regardless of
changes the underlying blockchain makes to the RPC interface.

## Core Model

This crate handles abstracting RPC types. It does not handle the actual
networking. The core model is as follows:

- `Transaction` - A trait that defines an abstract interface for EVM-like
  transactions.
- `Network` - A type-level mapping between a blockchain's consensus types and JSON-RPC request and
  response types. Providers use its associated types to define RPC inputs and outputs.
- `TransactionBuilder` - A trait for constructing and validating network-specific transaction requests. Used to build typed transactions for signing and submission. See [`TransactionBuilder`](https://docs.rs/alloy-network/latest/alloy_network/trait.TransactionBuilder.html).
- `NetworkWallet` - A trait for wallets that can sign transactions for a given network. Used to abstract over different signing backends. See [`NetworkWallet`](https://docs.rs/alloy-network/latest/alloy_network/trait.NetworkWallet.html).
- `BlockResponse`, `TransactionResponse`, `ReceiptResponse`, `HeaderResponse` - Traits (from `alloy-network-primitives`) that define the structure of block, transaction, receipt, and header types used in RPC responses. These are associated types in the `Network` trait and are implemented by network-specific types. See [`alloy-network-primitives`](https://docs.rs/alloy-network-primitives/).

## Usage

This crate is not intended to be used directly. It is used by the
[alloy-provider] library and reth to modify the input and output types of the
RPC methods.

This crate will primarily be used by blockchain maintainers to add bespoke RPC
types to the Alloy provider. This is done by implementing the `Network` trait,
and then parameterizing the `Provider` type with the new network type.

For example, to add a new network called `Foo`:

```rust,ignore
// A ZST is conventional because Network is a type-level marker, but it is not required.
#[derive(Clone, Copy, Debug)]
struct Foo;

impl Network for Foo {
    type TxType = FooTxType;
    type TxEnvelope = FooTxEnvelope;
    type UnsignedTx = FooUnsignedTx;
    type ReceiptEnvelope = FooReceiptEnvelope;
    type Header = FooHeader;
    type TransactionRequest = FooTransactionRequest;
    type TransactionResponse = FooTransactionResponse;
    type ReceiptResponse = FooReceiptResponse;
    type HeaderResponse = FooHeaderResponse;
    type BlockResponse = FooBlockResponse;
}
```

The user may then instantiate a `Provider<Foo>` and use it as normal. This
allows the user to use the same API for all networks, regardless of the
underlying RPC types.

If the network also needs custom RPC methods, define an extension trait with default method
implementations and add a blanket implementation for every matching provider:

```rust,ignore
use alloy_provider::Provider;
use alloy_transport::TransportResult;

trait FooProviderExt: Provider<Foo> {
    async fn custom_foo_method(&self) -> TransportResult<Something> {
        self.client().request("foo_customMethod", ()).await
    }
}

impl<P: Provider<Foo>> FooProviderExt for P {}
```

[alloy-provider]: https://docs.rs/alloy-provider/
