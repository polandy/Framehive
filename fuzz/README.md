<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# metadata-fuzz

Layer: `tooling` (ADR 0037)

## Purpose

Coverage-guided fuzz targets for our own parsers of untrusted bytes, run with
`cargo-fuzz` and LLVM's libFuzzer (ADR 0037). The crate is a workspace of its own with
its own `Cargo.lock`, so the main workspace, `cargo xtask ci` and the release build
never build libFuzzer. `cargo xtask check-dependencies`, `check-files` and `cargo deny`
still check it.

| Target | Input goes to |
|---|---|
| `capture_time` | `metadata::capture_time_from_bytes`, the EXIF reader (CORE-005, ADR 0021) |

## Running

Only in the fuzz shell, which has a nightly toolchain pinned to a date and
`cargo-fuzz`:

```sh
nix develop .#fuzz -c cargo xtask fuzz --seconds 60
```

`cargo xtask fuzz` writes the seeds into the ignored `fuzz/corpus/<target>/` and runs
every target; see `xtask/README.md`. The workflow `.github/workflows/fuzz.yml` runs it
every night for ten minutes per target and keeps the corpus in the Actions cache.

## Findings

libFuzzer keeps a crashing input in `fuzz/artifacts/<target>/`; the nightly workflow
uploads it with the log. Replay it with `cargo fuzz run <target> <file>`. Each finding
is fixed by a deterministic regression test in `metadata` that builds the bytes in
code, never as a file, and carries the `spec:` marker of the criterion it broke
(CORE-005.9 or .12). The mutation test in
`crates/services/metadata/tests/mutated_files.rs` stays the deterministic gate in every
pull request.

## Dependencies

- `metadata`
- `libfuzzer-sys` (ADR 0037), which builds libFuzzer's C++ sources and pulls in
  `arbitrary`; the targets take raw bytes and do not use it
