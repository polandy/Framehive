# CLI-011 – Free space in `framehive import` and the guided import

- Status: accepted
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

When the free space of a destination cannot be read, standard error gets
`cannot tell the free space at <folder>: <reason>`, and the command goes on.

**The guided import.** When a file system is short, the plan summary says, for each
short file system,
`Not enough space at <folders>: needs 12.8 GB, 3.1 GB free; type d or b to change a folder.`,
and Enter does not start. After `d` or `b`, the summary checks again. A warning about
unknown free space is shown in the summary as on standard error, and Enter starts.

## Acceptance criteria

### CLI-011.1 – `framehive import` stops when a destination is short
- **Given** a source with `DSCF0001.RAF` of 2.5 MB, and a destination with 1.0 MB free
- **When** the user runs `framehive import <source> <destination>`
- **Then** standard error has `not enough space at <destination>: needs 2.5 MB, 1.0 MB free`,
  the destination folder is not created, and the exit code is 2

### CLI-011.2 – Destinations on one file system are named together
- **Given** the source of CLI-011.1, and a destination and a backup folder on one file
  system with 4.0 MB free
- **When** the user runs `framehive import <source> <destination> --backup <backup>`
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
- **Then** standard error has `cannot tell the free space at <destination>: <reason>`,
  the file is copied, and the exit code is 0

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
- **Then** standard error has `cannot tell the free space at <destination>: <reason>`,
  and the file is copied

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
