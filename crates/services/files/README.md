<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# files

Layer: `service` (architecture §2, §3)

## Purpose

The files service holds the ports through which Framehive reaches storage devices
and computes checksums (ADR 0020 §4).

## Public API

| Item | Meaning |
|---|---|
| `FileSystem`, `WriteFile`, `EntryKind` | Port for files and folders, with explicit syncing |
| `Hasher`, `HashState` | Port for a hash function that yields a `domain::Checksum` |
| `testing` | `MemoryFileSystem`, `FnvHasher`, `TempDir` and the conformance suites of both ports |

## Dependencies

- `domain`
