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
  `FileSystem`, `adapter-blake3` implements `Hasher` and `adapter-free-space`
  implements `FreeSpace`. Only `testing::TempDir`
  touches the real file system, for adapter tests.
- **Verified copy.** `copy_verified` reads the source once, hashes it while writing
  `<target>.part`, syncs it with the source's modification time, renames it without
  replacing a file, syncs the folder, and reads the target back to compare checksums.
  Missing folders are created and synced into their parents. It never writes to the
  source.
- **Several targets.** `copy_verified_to_all` is the same copy to several targets with
  one read of the source (IMPORT-007, ADR 0030 §1): the bytes go to every `.part` file,
  and each target is synced, renamed, synced and verified on its own. A target that
  fails is dropped and cleaned up without touching the others; the read ends early only
  when every target failed. `copy_verified` is this copy with one target.
  The source may lie on its own `ReadOnlyFileSystem`, such as a camera (ADR 0034).
- **Cleanup.** On a failure the `.part` file is removed. After the rename, a failed
  folder sync, read-back or verification also removes the target, because the copy
  wrote it and it is not verified. A file another program put at the target is never
  touched. A file that cannot be removed is named in `CopyError::cleanup`.
- **Durability model.** `MemoryFileSystem` makes file content durable on
  `WriteFile::sync` and folder entries on `FileSystem::sync_folder`; `crash` drops
  the rest. The `FileSystem` conformance suite holds the contract every adapter must
  meet, including "rename never replaces" and a synced rename surviving a crash.
- **Cameras.** The `Cameras` port lists the cameras and phones connected over PTP or
  MTP and opens one as a `ReadOnlyFileSystem` with one folder per storage; dropping it
  releases the camera, and a camera another program holds is a `CameraError::InUse`
  (ADR 0034 §2, §6).
- **Trash.** The `Trash` port moves one file to the desktop's trash and returns where it
  lies there; a name already taken gets its own, and on an error the file stays.
  Folders and symbolic links are refused (ADR 0043). `adapter-freedesktop-trash` and
  `adapter-macos-trash` implement it.
- **Not covered here:** which files to copy and name conflicts (IMPORT-001, IMPORT-003),
  and continuing after a failed file (IMPORT-004.11), which is the import run's job.

## Public API

| Item | Meaning |
|---|---|
| `ReadOnlyFileSystem`, `ReadSeek`, `EntryKind`, `FolderEntry` | Port for reading files and folders (`entry`, `list_folder`, `file_size`, `modification_time`, `open_read`); files are read from any offset (ADR 0021); folders are listed without following symbolic links (IMPORT-005); the import reads its source only through it (ADR 0034) |
| `FileSystem`, `WriteFile` | Port for files and folders: `ReadOnlyFileSystem` with creating, renaming, replacing a file in one step (keeping its permissions, refusing a read-only file), removing and explicit syncing |
| `FreeSpace`, `SpaceOnFileSystem`, `FileSystemId` | Port for the free space and the identity of the file system a folder lies on; a missing folder is answered for its nearest existing parent (ADR 0033) |
| `Trash`, `not_a_file` | Port for moving a file to the trash, and the `InvalidInput` error every implementation returns for a path that is not a file (ADR 0043) |
| `Hasher`, `HashState` | Port for a hash function that yields a `domain::Checksum` with its algorithm id |
| `checksum_file` | Reads a file in full and returns its checksum; used to compare content (IMPORT-003) and to read a copy back |
| `copy_verified` | Copies one file through a `.part` file and verifies it (IMPORT-004) |
| `copy_verified_to_all`, `VerifiedCopies` | Copies one file to several targets with one read and verifies each, with one result per target (IMPORT-007); the source may lie on its own `ReadOnlyFileSystem` |
| `CopyError`, `CopyStep`, `CopyErrorKind`, `CleanupFailure` | The failed step, why, and a file that could not be removed |
| `Cameras`, `Camera`, `CameraError` | Port for cameras and phones over PTP or MTP: lists them sorted by port, opens one as a `ReadOnlyFileSystem`, and tells a camera in use by another program from other failures (ADR 0034) |
| `testing::MemoryFileSystem`, `testing::Fault` | `FileSystem` in memory with simulated crashes, symbolic links, read-only files, read, write, read-back, rename and listing faults, and stops after a number of operations |
| `testing::MemoryCameras` | `Cameras` that serve `MemoryFileSystem`s as cameras, with cameras in use, cameras disconnected again, a failing list, and whether a camera is still open |
| `testing::FnvHasher` | Non-cryptographic `Hasher` for tests; its checksums are written `fnv:<hex>` |
| `testing::TempDir` | A temporary folder on the real file system, removed when dropped |
| `testing::check_read_only_file_system`, `testing::ReadOnlyFileSystemUnderTest`, `testing::AddFolder`, `testing::AddFile`, `testing::MakeSymbolicLink` | Conformance suite of the `ReadOnlyFileSystem` port; it lays out its files through the storage of the implementation |
| `testing::check_file_system`, `testing::FileSystemUnderTest` | Conformance suite of the `FileSystem` port; it includes the `ReadOnlyFileSystem` suite |
| `testing::check_cameras`, `testing::CamerasUnderTest` | Conformance suite of the `Cameras` port; it runs the `ReadOnlyFileSystem` suite on an opened camera |
| `testing::MemoryFreeSpace` | `FreeSpace` with fixed answers per mounted folder, and file systems that do not report their free space |
| `testing::check_free_space`, `testing::FreeSpaceUnderTest` | Conformance suite of the `FreeSpace` port |
| `testing::check_hasher`, `testing::HasherUnderTest` | Conformance suite of the `Hasher` port |
| `testing::MemoryTrash` | `Trash` that moves files to a folder of a `MemoryFileSystem`, records what it moved, and refuses chosen files |
| `testing::check_trash`, `testing::TrashUnderTest`, `testing::WriteTrashFile`, `testing::ReadTrashFile`, `testing::RemoveTrashFile` | Conformance suite of the `Trash` port; it lays out and reads its files through the storage of the implementation and removes what it trashed |

## Dependencies

- `domain`
- `thiserror` (ADR 0016)
