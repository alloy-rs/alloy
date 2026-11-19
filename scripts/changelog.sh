#!/usr/bin/env bash
set -eo pipefail

run_unless_dry_run() {
    if [ "$DRY_RUN" = "true" ]; then
        echo "skipping due to dry run: $*" >&2
    else
        "$@"
    fi
}

root=$WORKSPACE_ROOT
crate=$CRATE_ROOT
crate_glob="${crate#"$root/"}/**"

if [[ "$crate" = */tests/* || "$crate" = *test-utils* ]]; then
    exit 0
fi

changelog="$crate/CHANGELOG.md"
if [ -f "$changelog" ] && grep -q '^## \[[0-9]' "$changelog"; then
    run_unless_dry_run git cliff --repository "$root" --config "$root/cliff.toml" --include-path "$crate_glob" --unreleased "${@}" --prepend "$changelog"
else
    run_unless_dry_run git cliff --repository "$root" --config "$root/cliff.toml" --include-path "$crate_glob" "${@}" --output "$changelog"
fi
