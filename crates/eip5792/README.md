# alloy-eip5792

Legacy sequencer-specific Wallet Call API types.

These types model an earlier draft and do **not** implement the Final [EIP-5792] `wallet_sendCalls`
schema. In particular, `SendCallsRequest` requires `from`, stores a chain ID per call, and has no
top-level `id`, `chainId`, or `atomicRequired`. `WalletCapabilities` models a custom
sequencer-sponsored [EIP-7702] delegation capability rather than EIP-5792's built-in `atomic`
capability. Use this crate only with APIs that expect this legacy shape.

[EIP-5792]: https://eips.ethereum.org/EIPS/eip-5792
[EIP-7702]: https://eips.ethereum.org/EIPS/eip-7702
