#!/usr/bin/env bash
set -eo pipefail

target=wasm32-unknown-unknown
crates=(
    alloy-eips
    alloy-genesis
    alloy-serde
    alloy-consensus
    alloy-network-primitives
    alloy-rpc-client
    alloy-rpc-types-eth
    alloy-rpc-types-engine
    alloy-transport
    alloy-transport-ws
)

cmd=(cargo +stable hack check --no-default-features --ignore-unknown-features --features ws --target "$target")
for crate in "${crates[@]}"; do
    cmd+=(-p "$crate")
done

echo "Running: ${cmd[*]}"
"${cmd[@]}"
