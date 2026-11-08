# 0038 – Public API snapshots with cargo-public-api on the stable toolchain

- Status: accepted
- Date: 2026-11-07

## Context

ADR 0004 makes the kernel contract the interface that must stay stable, and modules use
the services only through their public API. Architecture §7 asks for a
`cargo public-api` snapshot of `kernel` and the services, so that an API change is
never accidental: it must update the snapshot and add a changelog entry.

`cargo-public-api` reads the API from rustdoc's JSON output. That output is unstable,
so the tool builds it with a nightly toolchain through `rustup`. The project pins a
stable toolchain from Nix and has no `rustup` (ADR 0007); the only nightly lives in the
separate fuzz shell, which CI does not use for `cargo xtask ci` (ADR 0037 §3).

## Decision

### 1. One snapshot per crate, checked in `cargo xtask ci`

Every crate of the `kernel` and `service` layers has a `public-api.txt` next to its
`Cargo.toml`: the output of `cargo-public-api` for its library, one item per line,
sorted. Blanket impls (`impl<T> From<T> for T` and the like) are left out because they
hold for every type. Auto trait impls stay, because losing `Send` or `Sync` breaks
callers.

`cargo xtask check-public-api` renders the API of each crate and fails when it differs
from the snapshot, listing the added and removed lines. `cargo xtask public-api` writes
the snapshots. `cargo xtask ci` runs the check as the step "public API". New crates of
these layers are found by their `layer`, so a new service fails until its snapshot is
committed.

### 2. rustdoc JSON from the stable compiler with `RUSTC_BOOTSTRAP=1`

xtask builds the JSON itself with `cargo rustdoc --lib -- -Z unstable-options
--output-format json` and the variable `RUSTC_BOOTSTRAP=1`, which unlocks the unstable
flag on the pinned stable compiler. It then passes the file to `cargo-public-api
--rustdoc-json`, so the tool never calls `rustup`. The JSON goes to
`target/public-api/`, because Cargo treats `RUSTC_BOOTSTRAP` as part of the build
fingerprint and would otherwise rebuild `target/` on every run.

`RUSTC_BOOTSTRAP` is used only for this documentation build. No crate is compiled with
unstable features, and the product build never sees the variable.

`cargo-public-api` comes from nixpkgs in the default development shell. Each version
reads a range of rustdoc JSON format versions. Both the toolchain and nixpkgs are
pinned (`rust-toolchain.toml`, `flake.lock`), so the pair only changes in a PR; if a
toolchain update brings a format the tool cannot read, the same PR updates `flake.lock`.

### 3. A changed snapshot needs a `CHANGELOG.md` entry

When a `public-api.txt` differs from the merge base with the base branch (on `main`:
from the parent of `HEAD`), `CHANGELOG.md` must differ too. The base is chosen as for
the spec trailers (ADR 0010 §4); outside CI a missing base only warns. The check
compares with the working tree, so a local run sees uncommitted changes.

## Alternatives

- *A nightly toolchain in the default shell*: a second compiler for one check, and
  rustup's `+nightly` still does not exist in Nix. Rejected.
- *Run the check in the fuzz shell*: CI would need a second Nix shell and a second
  `target/` per run, and `cargo xtask ci` would no longer be one command. Rejected.
- *Render the API from rustdoc JSON in xtask*: printing types, generics and impls
  faithfully is a deep problem that `cargo-public-api` already solves. Rejected
  (ADR 0009).
- *`cargo-semver-checks`*: answers whether a release is semver-compatible against a
  published version; Framehive publishes no crates and wants every change visible, not
  only breaking ones.

## Consequences

- An API change of `kernel` or a service shows up in the PR diff as lines of
  `public-api.txt`, and reviewers see it next to the changelog entry.
- The check builds the documentation of nine crates, a few seconds with a warm cache.
- A toolchain update may need a matching `cargo-public-api` from nixpkgs.
