<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# xtask

## Purpose

Repository automation. It enforces the rules in `CLAUDE.md` mechanically, so quality
does not depend on human review. Besides `kernel`, for the module id rules, it uses
only the standard library: it reads `cargo metadata` output with its own small JSON
reader and `dependencies.yaml` with a strict reader for the subset of YAML that file
uses. Both are replaced by the YAML
adapter once ADR 0011 is implemented.

## Commands

Run them as `cargo xtask <command>`, inside `nix develop`.

| Command | Checks |
|---|---|
| `ci` | Everything, in CI order: format, clippy, the five checks below, `testdata check`, the spec report, nextest, doc tests, `cargo deny`, `reuse lint`. Quiet: one `ok` line per step plus warnings from our own checks, and the full output of the failing step only |
| `check-files` | Every file Git tracks or would track is UTF-8 text without NUL bytes and at most 256 KiB (ADR 0017); Rust files have at most 400 lines; Markdown outside `docs/adr/` and `CHANGELOG.md` has at most 10 KiB (architecture §7) |
| `check-architecture` | Every crate declares a valid layer; dependencies follow architecture §3; adapters use only their port service; module UIs use only their module and `ui-kit`; crate roots contain `#![forbid(unsafe_code)]` unless `ffi = true`; every README has the required sections |
| `check-dependencies` | Every external dependency is listed in `dependencies.yaml`, used only by allowed layers and crates, with allowed features (`default` counts as a feature), and pinned in `[workspace.dependencies]`; no unused entries |
| `check-specs` | Spec headers, IDs, status and fields; origins exist in `docs/requirements/`; dependencies exist (a warning for drafts); unique criteria; no open questions in binding specs; every `/// spec: <ID>.<n>` points to an existing, non-draft criterion; every criterion of an implemented spec has a test |
| `check-spec-trailers` | The diff check of ADR 0010 §4 on the commits since the base (in a pull request `origin/<base>..HEAD`, on `main` only `HEAD`, locally `main..HEAD`; merge commits skipped): a commit that changes a `.rs` file with markers for a spec changes that spec or has `Spec-Unchanged: <reason>`; a commit that changes an `implemented` spec changes a `.rs` file in a crate with markers for it, or has `Spec-Only: <reason>`. When `PULL_REQUEST_TITLE` is set (always in a pull request run in CI, together with `PULL_REQUEST_BODY`), the title and description are also judged as the future squash commit over all changes since the merge base. Without a local `main` it warns and passes; in CI (`CI=true`) a missing base, a shallow clone or, in a pull request run, a missing `PULL_REQUEST_TITLE` fails |
| `testdata check` | `testdata.lock` (ADR 0027): the header `framehive-testdata-lock 1`, then one entry per line, `<blake3> <size_bytes> <path> <url>`; 64 lowercase hex digits, a decimal size, a relative path without `.`/`..` (a trailing `/` marks a `.tar.zst` archive), an `https://` URL; no duplicate paths and no path inside an archive directory |
| `spec-report` | Not a check: writes `target/spec-report.md`, the overview requirement → spec → criteria → tests with status. `ci` runs it, and CI keeps the file as an artifact |
| `new-module <id>` | Not a check: generates `modules/<id>/<id>` and `modules/<id>/<id>-ui` from `templates/module/` (architecture §4.1, §4.5), adds both to `members` and `[workspace.dependencies]` and refreshes `Cargo.lock` through Cargo. Rejects invalid ids (`kernel::ModuleId`), Rust keywords and names already in the workspace before it writes anything |
| `new-adapter <port> <name>` | Not a check: generates `crates/adapters/<name>` (package `adapter-<name>`, layer `adapter`, `port = "<port>"`) from `templates/adapter/` with a README, a crate root and a placeholder conformance test, adds it to `members` and `[workspace.dependencies]` and refreshes `Cargo.lock` through Cargo. The port `domain` stands for the ports of `domain`, such as `Clock`; the crate then declares no `port`. Rejects invalid names, unknown ports and names already in the workspace before it writes anything |
| `ci-failures [<pr>]` | Not a check: prints only what failed in the CI run of a pull request (default: the pull request of the current branch), from the `FAIL` line of `cargo xtask ci` on, or the end of the log for a job that failed before. Needs `gh`, logged in |

## Crate metadata

Each crate declares its place in `Cargo.toml`:

```toml
[package.metadata.framehive]
layer = "adapter"        # domain, kernel, service, module, module-ui, adapter, app, tooling
port = "metadata"        # adapters only: the service crate whose port it implements
ffi = true               # FFI adapters only: allows unsafe code (every block needs // SAFETY:)
```
