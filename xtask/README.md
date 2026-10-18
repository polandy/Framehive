<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# xtask

## Purpose

Repository automation. It enforces the rules in `CLAUDE.md` mechanically, so quality
does not depend on human review. It uses only the standard library: it reads
`cargo metadata` output with its own small JSON reader and `dependencies.yaml` with a
strict reader for the subset of YAML that file uses. Both are replaced by the YAML
adapter once ADR 0011 is implemented.

## Commands

Run them as `cargo xtask <command>`, inside `nix develop`.

| Command | Checks |
|---|---|
| `ci` | Everything, in CI order: format, clippy, the four checks below, nextest, doc tests, `cargo deny`, `reuse lint`. Quiet: one `ok` line per step plus warnings from our own checks, and the full output of the failing step only |
| `check-files` | Every file Git tracks or would track is UTF-8 text without NUL bytes and at most 256 KiB (ADR 0017); Rust files have at most 400 lines; Markdown outside `docs/adr/` and `CHANGELOG.md` has at most 10 KiB (architecture §7) |
| `check-architecture` | Every crate declares a valid layer; dependencies follow architecture §3; adapters use only their port service; module UIs use only their module and `ui-kit`; crate roots contain `#![forbid(unsafe_code)]` unless `ffi = true`; every README has the required sections |
| `check-dependencies` | Every external dependency is listed in `dependencies.yaml`, used only by allowed layers and crates, with allowed features (`default` counts as a feature), and pinned in `[workspace.dependencies]`; no unused entries |
| `check-specs` | Spec headers, IDs, status and fields; origins exist in `docs/requirements/`; dependencies exist (a warning for drafts); unique criteria; no open questions in binding specs; every `/// spec: <ID>.<n>` points to an existing, non-draft criterion; every criterion of an implemented spec has a test |
| `ci-failures [<pr>]` | Not a check: prints only what failed in the CI run of a pull request (default: the pull request of the current branch), from the `FAIL` line of `cargo xtask ci` on, or the end of the log for a job that failed before. Needs `gh`, logged in |

## Crate metadata

Each crate declares its place in `Cargo.toml`:

```toml
[package.metadata.framehive]
layer = "adapter"        # domain, kernel, service, module, module-ui, adapter, app, tooling
port = "metadata"        # adapters only: the service crate whose port it implements
ffi = true               # FFI adapters only: allows unsafe code (every block needs // SAFETY:)
```
