# CLI-025 – Injected fault scenarios

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: T1, T3, T6
- Depends on: CLI-001, CLI-007, CLI-008, CLI-022

## Purpose

CLI-022 runs the fault scenarios of requirements section 9.2 that files on disk can
set up. This specification states what `framehive import` does in the scenarios that
need the fault-injection point of ADR 0048, so each one runs against the binary with
real adapters.

## Behaviour

The criteria add no new behaviour to the import: they combine the rules of IMPORT-004
(verified copy), IMPORT-009 (import history) and CLI-007 (format readiness).

| Scenario | Rule | Criterion |
|---|---|---|
| T1 The process is killed in the middle of a copy | The target name only ever has verified content; the partial copy is in `<target>.part` and replaced by the next run (IMPORT-004.8, .10); copied files are in the history (IMPORT-009) | CLI-025.1 |
| T3 A bit error at the destination after writing | The read-back from the storage device fails verification and the target is removed (IMPORT-004.6, .15) | CLI-025.2 |
| T6 The device is unplugged during the import | A source that cannot be read fails its file and leaves nothing behind (IMPORT-004.4, .11) | CLI-025.3 |

The faults exist only in a build with the cargo feature `fault-injection`, which reads
`FRAMEHIVE_FAULT` once at start (ADR 0048 §3):

| Value | Fault |
|---|---|
| `abort-while-writing <name>` | Writes half of the first write to `<name>.part`, then aborts the process |
| `flip-bit-after-sync <name>` | Flips the lowest bit of the first byte read back from the storage device for the target `<name>` |
| `unplug-while-reading <name>` | Fails the first read of the source `<name>` during its copy, and from then on every call on a path in its folder, with `Input/output error (os error 5)` |

Each fault happens once per run. A value that names no known fault, or no file, stops
the command with `error: FRAMEHIVE_FAULT: <reason>` and exit code 2 before anything is
read, so a typo never runs the import without the fault.

## Acceptance criteria

### CLI-025.1 – T1: a killed import leaves only a `.part` file and resumes
- **Given** a source with `DSCF0001.RAF`, `DSCF0002.RAF` and `DSCF0003.RAF`, each
  captured on 2026-10-17 with its own content
- **When** `framehive import <source> --destination <destination>` runs with
  `FRAMEHIVE_FAULT=abort-while-writing DSCF0002.RAF`, and then runs again without it
- **Then** the first run ends with `SIGABRT`, and what it printed is not promised,
  because a kill can come before the report is written; `DSCF0001.RAF` is at its
  target with its content,
  `2026/2026-10-17/DSCF0002.RAF.part` holds the first half of the file, and neither
  `DSCF0002.RAF` nor `DSCF0003.RAF` is at the destination; the second run prints
  `imported before  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`, copies the other two,
  prints the summary
  `2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, leaves
  no `.part` file, and exits with 0

### CLI-025.2 – T3: a bit error at the destination fails the file
- **Given** the source of CLI-025.1
- **When** the command runs with `FRAMEHIVE_FAULT=flip-bit-after-sync DSCF0002.RAF`, and
  then runs again without it
- **Then** the first run copies `DSCF0001.RAF` and `DSCF0003.RAF`, prints
  `failed  DSCF0002.RAF: cannot verify the target: the target has the checksum blake3:<hex>, the source blake3:<hex>`,
  the summary `2 copied, 0 already imported, 0 duplicates, 0 skipped, 1 failed, 0 left out`,
  `Do not format the source: not every file is verified in 1 location.` and
  `  DSCF0002.RAF: verified in 0 of 1 location`, leaves neither `DSCF0002.RAF` nor its
  `.part` file at the destination, and exits with 1; the second run copies
  `DSCF0002.RAF` with its content, prints the summary
  `1 copied, 2 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, and
  exits with 0

### CLI-025.3 – T6: an unplugged source fails the remaining files
- **Given** the source of CLI-025.1
- **When** the command runs with `FRAMEHIVE_FAULT=unplug-while-reading DSCF0002.RAF`,
  and then runs again without it
- **Then** the first run copies `DSCF0001.RAF`, prints
  `failed  <name>: cannot read the source: <path>: Input/output error (os error 5)` for
  `DSCF0002.RAF` and `DSCF0003.RAF`, the summary
  `1 copied, 0 already imported, 0 duplicates, 0 skipped, 2 failed, 0 left out`,
  `Do not format the source: not every file is verified in 1 location.` and a line
  `  <name>: verified in 0 of 1 location` for each of the two, leaves neither of them
  nor a `.part` file at the destination, and exits with 1; the second run prints
  `imported before  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`, copies the other two,
  prints `The source can be formatted: every file is verified in 1 location.`, and
  exits with 0

### CLI-025.4 – A fault that cannot be read is a usage error
- **Given** the source of CLI-025.1
- **When** the command runs with `FRAMEHIVE_FAULT=flip-bits DSCF0002.RAF`, or with a
  known fault and no file name
- **Then** it prints
  ``error: FRAMEHIVE_FAULT: unknown fault `flip-bits`; the faults are abort-while-writing, flip-bit-after-sync, unplug-while-reading``,
  or ``error: FRAMEHIVE_FAULT: `<fault>` needs a file name``, creates no destination, and
  exits with 2

## Out of scope

- T5 (a slow or hanging device): it needs a per-file time limit that no spec defines
  yet (ADR 0048 §4), before `hang-while-reading` is built.
- An unplugged camera: IMPORT-014.9 covers it in the services; the binary runs leave it
  to the virtual camera of libgphoto2 (requirements section 9.3).

## History

| Date | Change |
|---|---|
| 2026-11-19 | Created (ADR 0048) |
