<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-sequence-counters

Layer: `adapter`, port `naming` (architecture §3)

## Purpose

Implements the `SequenceCounters` port of `naming` with a plain-text file in
Framehive's state folder, with the standard library only (IMPORT-002, ADR 0029). It
adds no dependency, links no native code and has no `unsafe` code.

- **File:** `sequence-counters.txt` in the folder it is given, in the text format of
  ADR 0025: `format_version=1`, `imported_today_date=YYYY-MM-DD`,
  `imported_today_last` and `stored_last`. A missing file or key is 0. Values are at
  most six digits.
- **Broken files:** an unknown key, a key given twice, a value that is not a number
  or a date, or another format version is `SequenceCountersError::Broken` with the
  line. The file is never overwritten then.
- **Locking:** `update` holds an exclusive lock on `sequence-counters.lock`
  (`std::fs::File::lock`) while it reads, changes and stores, so imports in two
  processes reserve one after the other.
- **Writing:** the new content goes to `sequence-counters.txt.part`, is synced,
  renamed over the file, and the folder is synced, so a crash leaves the old or the
  new values, never a mix.
- **Not covered here:** where the state folder is (the app, ADR 0025 §1) and the
  rules for ranges and days (`naming`).

## Public API

| Item | Meaning |
|---|---|
| `FileSequenceCounters` | `naming::SequenceCounters` in a state folder; `new(folder)`, `file()` |
| `COUNTER_FILE_NAME` | The name of the counter file, `sequence-counters.txt` |

## Conformance

`tests/conformance.rs` runs the conformance suite of the port from
`naming::testing`.

## Dependencies

- `naming`
