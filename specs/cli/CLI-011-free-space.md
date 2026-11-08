# CLI-011 – Free space in `framehive import` and the guided import

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: I1, N18
- Depends on: CLI-001, CLI-003, CLI-005, CLI-006, IMPORT-012

## Purpose

`framehive import` and the guided import tell the person before the first copy that a
destination does not have enough space (IMPORT-012), how much is missing, and do not
start.

## Behaviour

**Sizes** are written as in the plan summary of CLI-003: decimal, `<n.n> GB` from one
gigabyte (10⁹ bytes) on, otherwise `<n.n> MB`.

**`framehive import`.** When a file system is short, standard error gets one line per
short file system, and the command stops before any file or folder is created, with
exit code 2. `<folders>` are the destinations on that file system as given on the
command line, separated by `, `.

```
not enough space at <folders>: needs 12.8 GB, 3.1 GB free
```

With `--dry-run`, the plan lines are printed first, then the same lines on standard
error, and the exit code is 2.

The import checks again when its job starts (IMPORT-012), because a job that waits for
another one in the job queue (IMPORT-015) can find the disk filled by it. A shortage
found then is reported with the same line and exit code, and the missing destination
folders are not created either: the import creates them only after that check.

When the free space of a destination cannot be read, standard error gets
`cannot tell the free space at <folder>: <reason>`, and the command goes on. The job
checks again when it starts; when that check cannot read the free space of a
destination the first check did not warn about, the same line follows the job's
summary on standard error. A destination is named at most once.

**The guided import.** When a file system is short, the plan summary says, for each
short file system,
`Not enough space at <folders>: needs 12.8 GB, 3.1 GB free; type d or b to change a folder.`,
and Enter does not start. After `d` or `b`, the summary checks again. A warning about
unknown free space is shown in the summary as on standard error, and Enter starts. A
destination whose free space only the job's own check cannot read is named on standard
error once the import ends, as in `framehive import`.

## Acceptance criteria

### CLI-011.1 – `framehive import` stops when a destination is short
- **Given** a source with `DSCF0001.RAF` of 2.5 MB, and a destination with 1.0 MB free
- **When** the user runs `framehive import <source> --destination <destination>`
- **Then** standard error has `not enough space at <destination>: needs 2.5 MB, 1.0 MB free`,
  the destination folder is not created, and the exit code is 2

### CLI-011.2 – Destinations on one file system are named together
- **Given** the source of CLI-011.1, and a destination and a backup folder on one file
  system with 4.0 MB free
- **When** the user runs `framehive import <source> --destination <destination> --backup <backup>`
- **Then** standard error has
  `not enough space at <destination>, <backup>: needs 5.0 MB, 4.0 MB free`, nothing is
  created, and the exit code is 2

### CLI-011.3 – A dry run reports the shortage
- **Given** the source and destination of CLI-011.1
- **When** the user runs the command of CLI-011.1 with `--dry-run`
- **Then** standard output has `would copy  DSCF0001.RAF -> <target path>`, standard
  error has the line of CLI-011.1, and the exit code is 2

### CLI-011.4 – Unknown free space warns
- **Given** the source of CLI-011.1, and a destination whose free space cannot be read
- **When** the user runs the command of CLI-011.1
- **Then** standard error has `cannot tell the free space at <destination>: <reason>`
  once, the file is copied, and the exit code is 0

### CLI-011.5 – The guided import does not start when a destination is short
- **Given** a card with `DSCF0001.RAF` of 2.5 MB, and a destination with 1.0 MB free
- **When** the plan summary is shown and the person presses Enter
- **Then** the summary says
  `Not enough space at <destination>: needs 2.5 MB, 1.0 MB free; type d or b to change a folder.`,
  and nothing is copied

### CLI-011.6 – Changing the folder checks again
- **Given** the summary of CLI-011.5, and another destination with 10.0 MB free
- **When** the person types `d`, enters the other destination, and presses Enter
- **Then** the summary has no shortage line, and the file is copied

### CLI-011.7 – Unknown free space in the guided import warns
- **Given** a card with `DSCF0001.RAF`, and a destination whose free space cannot be read
- **When** the plan summary is shown and the person presses Enter
- **Then** standard error has `cannot tell the free space at <destination>: <reason>`
  once, and the file is copied

### CLI-011.8 – A shortage found when the job starts creates nothing
- **Given** the source of CLI-011.1, and a destination that does not exist yet whose
  file system has enough space at the first check and 1.0 MB free when the job starts
- **When** the user runs the command of CLI-011.1
- **Then** standard error has `not enough space at <destination>: needs 2.5 MB, 1.0 MB free`,
  the destination folder is not created, and the exit code is 2

### CLI-011.9 – Unknown free space found when the job starts warns after the job
- **Given** the source of CLI-011.1, and a destination whose free space can be read at
  the first check but not when the job starts
- **When** the user runs the command of CLI-011.1
- **Then** the file is copied, standard error has
  `cannot tell the free space at <destination>: <reason>` once, after the summary on
  standard output, and the exit code is 0

### CLI-011.10 – The guided import warns after the import, too
- **Given** a card with `DSCF0001.RAF`, and a destination whose free space can be read
  at the plan summary but not when the job starts
- **When** the plan summary is shown and the person presses Enter
- **Then** the file is copied, and standard error has
  `cannot tell the free space at <destination>: <reason>` once

## Out of scope

- Which bytes count and how file systems are told apart: IMPORT-012.
- A destination that fills up during the copy: CLI-001, IMPORT-004.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-05 | Initial draft for I1 (IMPORT-012, ADR 0033); accepted by the owner |
| 2026-11-05 | Add CLI-011.7: the guided import warns about unknown free space, as the behaviour says |
| 2026-11-05 | Implemented in `framehive-cli`; `implemented` |
| 2026-11-07 | Examples name the destination with `--destination` (CLI-001, ADR 0036) |
| 2026-11-07 | Add CLI-011.8: a shortage found when the job starts creates no folder either |
| 2026-11-08 | Add CLI-011.9 and CLI-011.10: unknown free space that only the job's check finds is named after the job; CLI-011.4 and CLI-011.7 name a destination once (owner decision, issue #70) |
