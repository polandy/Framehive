<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-fs

Layer: `adapter`, port `files` (architecture §3)

## Purpose

Implements the `FileSystem` port of `files` over `std::fs`, for the verified copy and
the source scan of the import (ADR 0020 §3, §4). It adds no dependency.

- `entry` and `list_folder` do not follow symbolic links (`symlink_metadata`,
  `DirEntry::file_type`); a link is `EntryKind::SymbolicLink`, so a scan cannot loop
  or leave its source (IMPORT-005). `file_size` and `modification_time` follow links,
  like `open_read`.
- `create_file` sets the modification time with `File::set_modified` and syncs with
  `File::sync_all`; `sync_folder` opens the folder and syncs it.
- `rename_no_replace` never replaces: it creates `to` as a hard link to the file,
  which fails with `AlreadyExists` if anything is there, then removes `from`. If
  removing `from` fails, it removes `to` again.
- **Without hard links** (exFAT answers `EPERM`, others `EOPNOTSUPP`): only when the
  link fails with `PermissionDenied` or `Unsupported` and `from` is a file, it checks
  that nothing exists at `to` and renames with `std::fs::rename`. Another program that
  creates `to` between the check and the rename gets its file replaced; ADR 0020 §3
  accepts this race, because these file systems offer no rename that refuses to
  replace.

## Public API

| Item | Meaning |
|---|---|
| `StdFileSystem` | `files::FileSystem` over `std::fs`; `StdFileSystem::new()` or `Default` |

## Conformance

`tests/conformance.rs` runs `files::testing::check_file_system` on the real file
system in a `files::testing::TempDir`, with real symbolic links on Unix. The unit tests run the same suite with a hard
link that fails as on exFAT, to cover the fallback, and check that it never replaces
an existing file.

## Dependencies

- `domain`, `files`
