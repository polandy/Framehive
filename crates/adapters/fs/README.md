<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-fs

Layer: `adapter`, port `files` (architecture §3)

## Purpose

Implements the `FileSystem` port of `files` over `std::fs`, for the verified copy and
the source scan of the import (ADR 0020 §3, §4). It is an FFI crate for one call
(ADR 0047).

- `entry` and `list_folder` do not follow symbolic links (`symlink_metadata`,
  `DirEntry::file_type`); a link is `EntryKind::SymbolicLink`, so a scan cannot loop
  or leave its source (IMPORT-005). `file_size` and `modification_time` follow links,
  like `open_read`.
- `create_file` opens with `create_new`, which never follows a symbolic link; an old
  file is removed first, and a link or folder is refused (IMPORT-004.14).
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

- `open_read_from_medium` opens like `open_read`, then on Linux drops the file's
  cached pages with `posix_fadvise(POSIX_FADV_DONTNEED)`, so the read-back of a
  verified copy comes from the device; on macOS it sets `F_NOCACHE`, which is best
  effort because pages cached by the write may still be served (ADR 0047). The
  `unsafe` calls live in `src/medium.rs`, one per platform.

- `replace_file` refuses a file that is read-only by its mode bits, for the superuser
  too, and one the user cannot open for writing; it then copies the old file's
  permissions to `from` and renames with `std::fs::rename` (CORE-009).

## Public API

| Item | Meaning |
|---|---|
| `StdFileSystem` | `files::FileSystem` over `std::fs`; `StdFileSystem::new()` or `Default` |

## Conformance

`tests/conformance.rs` runs `files::testing::check_file_system` on the real file
system in a `files::testing::TempDir`, with real symbolic links on Unix. The unit tests run the same suite with a hard
link that fails as on exFAT, to cover the fallback, and check that it never replaces
an existing file. A Unix test checks that `replace_file` keeps permissions and refuses
a read-only file.

## Dependencies

- `domain`, `files`
- `libc` (ADR 0047): `posix_fadvise` and `fcntl` for `open_read_from_medium`
- `tracing` (ADR 0046): a detail when a rename falls back to the path without hard
  links
