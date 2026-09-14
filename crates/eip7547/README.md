# alloy-eip7547

Experimental types for the historical Engine API draft linked below.

These types do not match the current, stagnant [EIP-7547] text. Notably, this draft uses a
`2**22` inclusion-list gas limit, `{ address, nonce }` summary entries, and a `parentHash`; the
published EIP uses `2**21`, `{ address, gas_limit }`, and no `parent_hash`. Use these types only
with implementations of the [historical Engine API draft].

[EIP-7547]: https://eips.ethereum.org/EIPS/eip-7547
[historical Engine API draft]: https://github.com/michaelneuder/execution-apis/pull/1
