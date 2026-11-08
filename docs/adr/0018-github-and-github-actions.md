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
   `nix develop --command cargo xtask ci` as its only check, so a local run and a CI run
   are identical. A new check goes into `xtask`, never into the workflow.
3. **It runs on Linux and macOS** (`ubuntu-latest`, `macos-latest`) for every pull
   request and every push to `main`. Both must pass.
4. **Third-party actions are few and pinned to a commit SHA**, with the version in a
   comment. A tag can be moved by its owner; a commit cannot. Dependabot proposes
   updates for the actions only. Cargo dependencies stay manual (ADR 0009).
5. **The workflow has read-only permissions** and does not keep the checkout token.
   The Cargo registry, `target/` and the Nix store are cached, keyed on `flake.lock`,
   `rust-toolchain.toml` and `Cargo.lock` (`flake.nix` for the store). A cache never
   changes a result, only the time. CI builds without incremental data and with line
   tables only (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`),
   because both made `target/` several times larger and slowed the restore on macOS.
6. **The Nix store is cached with `nix-community/cache-nix-action`**, per OS and
   architecture, with garbage collection down to 4 GB before a save. LibRaw and
   libgphoto2 are overridden in `flake.nix`, so the public cache never has them and
   every run would build them. The shell is built in its own step and kept as a GC
   root, so a restored store holds it. No account or secret is needed.

## Alternatives

- **Codeberg or a self-hosted Forgejo with its own runners:** independent of a large
  vendor, but no hosted macOS runners.
- **A separate workflow step per check:** nicer to read in the web view, but CI and
  `cargo xtask ci` would drift apart.
- **A Nix binary cache service (Cachix, FlakeHub):** faster cold starts, but an extra
  account and secret. The public NixOS cache is enough for now.

## Consequences

- A ruleset protects `main`: changes arrive only through pull requests, both
  `xtask ci` jobs must pass on the latest base, history stays linear (squash or rebase
  merge), and force pushes and deletion are blocked. The owner may bypass it only
  when merging a pull request, so even the owner's token cannot push to `main`
  directly.
- A macOS-only failure is a real bug, like any red CI (ADR 0006).
- A cold run (new `flake.lock`) still builds the shell, which takes a few minutes.
  Later runs restore it from the Actions cache.
