<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# history

Layer: `service` (architecture §2, §3)

## Purpose

The history service holds the types and the port of the import history (IMPORT-009,
ADR 0031): every copy verified at a destination, and the start and end of each run. The
import module looks a file up by destination, original file name, size and capture time
before it copies it again, and confirms the match by checksum.

Boundaries:

- **A port, no storage.** `ImportHistory` is implemented by `adapter-sqlite`; only apps
  choose it, as `framehive-cli` does for both imports (CLI-008). The service reads no files and no clock: the caller passes every time as a
  `RecordedTime`.
- **The key.** `CopyKey` matches a recorded copy with the same destination, original
  file name and size. The capture time is compared only when the key has one; it is the
  capture time as the file records it, before camera time correction.
- **Oldest first.** `find_copies` returns the matches in the order they were recorded,
  so the last one is the most recent copy.
- **Not here:** deciding that a file is imported before or backed up before, which
  needs the checksum, belongs to the import module.

## Public API

| Item | Meaning |
|---|---|
| `ImportHistory`, `HistoryError` | Port to the import history, and why it cannot be used |
| `RunStart`, `RunId` | The start of a run, and the id the history gives it |
| `CopyRecord` | One verified copy at one destination |
| `CopyKey`, `FoundCopy` | What a file is looked up by, and the target and checksum of a match |
| `RecordedTime` | A machine-local time with its offset, written as RFC 3339 |
| `testing::MemoryImportHistory`, `testing::RecordedRun` | Test double that can be broken or fail to record |
| `testing::check_import_history`, `testing::ImportHistoryUnderTest` | Conformance suite of the port |

## Dependencies

- `domain`
- `thiserror` (ADR 0016)
