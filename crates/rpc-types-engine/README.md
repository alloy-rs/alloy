# alloy-rpc-types-engine

Types for the `engine` Ethereum JSON-RPC namespace.

Engine API types:
<https://github.com/ethereum/execution-apis/blob/main/src/engine/authentication.md>
and <https://eips.ethereum.org/EIPS/eip-3675>,
following the execution specs <https://github.com/ethereum/execution-apis/tree/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine>.

Non-standard Engine endpoints:
- `engine_getPayloadV1Hacked` .. `engine_getPayloadV4Hacked`
  - Params are encoded as `params: [["0x<signed tx>", ...]]`.
  - `engine_getPayloadV1Hacked` returns `ExecutionPayloadV1` (no envelope).
  - V2/V3/V4 return the same envelope shapes as their standard counterparts.
