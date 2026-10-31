<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# files

Layer: `service` (architecture §2, §3)

## Purpose

The files service holds the ports through which Framehive reaches storage devices
and computes checksums, and the verified copy that writes every imported file
(IMPORT-004, ADR 0020 §3, §4).

Boundaries:

- **Ports only.** The service never calls `std::fs`; `adapter-fs` implements
  `FileSystem` and `adapter-blake3` implements `Hasher`. Only `testing::TempDir`
  touches the real file system, for adapter tests.
- **Verified copy.** `copy_verified` reads the source once, hashes it while writing
  `<target>.part`, syncs it with the source's modification time, renames it without
  replacing a file, syncs the folder, and reads the target back to compare checksums.
  Missing folders are created and synced into their parents. It never writes to the
  source.
- **Cleanup.** On a failure the `.part` file is removed. After the rename, a failed
  folder sync, read-back or verification also removes the target, because the copy
  wrote it and it is not verified. A file another program put at the target is never
  touched. A file that cannot be removed is named in `CopyError::cleanup`.
- **Durability model.** `MemoryFileSystem` makes file content durable on
  `WriteFile::sync` and folder entries on `FileSystem::sync_folder`; `crash` drops
  the rest. The `FileSystem` conformance suite holds the contract every adapter must
  meet, including "rename never replaces" and a synced rename surviving a crash.
- **Not covered here:** which files to copy and name conflicts (IMPORT-001, IMPORT-003),
  and continuing after a failed file (IMPORT-004.11), which is the import run's job.

## Public API

| Item | Meaning |
|---|---|
| `FileSystem`, `ReadSeek`, `WriteFile`, `EntryKind`, `FolderEntry` | Port for files and folders, with explicit syncing; files are read from any offset (ADR 0021); folders are listed without following symbolic links (IMPORT-005) |
| `Hasher`, `HashState` | Port for a hash function that yields a `domain::Checksum` with its algorithm id |
| `checksum_file` | Reads a file in full and returns its checksum; used to compare content (IMPORT-003) and to read a copy back |
| `copy_verified` | Copies one file through a `.part` file and verifies it (IMPORT-004) |
| `CopyError`, `CopyStep`, `CopyErrorKind`, `CleanupFailure` | The failed step, why, and a file that could not be removed |
| `testing::MemoryFileSystem`, `testing::Fault` | `FileSystem` in memory with simulated crashes, symbolic links, read, write, read-back and listing faults, and stops after a number of operations |
| `testing::FnvHasher` | Non-cryptographic `Hasher` for tests; its checksums are written `fnv:<hex>` |
| `testing::TempDir` | A temporary folder on the real file system, removed when dropped |
| `testing::check_file_system`, `testing::FileSystemUnderTest`, `testing::MakeSymbolicLink` | Conformance suite of the `FileSystem` port |
| `testing::check_hasher`, `testing::HasherUnderTest` | Conformance suite of the `Hasher` port |

## Dependencies

- `domain`
- `thiserror` (ADR 0016)
