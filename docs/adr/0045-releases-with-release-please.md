# 0045 – Releases with release-please

- Status: accepted
- Date: 2026-11-10

## Context

The owner asked for the first release, 0.1.0, and for a release process that needs no
hand-made steps. Every commit on `main` is already a Conventional Commit, because pull
requests are squash-merged with a Conventional Commit title (`docs/pull-requests.md`).
Until now `CHANGELOG.md` was written by hand in the Keep a Changelog format, and ADR 0038
§3 made a changed public API snapshot require a `CHANGELOG.md` change.

The workspace has one version, `workspace.package.version` in `Cargo.toml`; every member
inherits it, so `Cargo.lock` holds the same version for each member.

## Decision

### 1. release-please writes versions, tags and the changelog

The workflow `.github/workflows/release-please.yml` runs
`googleapis/release-please-action` v5, pinned to a commit (ADR 0018), on every push to
`main`. It keeps a release pull request open with the next version and the changelog
entry built from the commits since the last release. Merging that pull request tags
`v<version>` and creates the GitHub release. The token is the `RELEASE_PLEASE_TOKEN`
secret, so CI runs on the release pull request, or `GITHUB_TOKEN` without it.

### 2. One version for the workspace

`release-please-config.json` uses the `simple` release type for the repository root,
because the `rust` release type only handles a `[package]` manifest, not a virtual
workspace. Two TOML extra files carry the version: `$.workspace.package.version` in
`Cargo.toml`, and `$.package[?(!@.source)].version` in `Cargo.lock`, which matches
exactly the workspace members, since only path packages have no `source`. The lock file
stays consistent with `cargo --locked`. Before 1.0 a feature bumps the patch version and
a breaking change the minor version.

### 3. The changelog belongs to release-please

From 0.1.0 on, nobody edits `CHANGELOG.md` by hand. A user-visible change is a `feat`
or `fix` commit (or `perf`, `revert`, or breaking with `!`), and its subject becomes the
entry. This replaces ADR 0038 §3: a changed public API snapshot needs such a commit
since the base (the pull request title counts too), not a `CHANGELOG.md` change. CI
enforces it; a local run only warns, because the commit may not be written yet.

### 4. The first release is 0.1.0

`Cargo.toml` already says 0.1.0, and there is no tag. The config pins the release with
`release-as: 0.1.0`. The hand-written notes become the `[0.1.0]` section, and
`skip-changelog: true` keeps release-please from adding a second, generated 0.1.0
section above them. `bootstrap-sha`, the last commit before this decision, keeps the
generated release notes from listing the whole history. After the 0.1.0 release the
three settings are removed (`TODO.md`).

## Alternatives

- *One release-please package per crate with the `cargo-workspace` plugin*: 29 versions
  and tags for one program whose crates are never published. Rejected.
- *Keep the hand-written changelog*: every pull request touches the same section, which
  conflicts between parallel branches, and the release still needs a manual version bump
  and tag. Rejected.
- *Let release-please generate 0.1.0 from all commits*: duplicates the curated notes
  with over a hundred raw commit subjects. Rejected.

## Consequences

- The pull request title is the changelog entry, so it must say what changed for the
  user.
- Between this decision and the 0.1.0 release, user-visible changes appear only in the
  release pull request and the GitHub release, not in `CHANGELOG.md`.
- A release is merging the release pull request; `docs/pull-requests.md` §8 describes it.
