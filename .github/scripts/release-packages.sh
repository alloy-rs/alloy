#!/usr/bin/env bash
set -euo pipefail

# Lists the workspace crates published to crates.io for a release tag such as
# `v2.5.0` and checks that every crate's manifest version matches the tag.
# Prints `packages=...` for `$GITHUB_OUTPUT`.

TAG="${1:?usage: release-packages.sh <tag>}"
VERSION="${TAG#v}"

if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    echo "error: '${TAG}' is not a release tag like v1.2.3" >&2
    exit 1
fi

metadata="$(cargo metadata --format-version 1 --no-deps --locked)"
# `publish` is null when unrestricted, or a list of allowed registries.
packages="$(jq -r '.packages[] | select(.publish == null or (.publish | index("crates-io"))) | .name' <<<"$metadata")"
if [[ -z "$packages" ]]; then
    echo "error: no publishable packages in the workspace" >&2
    exit 1
fi

for package in $packages; do
    actual="$(jq -r --arg name "$package" '.packages[] | select(.name == $name) | .version' <<<"$metadata")"
    if [[ "$actual" != "$VERSION" ]]; then
        echo "error: ${package} is ${actual}, but tag ${TAG} releases ${VERSION}" >&2
        exit 1
    fi
done

echo "packages=$(paste -sd ' ' - <<<"$packages")"
