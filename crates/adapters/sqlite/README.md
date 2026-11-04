<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-sqlite

Layer: `adapter`, port `history` (architecture §3)

## Purpose

Implements the `ImportHistory` port of `history` with a SQLite database,
`import-history.sqlite` in the state folder, through `rusqlite` with the bundled SQLite
(ADR 0031). It knows the layout of the database and nothing about the import.
`framehive-cli` uses it for `framehive import` and the guided import (CLI-008).

- **Created on first write.** A missing database reads as empty; the first write creates
  the state folder and the database, so a dry run creates nothing.
- **Never overwritten.** A file that is not a SQLite database, a `user_version` other
  than 1, or a database without a version that holds tables is an error, and the file is
  not changed.
- **Crash-safe and shared.** WAL journal, `synchronous = FULL`, and a busy timeout of
  5 s for a second process that writes at the same time. Each copy is one statement and
  so one transaction.
- **Exact paths.** Folder and source paths are stored as their bytes (BLOB), so a path
  that is not valid UTF-8 is found again.

## Public API

| Item | Meaning |
|---|---|
| `SqliteImportHistory` | `ImportHistory` in the database of a state folder; `new`, `file` |
| `HISTORY_FILE_NAME` | `import-history.sqlite` |

## Conformance

`tests/conformance.rs` runs the conformance suite of the port from
`history::testing`; `tests/database_file.rs` checks the file and its layout.

## Dependencies

- `domain`
- `history`
- `rusqlite` with `bundled` (ADR 0031)
