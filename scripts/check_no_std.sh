#!/usr/bin/env bash
set -eo pipefail

no_std_packages=(
    alloy-eips
    alloy-genesis
    alloy-serde
)

for package in "${no_std_packages[@]}"; do
  cmd="cargo +stable build -p $package --target riscv32imac-unknown-none-elf --no-default-features"
  [ -n "$CI" ] && echo "::group::$cmd"
  $cmd
  [ -n "$CI" ] && echo "::endgroup::"
done
