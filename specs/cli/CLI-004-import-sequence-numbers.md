# CLI-004 – Sequence numbers in `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N18, R9
- Depends on: CLI-001, IMPORT-002, IMPORT-009

## Purpose

`framehive import` (CLI-001) numbers the assets of a template with a sequence token, and
keeps the persisted counters consistent across runs, so a script can rely on numbers
that never repeat.

## Behaviour

A template with a sequence token numbers the assets of the import (IMPORT-002). The
numbers of `{seq_today:N}` and `{seq_stored:N}` are reserved after
the scan and before the first file is copied, in the counter file of the state folder
(ADR 0029): `$XDG_STATE_HOME/framehive/` when `XDG_STATE_HOME` is set and absolute,
otherwise `~/.local/state/framehive/`. `--dry-run` shows the numbers and reserves none.
Files imported before get no number (IMPORT-009, CLI-008).
Session and letter numbers start at 1 in every run and are never stored.

Sequence numbers that do not fit the template, and counters that cannot be read or
stored, stop the import before any file is copied, with exit code 2 (CLI-001,
"Exit codes").

## Acceptance criteria

### CLI-004.1 – Persisted sequence counter
- **Given** a counter file with `stored_last=250` in the state folder, and a source with
  `a.MOV` and `b.MOV`
- **When** the user runs `framehive import <source> <destination> --template '{seq_stored:4}.{ext}'`
- **Then** the files are copied as `0251.MOV` and `0252.MOV`, the counter file holds
  `stored_last=252`, standard error has no warning, and the exit code is 0

### CLI-004.2 – A dry run reserves no sequence number
- **Given** the counter file and source of CLI-004.1
- **When** the user runs the same command with `--dry-run`
- **Then** it prints `would copy  a.MOV -> 0251.MOV` and `would copy  b.MOV -> 0252.MOV`,
  and the counter file is unchanged

### CLI-004.3 – Sequence numbers that cannot be reserved
- **Given** the source of CLI-004.1 and a counter file with `stored_last=99`, or one
  with the line `stored_last=many`
- **When** the user runs `framehive import <source> <destination> --template '{seq_stored:2}.{ext}'`
- **Then** no file is copied, the counter file is unchanged, the exit code is 2, and
  the error suggests `{seq_stored:3}.{ext}`, or names the counter file and line 2

## Out of scope

- Numbering in the guided import: CLI-003.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Split from CLI-001, which reached the size limit; implemented in `framehive-cli` |
| 2026-11-04 | The import history recognizes files imported before (CLI-008): the warning about a persisted counter is removed; CLI-004.1 now checks that no warning is printed |
