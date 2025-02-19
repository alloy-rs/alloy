#!/usr/bin/env bash
set -eo pipefail

target=riscv32imac-unknown-none-elf
crates=(
    alloy-eips
    alloy-genesis
    alloy-serde
    alloy-consensus
    alloy-network-primitives
    alloy-rpc-types-eth
    alloy-rpc-types-engine
)

cmd=(cargo +stable hack check --no-default-features --target "$target")
for crate in "${crates[@]}"; do
    cmd+=(-p "$crate")
done

echo "Running: ${cmd[*]}"
"${cmd[@]}"
