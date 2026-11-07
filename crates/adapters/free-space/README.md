<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-free-space

Layer: `adapter`, port `files` (architecture §3), FFI adapter

## Purpose

Implements the `FreeSpace` port of `files` with `statvfs(3)` from the C library, on
Linux and macOS (ADR 0033). The import uses it to check, before the first copy, that
every destination can hold the files it is about to write (IMPORT-012).

- **Free space** is `f_bavail × f_frsize`: the bytes an unprivileged process may still
  write. It already leaves out the blocks reserved for root; no reserve is added.
- **File system identity** is `st_dev` of the folder, so destinations on one file
  system are added up.
- **Missing folders** are answered for their nearest existing parent folder. A path
  that cannot be inspected for another reason, such as a file on the way, is an error.
- **`unsafe`:** the crate holds one `unsafe` block, the call of `libc::statvfs`, with a
  `// SAFETY:` comment. It is the only crate that depends on `libc`.
- **Not covered here:** which bytes count and what a shortage means (IMPORT-012, in
  `import`).

## Public API

| Item | Meaning |
|---|---|
| `StatvfsFreeSpace` | `files::FreeSpace` over `statvfs(3)` and `st_dev` |

## Conformance

`tests/conformance.rs` runs `files::testing::check_free_space` in a `TempDir`. Unit
tests in `src/lib.rs` cover the `f_bavail × f_frsize` conversion and its overflow,
which no real file system reaches.

## Dependencies

- `files`
- `libc` (ADR 0033)
