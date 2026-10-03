# Release checklist

This checklist is meant to be used as a guide for the `crates.io` release process.

Releases are always made in lockstep, meaning that all crates in the repository
are released with the same version number, regardless of whether they have
changed or not.

## Requirements

- [cargo-release](https://github.com/crate-ci/cargo-release): `cargo install cargo-release --version 1.1.6 --locked`
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks): `cargo install cargo-semver-checks`
- [git-cliff](https://github.com/orhun/git-cliff) (patched with [orhun/git-cliff#711](https://github.com/orhun/git-cliff/pull/711)): `cargo install --git https://github.com/DaniPopes/git-cliff.git --branch fix-include-paths git-cliff`

## One-time CI publishing setup

- Configure each published crate's crates.io Trusted Publishing settings with owner
  `alloy-rs`, repository `alloy`, workflow `release.yml`, and environment `release`.
- Enable **Require trusted publishing for all new versions** for each crate, then
  revoke obsolete publishing tokens and remove their stored copies.
- Configure the GitHub `release` environment with required reviewers and deployment
  rules allowing release tags (`v*`). Restrict creation, updates, and deletion of
  those tags to release maintainers using a repository ruleset.
- Verify that `secure-runner` authentication succeeds for tag workflows and the
  `release` environment before the first production release.

## Steps

- [ ] Update the version number in the [README](./README.md#installation) to the new version.
- [ ] Make sure you're on the `main` branch.
- [ ] (optional) Dry run `cargo-release`: `cargo release <version>`
- [ ] Run `cargo-semver-checks` for a non-breaking release: `cargo +stable semver-checks`
  - [ ] Breaking changes are not a blocker even for non-breaking release, but you must review them carefully in case of an accidental breaking change.
- [ ] Prepare the release locally without publishing or pushing:
  `cargo release --execute --no-publish --no-push <version>`.
  Include the updated `Cargo.lock` in the signed release commit.
  - Ignore these warnings:
    - `warning: updating <crate> to <version> despite no changes made since...`
    - `git-cliff` warning `there is already a tag (<tag>) for ...`
- [ ] Push the release commit through the normal review/merge process. Ensure the
  signed local tag points at the merged commit; recreate the still-unpushed tag if
  merging changed its SHA. Wait for **main CI on that exact commit** to pass, then
  push only its release tag: `git push origin v<version>`.
- [ ] Review and approve the `release` environment deployment. CI verifies the tag
  against crate versions, checks main ancestry, and performs a
  locked workspace publish dry run before obtaining a temporary publishing token.
  Actual publishing uses `cargo release publish` with Cargo's package verification;
  unlike the dry run, cargo-release does not enforce `--locked`.
- [ ] If publishing fails, inspect the log and rerun failed jobs for the same tag.
  `cargo release publish` skips versions already published, allowing a partial
  release to resume. Do not move the tag or change already-published crate versions;
  source changes require a new release version. If all versions were already
  uploaded before the failure, verify them on crates.io rather than republishing.
- [ ] Create a new GitHub release with the automatically generated changelog and with the name set to `<repo> v<X.Y.Z>`
- [ ] Update version in `alloy` meta crate [README.md](./crates/alloy/README.md#installation) to the new version.
- [ ] Update version in [alloy-docs top navbar](https://github.com/alloy-rs/docs/blob/main/vocs/vocs.config.tsx#L58)
