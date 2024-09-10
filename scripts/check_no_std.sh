#!/usr/bin/env bash
set -eo pipefail

no_std_packages=(
    alloy-eips
    alloy-genesis
    alloy-serde
    alloy-consensus
    alloy-network-primitives
    alloy-rpc-types-eth
)

for package in "${no_std_packages[@]}"; do
  cmd="cargo +stable build -p $package --target riscv32imac-unknown-none-elf --no-default-features"
  if [ -n "$CI" ]; then
    echo "::group::$cmd"
  else
    printf "\n%s:\n  %s\n" "$package" "$cmd"
  fi

  $cmd

  if [ -n "$CI" ]; then
    echo "::endgroup::"
  fi
done
