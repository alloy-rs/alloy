# Release checklist

This checklist is meant to be used as a guide for the `crates.io` release process.

`cargo-release` bumps the versions, commits, tags and pushes; pushing the `v<X.Y.Z>` tag runs the
[release workflow](./.github/workflows/release.yml), which publishes every crate to `crates.io`
with trusted publishing and creates the GitHub release.

Releases are always made in lockstep, meaning that all crates in the repository
are released with the same version number, regardless of whether they have
changed or not.

## Requirements

- [cargo-release](https://github.com/crate-ci/cargo-release): `cargo install cargo-release`
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks): `cargo install cargo-semver-checks`
- [git-cliff](https://github.com/orhun/git-cliff) (patched with [orhun/git-cliff#711](https://github.com/orhun/git-cliff/pull/711)): `cargo install --git https://github.com/DaniPopes/git-cliff.git --branch fix-include-paths git-cliff`

## Steps

- [ ] Update the version number in the [README](./README.md#installation) to the new version.
- [ ] Make sure you're on the `main` branch.
- [ ] Make sure every crate is already on crates.io. Trusted publishing can't create a crate, so the release workflow stops if one is missing. For a new crate, a crates.io owner publishes a placeholder `0.0.0` by hand and adds this repository's `release.yml` (environment `release`) as its trusted publisher before you tag.
- [ ] (optional) Dry run `cargo-release`: `cargo release <version>`
- [ ] Check that every crate packages and builds: `cargo publish --workspace --dry-run --locked`. With `publish = false`, `cargo release` no longer does this, and CI only checks it after the tag is pushed.
- [ ] Run `cargo-semver-checks` for a non-breaking release: `cargo +stable semver-checks`
  - [ ] Breaking changes are not a blocker even for non-breaking release, but you must review them carefully in case of an accidental breaking change.
- [ ] Run `cargo-release`: `cargo release --execute <version>`
  - Ignore these warnings:
    - `warning: updating <crate> to <version> despite no changes made since...`
    - `git-cliff` warning `there is already a tag (<tag>) for ...`
  - [ ] If a failure happened before pushing, you should have an unpushed commit. After the issue is fixed, retry the release process with `--no-push` and squash the commits together, then push the commit and the tag together with `git push --atomic origin main v<version>`.
- [ ] Approve the `publish` job of the [release workflow](./.github/workflows/release.yml) run for the tag.
  - [ ] If publishing stopped partway, re-run the failed job: crates already on `crates.io` are skipped.
- [ ] Check the GitHub release the workflow created.
- [ ] Update version in `alloy` meta crate [README.md](./crates/alloy/README.md#installation) to the new version.
- [ ] Update version in [alloy-docs top navbar](https://github.com/alloy-rs/docs/blob/main/vocs/vocs.config.tsx#L58)
