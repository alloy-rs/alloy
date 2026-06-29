# alloy-eip5792

Types for the Wallet Call API.

- `wallet_getCapabilities` based on [EIP-5792][eip-5792], with the only capability being
  `delegation`.
- `wallet_sendTransaction` that can perform sequencer-sponsored [EIP-7702][eip-7702] delegations
  and send other sequencer-sponsored transactions on behalf of EOAs with delegated code.

[eip-5792]: https://eips.ethereum.org/EIPS/eip-5792
[eip-7702]: https://eips.ethereum.org/EIPS/eip-7702
