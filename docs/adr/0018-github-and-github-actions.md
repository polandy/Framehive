# 0018 – Hosting on GitHub, CI with GitHub Actions

- Status: accepted
- Date: 2026-10-18

## Context

The repository needs a public home, pull requests and a CI that blocks a merge when
`cargo xtask ci` fails (ADR 0007, ADR 0008). The CI must run on Linux and macOS, the
two primary platforms (ADR 0002). Agents work through the `gh` command line, and the
owner already uses GitHub.

## Decision

1. **The repository is hosted on GitHub** as `polandy/Framehive`, public. The test data
   and review screenshot repositories of ADR 0017 live next to it.
2. **CI is one GitHub Actions workflow**, `.github/workflows/ci.yml`. It runs
   `nix develop --command cargo xtask ci` and nothing else, so a local run and a CI run
   are identical. A new check goes into `xtask`, never into the workflow.
3. **It runs on Linux and macOS** (`ubuntu-latest`, `macos-latest`) for every pull
   request and every push to `main`. Both must pass.
4. **Third-party actions are few and pinned to a commit SHA**, with the version in a
   comment. A tag can be moved by its owner; a commit cannot. Dependabot proposes
   updates for the actions only. Cargo dependencies stay manual (ADR 0009).
5. **The workflow has read-only permissions** and does not keep the checkout token.
   Only the Cargo registry and `target/` are cached, keyed on `flake.lock`,
   `rust-toolchain.toml` and `Cargo.lock`. A cache never changes a result, only the time.

## Alternatives

- **Codeberg or a self-hosted Forgejo with its own runners:** independent of a large
  vendor, but no hosted macOS runners.
- **A separate workflow step per check:** nicer to read in the web view, but CI and
  `cargo xtask ci` would drift apart.
- **A Nix binary cache service (Cachix, FlakeHub):** faster cold starts, but an extra
  account and secret. The public NixOS cache is enough for now.

## Consequences

- A red check on either platform blocks the merge once branch protection requires the
  `xtask ci` jobs.
- A macOS-only failure is a real bug, like any red CI (ADR 0006).
- Cold runs download the Nix shell from the public cache, which takes a few minutes.
