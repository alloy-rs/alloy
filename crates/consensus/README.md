# alloy-consensus

Ethereum consensus interface.

This crate contains constants, types, and functions for implementing Ethereum
EL consensus and communication. This includes headers, blocks, transactions,
eip2718 envelopes, eip2930, eip4844, and more. The types in this crate
implement many of the traits found in [alloy_network].

In general a type belongs in this crate if it is committed to in the EL block
header. This includes:

- transactions
- blocks
- headers
- receipts
- [EIP-2718] envelopes.

[alloy-network]: ../network
[EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718

## Provenance

Much of this code was ported from [reth-primitives] as part of ongoing alloy
migrations.

[reth-primitives]: https://github.com/paradigmxyz/reth/tree/main/crates/primitives
