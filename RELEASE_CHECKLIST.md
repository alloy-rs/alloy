# Release checklist

This checklist is meant to be used as a guide for the `crates.io` release process.

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
- [ ] (optional) Dry run `cargo-release`: `cargo release <version>`
- [ ] Run `cargo-semver-checks` for a non-breaking release: `cargo +stable semver-checks`
  - [ ] Breaking changes are not a blocker even for non-breaking release, but you must review them carefully in case of an accidental breaking change.
- [ ] Run `cargo-release`: `PUBLISH_GRACE_SLEEP=10 cargo release --execute [--no-verify] <version>`
  - Ignore these warnings:
    - `warning: updating <crate> to <version> despite no changes made since...`
    - `git-cliff` warning `there is already a tag (<tag>) for ...`
  - [ ] If a failure happened:
    - [ ] You should have an unpushed commit. After the issue is fixed, retry the release process with `--no-push` and squash the commits together.
    - [ ] If some crates were published before the error, AFAICT you must manually `--exclude <crate>` each already-published crate.
    - [ ] Verify that the commit is correct, and push to the repository with `git push --tags`.
- [ ] Create a new GitHub release with the automatically generated changelog and with the name set to `<repo> v<X.Y.Z>`
- [ ] Update version in `alloy` meta crate [README.md](./crates/alloy/README.md#installation) to the new version.
- [ ] Update version in [alloy-docs top navbar](https://github.com/alloy-rs/docs/blob/main/vocs/vocs.config.tsx#L58)
